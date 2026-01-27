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

/// Returns Some(commands) if additional commands need to be executed (e.g., from history -r)
pub fn handle_pipeline(commands: Vec<Command>, streams: &mut OutputStreams, history: &History) -> Option<Vec<String>> {
    let mut commands = commands;
    let len = commands.len();

    if len == 0 {
        return None;
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
            Ok(ExecuteResult::Pipe(output)) => previous_stdout = output,
            Ok(ExecuteResult::RunCommands(cmds)) => return Some(cmds),
            Err(e) => {
                streams.stderr.print(&e.to_string());
                return None;
            }
        }
    }

    match execute_command(
        last_command,
        previous_stdout,
        Some(&mut *streams.stdout),
        &mut *streams.stderr,
        history,
    ) {
        Ok(ExecuteResult::RunCommands(cmds)) => Some(cmds),
        Err(e) => {
            streams.stderr.print(&e.to_string());
            None
        }
        _ => None,
    }
}
