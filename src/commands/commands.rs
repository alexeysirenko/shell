use anyhow::{Result, anyhow};
use is_executable::IsExecutable;
use os_pipe::{PipeReader, pipe};
use std::fs;
use std::io::Write;
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::PathBuf;
use std::process::{Command as CmdCommand, Stdio};
use std::thread;
use std::{env, process};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};

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

fn is_built_in(command: &str) -> bool {
    command.parse::<CommandKind>().is_ok()
}

pub fn builtin_commands() -> Vec<String> {
    CommandKind::iter()
        .map(|k| format!("{:?}", k).to_lowercase())
        .collect()
}

pub fn execute_command(command: Command, input: Option<PipeReader>) -> Result<Option<PipeReader>> {
    match command {
        Command::Exit => process::exit(0),
        Command::Cd(path) => {
            cd(&path)?;
            Ok(None)
        }
        Command::Echo(text) => pipe_string(text),
        Command::Pwd => {
            let dir = fs::canonicalize(env::current_dir()?)?;
            pipe_string(dir.display().to_string())
        }
        Command::Type(cmd) => {
            let text = if is_built_in(&cmd) {
                format!("{} is a shell builtin", cmd)
            } else if let Some(path) = find_in_path(&cmd) {
                format!("{} is {}", cmd, path.display())
            } else {
                format!("{}: not found", cmd)
            };
            pipe_string(text)
        }
        Command::Exec { command, args } => exec_piped(&command, &args, input),
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
) -> Result<Option<PipeReader>> {
    find_in_path(command).ok_or_else(|| anyhow!("{}: command not found", command))?;

    let stdin_cfg = match &input {
        Some(reader) => unsafe { Stdio::from_raw_fd(reader.as_raw_fd()) },
        None => Stdio::inherit(),
    };

    let mut child = CmdCommand::new(command)
        .args(args)
        .stdin(stdin_cfg)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    drop(input);

    let stdout = child.stdout.take().expect("stdout was piped");
    let (reader, mut writer) = pipe()?;

    thread::spawn(move || {
        let mut stdout = stdout;
        std::io::copy(&mut stdout, &mut writer).ok();
        drop(writer);
        child.wait().ok();
    });

    Ok(Some(reader))
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
