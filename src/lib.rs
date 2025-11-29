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
    #[strum(serialize = "pwd")]
    Pwd,
}

impl CommandKind {
    const BUILTINS: &'static [CommandKind] = &[
        CommandKind::Exit,
        CommandKind::Echo,
        CommandKind::Type,
        CommandKind::Pwd,
    ];

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
    Pwd,
}

pub fn parse_command(prompt: &str) -> Result<Command> {
    let parts: Vec<&str> = prompt.split_ascii_whitespace().collect();
    let (name, args) = parts
        .split_first()
        .ok_or_else(|| anyhow!("Empty command"))?;

    let arg_str = args.join(" ");

    match name.parse::<CommandKind>() {
        Ok(CommandKind::Exit) => Ok(Command::Exit),
        Ok(CommandKind::Echo) => Ok(Command::Echo(arg_str)),
        Ok(CommandKind::Type) => Ok(Command::Type(arg_str)),
        Ok(CommandKind::Pwd) => Ok(Command::Pwd),
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
        Command::Exec { command, args } => try_exec(&command, &args),
        Command::Pwd => pwd(),
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

fn try_exec(command: &str, args: &[String]) {
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
                fs::canonicalize(&full_path).ok()
            } else {
                None
            }
        })
    })
}

fn exec(command: &str, args: &[String]) -> Result<i32> {
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

    let code = child.wait()?;
    Ok(code.code().unwrap_or(1))
}

fn pwd() {
    if let Ok(dir) = env::current_dir() {
        if let Ok(absolute) = fs::canonicalize(dir) {
            println!("{}", absolute.display())
        }
    }
}
