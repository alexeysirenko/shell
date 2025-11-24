use std::process;

pub fn handle_command(command: &str) -> () {
    // dbg!(command);
    match command {
        "exit" => {
            println!("exiting...");
            process::exit(1)
        }
        _ => println!("{}: command not found", command.trim()),
    }
}
