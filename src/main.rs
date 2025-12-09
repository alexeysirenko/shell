use codecrafters_shell::completer::ShellCompleter;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};

use codecrafters_shell::{handle_command, parse_command, parse_prompt};

fn main() {
    let config = Config::builder()
        .completion_type(CompletionType::Circular)
        .build();
    let mut rl = Editor::with_config(config).unwrap();
    rl.set_helper(Some(ShellCompleter::new()));

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
