pub mod commands;
pub mod completer;
pub mod finder;
mod output;
pub mod parser;

use os_pipe::PipeReader;
use std::io::Read;

pub use crate::commands::*;
pub use crate::output::{FileOutput, Output, OutputStreams, StdErrOutput, StdOutput};

pub fn handle_pipeline(commands: Vec<Command>, streams: &mut OutputStreams) {
    let mut commands = commands;
    let len = commands.len();

    if len == 0 {
        return;
    }

    let last_command = commands.pop().unwrap();
    let mut previous_stdout: Option<PipeReader> = None;

    for (i, command) in commands.into_iter().enumerate() {
        let is_last = i == len - 1;

        match execute_command(command, previous_stdout.take(), None) {
            Ok(output) => {
                if is_last {
                    if let Some(mut reader) = output {
                        let mut buf = String::new();
                        if reader.read_to_string(&mut buf).is_ok() {
                            for line in buf.lines() {
                                streams.stdout.print(line);
                            }
                        }
                    }
                } else {
                    previous_stdout = output;
                }
            }
            Err(e) => {
                streams.stderr.print(&e.to_string());
                return;
            }
        }
    }

    if let Err(e) = execute_command(last_command, previous_stdout, Some(&mut *streams.stdout)) {
        streams.stderr.print(&e.to_string());
    }
}
