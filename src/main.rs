use std::io::{self, Write, stdin};

use codecrafters_shell::{handle_command, parse_command, parse_prompt};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut prompt = String::new();
        stdin().read_line(&mut prompt).unwrap();
        let prompt = prompt.trim();

        if prompt.is_empty() {
            continue;
        }

        match parse_command(parse_prompt(prompt)) {
            Ok((command, mut streams)) => handle_command(command, &mut streams),
            Err(_) => eprintln!("{}: command not found", prompt),
        }
    }
}
