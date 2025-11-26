use anyhow::{Result, anyhow};
use is_executable::IsExecutable;
use std::fs;
use std::path::PathBuf;
use std::{env, process};
use strum_macros::{EnumIter, EnumString};

#[derive(Debug, EnumString, EnumIter, PartialEq)]
pub enum CommandKind {
    #[strum(serialize = "exit")]
    Exit,
    #[strum(serialize = "echo")]
    Echo,
    #[strum(serialize = "type")]
    Type,
}

impl CommandKind {
    const BUILTINS: &'static [CommandKind] =
        &[CommandKind::Exit, CommandKind::Echo, CommandKind::Type];

    fn is_builtin(&self) -> bool {
        Self::BUILTINS.contains(self)
    }
}

#[derive(Debug)]
pub enum Command {
    Exit,
    Echo(String),
    Type(String),
}

pub fn parse_command(prompt: &str) -> Result<Command> {
    let parts: Vec<&str> = prompt.split_ascii_whitespace().collect();
    let (name, args) = parts
        .split_first()
        .ok_or_else(|| anyhow!("Empty command"))?;

    let kind: CommandKind = name.parse().map_err(|_| anyhow!("{name}: not found"))?;
    let arg = args.join(" ");

    match kind {
        CommandKind::Exit => Ok(Command::Exit),
        CommandKind::Echo => Ok(Command::Echo(arg)),
        CommandKind::Type => Ok(Command::Type(arg)),
    }
}

pub fn handle_command(command: Command) -> () {
    match command {
        Command::Exit => exit(),
        Command::Echo(text) => echo(&text),
        Command::Type(command) => r#type(&command),
    }
}

fn exit() -> () {
    process::exit(0)
}

fn echo(text: &str) -> () {
    println!("{text}")
}

fn r#type(command: &str) {
    let is_built_in = command
        .parse::<CommandKind>()
        .map(|k| k.is_builtin())
        .unwrap_or(false);

    if is_built_in {
        println!("{} is a shell builtin", command);
    } else if let Some(path) = find_in_path(command) {
        println!("{command} is {}", path.display());
    } else {
        println!("{}: not found", command);
    }
}

fn find_in_path(executable: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(executable);
            if full_path.is_file() && full_path.is_executable() {
                match fs::canonicalize(&full_path) {
                    Ok(absolute) => Some(absolute),
                    Err(_) => None,
                }
            } else {
                None
            }
        })
    })
}
