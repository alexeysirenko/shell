use crate::{
    Command, CommandKind,
    output::{FileOutput, Output, OutputStreams, StdErrOutput, StdOutput},
};
use anyhow::{Result, anyhow};

pub enum PromptQuote {
    Unquoted,
    SingleQuoted,
    DoubleQuoted,
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
                '|' => {
                    push(&mut buffer, &mut tokens);
                    tokens.push("|".to_string());
                }
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
            "2>>" => {
                let path = iter
                    .next()
                    .ok_or_else(|| anyhow!("redirect path missing"))?;
                let file = FileOutput::new(path, true)?;
                stderr = Box::new(file);
            }
            _ => filtered.push(arg.clone()),
        }
    }

    Ok((filtered, stdout, stderr))
}

pub fn parse_pipeline(tokens: Vec<String>) -> Result<(Vec<Command>, OutputStreams)> {
    let segments: Vec<Vec<String>> = tokens
        .split(|t| t == "|")
        .map(|s| s.to_vec())
        .filter(|s| !s.is_empty())
        .collect();

    if segments.is_empty() {
        return Err(anyhow!("empty pipeline"));
    }

    let mut commands = Vec::new();
    let mut final_streams: Option<OutputStreams> = None;

    for (i, segment) in segments.iter().enumerate() {
        let is_last = i == segments.len() - 1;
        let (command, streams) = parse_command(segment.clone())?;
        commands.push(command);

        if is_last {
            final_streams = Some(streams);
        }
    }

    Ok((
        commands,
        final_streams.unwrap_or_else(|| {
            OutputStreams::new(Box::new(StdOutput::new()), Box::new(StdErrOutput::new()))
        }),
    ))
}

fn parse_command(args: Vec<String>) -> Result<(Command, OutputStreams)> {
    let (name, rest) = args.split_first().ok_or_else(|| anyhow!("Empty command"))?;
    let (args, stdout, stderr) = extract_redirects(rest)?;

    let arg_str = args.join(" ");

    let command = match name.parse::<CommandKind>() {
        Ok(CommandKind::Exit) => Command::Exit,
        Ok(CommandKind::Echo) => {
            let (interpret_escapes, text) = if args.first().map(|arg| arg.as_str()) == Some("-e") {
                (true, args[1..].join(" "))
            } else {
                (false, arg_str)
            };
            Command::Echo {
                text,
                interpret_escapes,
            }
        }
        Ok(CommandKind::Type) => Command::Type(arg_str),
        Ok(CommandKind::Pwd) => Command::Pwd,
        Ok(CommandKind::Cd) => Command::Cd(arg_str),
        Ok(CommandKind::History) => Command::History,
        Err(_) => Command::Exec {
            command: name.to_string(),
            args,
        },
    };

    Ok((command, OutputStreams::new(stdout, stderr)))
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

    #[test]
    fn test_pipe_simple() {
        assert_eq!(
            parse_prompt("ls | grep foo"),
            vec!["ls", "|", "grep", "foo"]
        );
    }

    #[test]
    fn test_pipe_no_spaces() {
        assert_eq!(parse_prompt("ls|grep foo"), vec!["ls", "|", "grep", "foo"]);
    }

    #[test]
    fn test_pipe_chain() {
        assert_eq!(
            parse_prompt("cat file | grep foo | wc -l"),
            vec!["cat", "file", "|", "grep", "foo", "|", "wc", "-l"]
        );
    }

    #[test]
    fn test_pipe_in_quotes() {
        // Pipe inside quotes should not be treated as separator
        assert_eq!(
            parse_prompt("echo 'hello | world'"),
            vec!["echo", "hello | world"]
        );
    }
}
