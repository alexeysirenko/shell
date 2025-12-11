pub mod completer;
pub mod finder;
mod output;
pub mod parser;

use anyhow::{Result, anyhow};
use is_executable::IsExecutable;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command as CmdCommand, Stdio};
use std::{env, process};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};

pub use crate::output::{FileOutput, Output, OutputStreams, StdErrOutput, StdOutput};


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

pub fn builtin_commands() -> Vec<String> {
    CommandKind::iter()
        .map(|k| format!("{:?}", k).to_lowercase())
        .collect()
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
