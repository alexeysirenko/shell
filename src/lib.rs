mod output;

use anyhow::{Result, anyhow};
use is_executable::IsExecutable;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command as CmdCommand, Stdio};
use std::{env, process};
use strum_macros::{EnumIter, EnumString};

pub use crate::output::{FileOutput, Output, OutputStreams, StdErrOutput, StdOutput};

pub enum PromptQuote {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
}

#[derive(Debug, EnumString, EnumIter, PartialEq)]
pub enum CommandKind {
    #[strum(serialize = "exit")]
    Exit,
    #[strum(serialize = "echo")]
    Echo,
    #[strum(serialize = "type")]
    Type,
    #[strum(serialize = "pwd")]
    Pwd,
    #[strum(serialize = "cd")]
    Cd,
}

#[derive(Debug)]
pub enum Command {
    Exit,
    Echo(String),
    Type(String),
    Exec { command: String, args: Vec<String> },
    Pwd,
    Cd(String),
}

pub fn parse_prompt(prompt: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut buffer = String::new();
    let mut quote = PromptQuote::Unquoted;

    let push = |buffer: &mut String, tokens: &mut Vec<String>| {
        if !buffer.is_empty() {
            tokens.push(buffer.to_string());
        }
        buffer.clear();
    };

    let mut chars = prompt.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            PromptQuote::Unquoted => match c {
                ' ' | '\t' | '\n' => push(&mut buffer, &mut tokens),
                '\'' => quote = PromptQuote::SingleQuoted,
                '"' => quote = PromptQuote::DoubleQuoted,
                '\\' => {
                    if let Some(next_char) = chars.next() {
                        buffer.push(next_char)
                    }
                }
                _ => buffer.push(c),
            },
            PromptQuote::SingleQuoted => match c {
                '\'' => quote = PromptQuote::Unquoted,
                _ => buffer.push(c),
            },
            PromptQuote::DoubleQuoted => match c {
                '"' => quote = PromptQuote::Unquoted,
                '\\' => {
                    if let Some(&next_ch) = chars.peek() {
                        if matches!(next_ch, '\\' | '"' | '$' | '`' | '\n') {
                            chars.next();
                            if next_ch != '\n' {
                                buffer.push(next_ch);
                            }
                        } else {
                            buffer.push(c);
                        }
                    } else {
                        buffer.push(c);
                    }
                }
                _ => buffer.push(c),
            },
        }
    }
    push(&mut buffer, &mut tokens);

    tokens
}

fn extract_redirects(args: &[String]) -> Result<(Vec<String>, Box<dyn Output>, Box<dyn Output>)> {
    let mut filtered = Vec::new();
    let mut stdout: Box<dyn Output> = Box::new(StdOutput::new());
    let mut stderr: Box<dyn Output> = Box::new(StdErrOutput::new());

    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            ">" | "1>" => {
                let path = iter
                    .next()
                    .ok_or_else(|| anyhow!("redirect path missing"))?;
                let file = FileOutput::new(path, false)?;
                stdout = Box::new(file);
            }
            "2>" => {
                let path = iter
                    .next()
                    .ok_or_else(|| anyhow!("redirect path missing"))?;
                let file = FileOutput::new(path, false)?;
                stderr = Box::new(file);
            }
            ">>" | "1>>" => {
                let path = iter
                    .next()
                    .ok_or_else(|| anyhow!("redirect path missing"))?;
                let file = FileOutput::new(path, true)?;
                stdout = Box::new(file);
            }
            _ => filtered.push(arg.clone()),
        }
    }

    Ok((filtered, stdout, stderr))
}

pub fn parse_command(args: Vec<String>) -> Result<(Command, OutputStreams)> {
    let (name, rest) = args.split_first().ok_or_else(|| anyhow!("Empty command"))?;
    let (args, stdout, stderr) = extract_redirects(rest)?;

    let arg_str = args.join(" ");

    let command = match name.parse::<CommandKind>() {
        Ok(CommandKind::Exit) => Command::Exit,
        Ok(CommandKind::Echo) => Command::Echo(arg_str),
        Ok(CommandKind::Type) => Command::Type(arg_str),
        Ok(CommandKind::Pwd) => Command::Pwd,
        Ok(CommandKind::Cd) => Command::Cd(arg_str),
        Err(_) => Command::Exec {
            command: name.to_string(),
            args,
        },
    };

    Ok((command, OutputStreams::new(stdout, stderr)))
}

pub fn handle_command(command: Command, streams: &mut OutputStreams) {
    let result: Result<()> = match command {
        Command::Exit => exit(),
        Command::Echo(text) => echo(&text, &mut *streams.stdout),
        Command::Type(command) => r#type(&command, &mut *streams.stdout),
        Command::Exec { command, args } => exec(&command, &args, streams),
        Command::Pwd => pwd(&mut *streams.stdout),
        Command::Cd(path) => cd(&path),
    };

    if let Err(e) = result {
        streams.stderr.print(&e.to_string());
    }
}

fn exit() -> Result<()> {
    process::exit(0)
}

