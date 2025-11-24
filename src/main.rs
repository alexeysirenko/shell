use codecrafters_shell::handle_command;
use io::stdin;
#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let mut command = "".to_string();
        stdin().read_line(&mut command).unwrap();
        handle_command(command.trim())
    }
}
