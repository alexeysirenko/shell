use std::collections::HashSet;

use codecrafters_shell::completer::ShellCompleter;
use codecrafters_shell::finder::ExecutablesFinder;
use codecrafters_shell::parser::{parse_command, parse_prompt};
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};

use codecrafters_shell::{builtin_commands, handle_command};

fn main() {
    let path_executables = ExecutablesFinder::new().find_executables_in_path().unwrap();

    let builtin_commands = builtin_commands();
    let all_commands = path_executables
        .into_iter()
        .chain(builtin_commands)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<String>>();

    let config = Config::builder()
        .completion_type(CompletionType::List)
        .completion_prompt_limit(100)
        .build();
    let mut rl = Editor::with_config(config).unwrap();
    rl.set_helper(Some(ShellCompleter::new(all_commands)));

    loop {
        match rl.readline("$ ") {
            Ok(line) => {
                let prompt = line.trim();
                if prompt.is_empty() {
                    continue;
                }

                rl.add_history_entry(&line).ok();

                match parse_command(parse_prompt(prompt)) {
                    Ok((command, mut streams)) => handle_command(command, &mut streams),
                    Err(_) => eprintln!("{}: command not found", prompt),
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("error: {:?}", err);
                break;
            }
        }
    }
}
