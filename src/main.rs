use std::collections::HashSet;

use codecrafters_shell::completer::ShellCompleter;
use codecrafters_shell::finder::ExecutablesFinder;
use codecrafters_shell::parser::{parse_pipeline, parse_prompt};
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};

use codecrafters_shell::{History, builtin_commands, handle_pipeline};

fn execute_line(line: &str, history: &mut History) -> bool {
    let prompt = line.trim();
    if prompt.is_empty() {
        return false;
    }

    history.add_history_item(line).ok();

    match parse_pipeline(parse_prompt(prompt)) {
        Ok((command, mut streams)) => {
            let (lines_to_add, should_exit) = handle_pipeline(command, &mut streams, history);
            if let Some(lines) = lines_to_add {
                for line in lines {
                    history.add_history_item(&line).ok();
                }
            }
            should_exit
        }
        Err(error) => {
            eprintln!("{}: {}", prompt, error);
            false
        }
    }
}

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

    let mut history = History::new();
    if let Some(path) = history.histfile.clone() {
        history.load_from_file(&path).ok();
    }
    loop {
        match rl.readline("$ ") {
            Ok(line) => {
                let prompt = line.trim();
                if prompt.is_empty() {
                    continue;
                }

                rl.add_history_entry(&line).ok();
                if execute_line(&line, &mut history) {
                    break;
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

    if let Some(path) = history.histfile.clone() {
        history.save_to_file(&path).ok();
    }
}
