pub mod commands;
pub mod completer;
pub mod finder;
pub mod history;
pub mod output;
pub mod parser;

use os_pipe::PipeReader;

pub use crate::commands::*;
pub use crate::history::*;
pub use crate::output::{FileOutput, Output, OutputStreams, StdErrOutput, StdOutput};

pub fn handle_pipeline(commands: Vec<Command>, streams: &mut OutputStreams, history: &History) {
    let mut commands = commands;
    let len = commands.len();

    if len == 0 {
        return;
    }

    let last_command = commands.pop().unwrap();
    let mut previous_stdout: Option<PipeReader> = None;

    for command in commands {
        match execute_command(
            command,
            previous_stdout.take(),
            None,
            &mut *streams.stderr,
            history,
        ) {
            Ok(output) => previous_stdout = output,
            Err(e) => {
                streams.stderr.print(&e.to_string());
                return;
            }
        }
    }

    if let Err(e) = execute_command(
        last_command,
        previous_stdout,
        Some(&mut *streams.stdout),
        &mut *streams.stderr,
        history,
    ) {
        streams.stderr.print(&e.to_string());
    }
}
