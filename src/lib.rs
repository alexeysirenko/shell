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

pub fn handle_pipeline(
    commands: Vec<Command>,
    streams: &mut OutputStreams,
    history: &mut History,
) -> (Option<Vec<String>>, bool) {
    let mut commands = commands;
    let len = commands.len();

    if len == 0 {
        return (None, false);
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
            Ok(ExecuteResult::Exit) => return (None, true),
            Ok(ExecuteResult::Pipe(output)) => previous_stdout = output,
            Ok(ExecuteResult::AddToHistory(lines)) => return (Some(lines), false),
            Ok(ExecuteResult::UpdateLastSaved(index)) => {
                history.last_saved_index = index;
            }
            Err(e) => {
                streams.stderr.print(&e.to_string());
                return (None, false);
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
        Ok(ExecuteResult::Exit) => (None, true),
        Ok(ExecuteResult::AddToHistory(lines)) => (Some(lines), false),
        Ok(ExecuteResult::UpdateLastSaved(index)) => {
            history.last_saved_index = index;
            (None, false)
        }
        Err(e) => {
            streams.stderr.print(&e.to_string());
            (None, false)
        }
        _ => (None, false),
    }
}
