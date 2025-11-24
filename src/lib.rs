use std::process;

pub fn handle_command(prompt: &str) -> () {
    let parts: Vec<&str> = prompt.split_ascii_whitespace().collect();
    match parts.as_slice() {
        ["exit", ..] => exit(),
        ["echo", text] => echo(text),
        _ => println!("{}: command not found", prompt),
    }
}

fn exit() -> () {
    process::exit(0)
}

fn echo(text: &str) -> () {
    println!("{text}")
}
