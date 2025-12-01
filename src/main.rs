use codecrafters_shell::{handle_command, parse_command, parse_prompt};
use io::stdin;
#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();
        let mut prompt = "".to_string();
        stdin().read_line(&mut prompt).unwrap();
        let prompt = prompt.trim();
        if prompt.is_empty() {
            continue;
        }

        match parse_command(parse_prompt(prompt)) {
            Ok(command) => handle_command(command),
            Err(_) => println!("{prompt}: command not found"),
        }
    }
}
