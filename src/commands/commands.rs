use anyhow::{Result, anyhow};
use is_executable::IsExecutable;
use os_pipe::{PipeReader, pipe};
use std::fs;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::io::FromRawFd;
use std::os::unix::io::IntoRawFd;
use std::path::PathBuf;
use std::process::{Command as CmdCommand, Stdio};
use std::thread;
use std::{env, process};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};

use crate::Output;

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
    Echo {
        text: String,
        interpret_escapes: bool,
    },
    Type(String),
    Exec {
        command: String,
        args: Vec<String>,
    },
    Pwd,
    Cd(String),
}

fn is_built_in(command: &str) -> bool {
    command.parse::<CommandKind>().is_ok()
}

pub fn builtin_commands() -> Vec<String> {
    CommandKind::iter()
        .map(|k| format!("{:?}", k).to_lowercase())
        .collect()
}

pub fn execute_command(
    command: Command,
    input: Option<PipeReader>,
    stdout_output: Option<&mut dyn Output>,
    stderr_output: &mut dyn Output,
) -> Result<Option<PipeReader>> {
    match command {
        Command::Exit => process::exit(0),
        Command::Cd(path) => {
            cd(&path)?;
            Ok(None)
        }
        Command::Echo {
            text,
            interpret_escapes,
        } => {
            let output = if interpret_escapes {
                interpret_escape_sequences(&text)
            } else {
                text
            };
            if let Some(out) = stdout_output {
                out.print(&output);
                Ok(None)
            } else {
                pipe_string(output)
            }
        }
        Command::Pwd => {
            let dir = fs::canonicalize(env::current_dir()?)?;
            let text = dir.display().to_string();
            if let Some(out) = stdout_output {
                out.print(&text);
                Ok(None)
            } else {
                pipe_string(text)
            }
        }
        Command::Type(cmd) => {
            let text = if is_built_in(&cmd) {
                format!("{} is a shell builtin", cmd)
            } else if let Some(path) = find_in_path(&cmd) {
                format!("{} is {}", cmd, path.display())
            } else {
                format!("{}: not found", cmd)
            };
            if let Some(out) = stdout_output {
                out.print(&text);
                Ok(None)
            } else {
                pipe_string(text)
            }
        }
        Command::Exec { command, args } => {
            let is_final = stdout_output.is_some();
            exec_piped(
                &command,
                &args,
                input,
                is_final,
                stdout_output,
                stderr_output,
            )
        }
    }
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

fn exec_piped(
    command: &str,
    args: &[String],
    input: Option<PipeReader>,
    is_final: bool,
    stdout_output: Option<&mut dyn Output>,
    stderr_output: &mut dyn Output,
) -> Result<Option<PipeReader>> {
    find_in_path(command).ok_or_else(|| anyhow!("{}: command not found", command))?;

    let stdin_cfg = match input {
        Some(reader) => unsafe { Stdio::from_raw_fd(reader.into_raw_fd()) },
        None => Stdio::inherit(),
    };

    let is_stdout_redirected = stdout_output
        .as_ref()
        .map(|o| o.is_redirected())
        .unwrap_or(false);

    let stdout_cfg = if is_final && !is_stdout_redirected {
        Stdio::inherit()
    } else {
        Stdio::piped()
    };

    let is_stderr_redirected = stderr_output.is_redirected();
    let stderr_cfg = if is_stderr_redirected {
        Stdio::piped()
    } else {
        Stdio::inherit()
    };

    let mut child = CmdCommand::new(command)
        .args(args)
        .stdin(stdin_cfg)
        .stdout(stdout_cfg)
        .stderr(stderr_cfg)
        .spawn()?;

    // Handle stderr if redirected
    if is_stderr_redirected {
        if let Some(stderr) = child.stderr.take() {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                stderr_output.print(&line);
            }
        }
    }

    if is_final && is_stdout_redirected {
        if let Some(stdout) = child.stdout.take() {
            if let Some(out) = stdout_output {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    out.print(&line);
                }
            }
        }
        child.wait()?;
        Ok(None)
    } else if is_final {
        child.wait()?;
        Ok(None)
    } else {
        let stdout = child.stdout.take().expect("stdout was piped");
        let reader = unsafe { PipeReader::from_raw_fd(stdout.into_raw_fd()) };

        thread::spawn(move || {
            child.wait().ok();
        });

        Ok(Some(reader))
    }
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

fn pipe_string(text: String) -> Result<Option<PipeReader>> {
    let (reader, mut writer) = pipe()?;
    thread::spawn(move || {
        let _ = writeln!(writer, "{}", text);
    });
    Ok(Some(reader))
}

fn interpret_escape_sequences(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('a') => result.push('\x07'),
                Some('b') => result.push('\x08'),
                Some('f') => result.push('\x0C'),
                Some('v') => result.push('\x0B'),
                Some('e') => result.push('\x1B'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }

    result
}
