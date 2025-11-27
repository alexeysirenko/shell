use anyhow::{Result, anyhow};
use is_executable::IsExecutable;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command as CmdCommand, Stdio};
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
    Exec { command: String, args: Vec<String> },
}

pub fn parse_command(prompt: &str) -> Result<Command> {
    let parts: Vec<&str> = prompt.split_ascii_whitespace().collect();
    let (name, args) = parts
        .split_first()
        .ok_or_else(|| anyhow!("Empty command"))?;

    let maybe_kind: Result<CommandKind> = name.parse().map_err(|_| anyhow!("{name}: not found"));

    if let Ok(known_kind) = maybe_kind {
        match known_kind {
            CommandKind::Exit => Ok(Command::Exit),
            CommandKind::Echo => Ok(Command::Echo(args.join(" "))),
            CommandKind::Type => Ok(Command::Type(args.join(" "))),
        }
    } else {
        Ok(Command::Exec {
            command: name.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        })
    }
}

pub fn handle_command(command: Command) -> () {
    match command {
        Command::Exit => exit(),
        Command::Echo(text) => echo(&text),
        Command::Type(command) => r#type(&command),
        Command::Exec { command, args } => try_exec(command, args),
    }
}

fn exit() -> () {
    process::exit(0)
}

fn echo(text: &str) -> () {
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

fn try_exec(command: String, args: Vec<String>) -> () {
    if let Err(e) = exec(command, args) {
        println!("{e}");
    }
}

fn is_built_in(command: &str) -> bool {
    command
        .parse::<CommandKind>()
        .map(|k| k.is_builtin())
        .unwrap_or(false)
}

fn find_in_path(executable: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let full_path = dir.join(executable);
            if full_path.is_file() && full_path.is_executable() {
                /*
                match fs::canonicalize(&full_path) {
                    Ok(absolute) => Some(absolute),
                    Err(_) => None,
                }
                */
                Some(full_path)
            } else {
                None
            }
        })
    })
}

fn exec(command: String, args: Vec<String>) -> Result<()> {
    let absolute_path = find_in_path(&command).ok_or_else(|| anyhow!("{}: not found", command))?;

    let mut child = CmdCommand::new(absolute_path)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            println!("{line}");
        }
    }

    child.wait()?;
    Ok(())
}
