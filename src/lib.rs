use anyhow::{Result, anyhow};
use is_executable::IsExecutable;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command as CmdCommand, Stdio};
use std::{env, process};
use strum_macros::{EnumIter, EnumString};

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

pub fn parse_command(args: Vec<String>) -> Result<Command> {
    let (name, args) = args.split_first().ok_or_else(|| anyhow!("Empty command"))?;

    let arg_str = args.join(" ");

    match name.parse::<CommandKind>() {
        Ok(CommandKind::Exit) => Ok(Command::Exit),
        Ok(CommandKind::Echo) => Ok(Command::Echo(arg_str)),
        Ok(CommandKind::Type) => Ok(Command::Type(arg_str)),
        Ok(CommandKind::Pwd) => Ok(Command::Pwd),
        Ok(CommandKind::Cd) => Ok(Command::Cd(arg_str)),
        Err(_) => Ok(Command::Exec {
            command: name.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }),
    }
}

pub fn handle_command(command: Command) {
    match command {
        Command::Exit => exit(),
        Command::Echo(text) => echo(&text),
        Command::Type(command) => r#type(&command),
        Command::Exec { command, args } => try_run(|| exec(&command, &args)),
        Command::Pwd => pwd(),
        Command::Cd(path) => try_run(|| cd(&path)),
    }
}

fn exit() {
    process::exit(0)
}

fn echo(text: &str) {
    println!("{text}")
}

fn r#type(command: &str) {
    if is_built_in(command) {
        println!("{} is a shell builtin", command);
    } else if let Some(path) = find_in_path(command) {
        println!("{command} is {}", path.display());
    } else {
        println!("{}: not found", command);
    }
}

fn try_run<F>(f: F)
where
    F: FnOnce() -> Result<()>,
{
    if let Err(e) = f() {
        println!("{e}");
    }
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

fn exec(command: &str, args: &[String]) -> Result<()> {
    find_in_path(command).ok_or(anyhow!("{command}: command not found"))?;

    let mut child = CmdCommand::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            println!("{line}");
        }
    }

    let _code = child.wait()?;
    Ok(())
}

fn pwd() {
    if let Ok(dir) = env::current_dir() {
        if let Ok(absolute) = fs::canonicalize(dir) {
            println!("{}", absolute.display())
        }
    }
}

fn cd(path: &str) -> Result<()> {
    let target = match path {
        "" | "~" => env::home_dir(),
        p if p.starts_with("~/") => env::home_dir().map(|home| home.join(&p[2..])),
        p => Some(PathBuf::from(p)),
    };

    match target {
        Some(t) => {
            env::set_current_dir(t).map_err(|_| anyhow!("cd: {path}: No such file or directory"))
        }
        None => Err(anyhow!("cd: {path}: No such file or directory")),
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
