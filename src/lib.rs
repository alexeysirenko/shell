use std::process;

pub fn handle_command(command: &str) -> () {
    // dbg!(command);
    match command {
        "exit" => process::exit(0),
        _ => println!("{}: command not found", command.trim()),
    }
}
