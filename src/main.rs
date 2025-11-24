use io::stdin;
#[allow(unused_imports)]
use std::io::{self, Write};

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    let mut command = "".to_string();
    stdin().read_line(&mut command).unwrap();
    println!("{}: command not found", command.trim());
}
