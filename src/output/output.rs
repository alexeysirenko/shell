use anyhow::{Context, Result, anyhow};
use std::fs::{self, File, OpenOptions};
use std::io::Write;

pub trait Output {
    fn print(&mut self, text: &str);
}

pub struct StdOutput;

impl StdOutput {
    pub fn new() -> Self {
        Self
    }
}

impl Output for StdOutput {
    fn print(&mut self, text: &str) {
        println!("{}", text);
    }
}

pub struct FileOutput {
    file: File,
}

impl FileOutput {
    pub fn new(path: &str, append: bool) -> Result<Self> {
        let file = if append {
            OpenOptions::new().create(true).append(true).open(path)
        } else {
            File::create(path)
        }
        .with_context(|| format!("{}: cannot open file", path))?;

        Ok(Self { file })
    }
}

impl Output for FileOutput {
    fn print(&mut self, text: &str) {
        let _ = writeln!(self.file, "{}", text);
    }
}
