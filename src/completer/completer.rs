use rustyline::{
    Helper, completion::{Completer, Pair}, highlight::Highlighter, hint::Hinter, validate::Validator
};
use strum::IntoEnumIterator;

use crate::CommandKind;

pub struct ShellCompleter;

impl ShellCompleter {
    pub fn new() -> Self {
        Self {}
    }

    fn builtin_commands() -> Vec<String> {
        CommandKind::iter()
            .map(|k| format!("{:?}", k).to_lowercase())
            .collect()
    }
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self, // FIXME should be `&mut self`
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let word_start = line[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[word_start..pos];

        let matches: Vec<Pair> = Self::builtin_commands()
            .into_iter()
            .filter(|cmd| cmd.starts_with(word))
            .map(|cmd| Pair {
                display: cmd.clone(),
                replacement: cmd,
            })
            .collect();

        Ok((word_start, matches))
    }
}

impl Hinter for ShellCompleter {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        if pos < line.len() {
            return None;
        }

        let word_start = line[..pos].rfind(' ').map(|i| i + 1).unwrap_or(0);
        let word = &line[word_start..pos];

        if word.is_empty() {
            return None;
        }

        Self::builtin_commands()
            .into_iter()
            .find(|cmd| cmd.starts_with(word) && cmd.len() > word.len())
            .map(|cmd| cmd[word.len()..].to_string())
    }
}

impl Highlighter for ShellCompleter {}
impl Validator for ShellCompleter {}
impl Helper for ShellCompleter {}