fn echo(text: &str, output: &mut dyn Output) -> Result<()> {
    output.print(text);
    Ok(())
}

fn r#type(command: &str, output: &mut dyn Output) -> Result<()> {
    if is_built_in(command) {
        output.print(&format!("{} is a shell builtin", command));
    } else if let Some(path) = find_in_path(command) {
        output.print(&format!("{} is {}", command, path.display()));
    } else {
        output.print(&format!("{}: not found", command));
    }
    Ok(())
}

fn is_built_in(command: &str) -> bool {
    command.parse::<CommandKind>().is_ok()
}

fn find_in_path(executable: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(executable);
            if full_path.is_file() && full_path.is_executable() {
                fs::canonicalize(&full_path).ok()
            } else {
                None
            }
        })
    })
}

fn exec(command: &str, args: &[String], streams: &mut OutputStreams) -> Result<()> {
    find_in_path(command).ok_or_else(|| anyhow!("{}: command not found", command))?;

    let mut child = CmdCommand::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            streams.stdout.print(&line);
        }
    }

    if let Some(stderr) = child.stderr.take() {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            streams.stderr.print(&line);
        }
    }

    child.wait()?;
    Ok(())
}

fn pwd(output: &mut dyn Output) -> Result<()> {
    let dir = env::current_dir()?;
    let absolute = fs::canonicalize(dir)?;
    output.print(&format!("{}", absolute.display()));
    Ok(())
}

fn cd(path: &str) -> Result<()> {
    let target = match path {
        "" | "~" => dirs::home_dir(),
        p if p.starts_with("~/") => dirs::home_dir().map(|home| home.join(&p[2..])),
        p => Some(PathBuf::from(p)),
    };

    match target {
        Some(t) => {
            env::set_current_dir(t).map_err(|_| anyhow!("cd: {}: No such file or directory", path))
        }
        None => Err(anyhow!("cd: {}: No such file or directory", path)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_words() {
        assert_eq!(parse_prompt("echo hello"), vec!["echo", "hello"]);
    }

    #[test]
    fn test_single_quotes() {
        assert_eq!(
            parse_prompt("echo 'hello world'"),
            vec!["echo", "hello world"]
        );
    }

    #[test]
    fn test_multiple_spaces() {
        assert_eq!(parse_prompt("echo   hello"), vec!["echo", "hello"]);
    }

    #[test]
    fn test_redirect_stdout() {
        let args = vec!["echo".into(), "hello".into(), ">".into(), "out.txt".into()];
        let (filtered, _, _) = extract_redirects(&args[1..]).unwrap();
        assert_eq!(filtered, vec!["hello"]);
    }

    #[test]
    fn test_redirect_stderr() {
        let args = vec!["cmd".into(), "2>".into(), "err.txt".into()];
        let (filtered, _, _) = extract_redirects(&args[1..]).unwrap();
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_spaces_in_quotes() {
        assert_eq!(parse_prompt("'hello   world'"), vec!["hello   world"]);
    }

    #[test]
    fn test_empty() {
        assert_eq!(parse_prompt(""), Vec::<String>::new());
    }

    #[test]
    fn test_mixed() {
        assert_eq!(
            parse_prompt("cmd 'arg one' arg2"),
            vec!["cmd", "arg one", "arg2"]
        );
    }

    #[test]
    fn test_mixed2() {
        assert_eq!(
            parse_prompt("echo 'hello     script' 'shell''world' example''test"),
            vec!["echo", "hello     script", "shellworld", "exampletest"]
        );
    }

    #[test]
    fn test_double_quotes1() {
        assert_eq!(
            parse_prompt("echo \"hello    world\""),
            vec!["echo", "hello    world"]
        );
    }

    #[test]
    fn test_double_quotes2() {
        assert_eq!(
            parse_prompt("echo \"hello\"\"world\""),
            vec!["echo", "helloworld"]
        );
    }

    #[test]
    fn test_double_quotes3() {
        assert_eq!(
            parse_prompt("echo \"hello\" \"world\""),
            vec!["echo", "hello", "world"]
        );
    }

    #[test]
    fn test_double_quotes4() {
        assert_eq!(
            parse_prompt("echo \"shell's test\""),
            vec!["echo", "shell's test"]
        );
    }

    #[test]
    fn test_backslash1() {
        assert_eq!(
            parse_prompt("echo world\\ \\ \\ \\ \\ \\ script"),
            vec!["echo", "world      script"]
        );
    }

    #[test]
    fn test_backslash2() {
        assert_eq!(
            parse_prompt("echo before\\ after"),
            vec!["echo", "before after"]
        );
    }

    #[test]
    fn test_backslash3() {
        assert_eq!(
            parse_prompt("echo test\nexample"),
            vec!["echo", "test", "example"]
        );
    }

    #[test]
    fn test_backslash4() {
        assert_eq!(
            parse_prompt("echo hello\\\\world"),
            vec!["echo", "hello\\world"]
        );
    }

    #[test]
    fn test_backslash5() {
        assert_eq!(parse_prompt("echo \'hello\'"), vec!["echo", "hello"]);
    }
}
