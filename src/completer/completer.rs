use rustyline::{
    Helper,
    completion::{Completer, Pair},
    highlight::Highlighter,
    hint::Hinter,
    validate::Validator,
};

pub struct ShellCompleter {
    commands: Vec<String>,
}

impl ShellCompleter {
    pub fn new(mut commands: Vec<String>) -> Self {
        commands.sort();
        Self { commands }
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

        let matches: Vec<Pair> = self
            .commands
            .iter()
            .filter(|cmd| cmd.starts_with(word))
            .map(|cmd| Pair {
                display: cmd.clone(),
                replacement: cmd.clone(),
            })
            .collect();

        Ok((word_start, matches))
    }
}
impl Hinter for ShellCompleter {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<Self::Hint> {
        None
    }
}
impl Highlighter for ShellCompleter {}
impl Validator for ShellCompleter {}
impl Helper for ShellCompleter {}
