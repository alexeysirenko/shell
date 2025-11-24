use codecrafters_shell::handle_command;
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

        handle_command(&prompt)
    }
}
