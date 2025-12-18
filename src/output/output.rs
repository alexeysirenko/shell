use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;

pub trait Output {
    fn print(&mut self, text: &str);
    fn is_redirected(&self) -> bool {
        false
    }
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

pub struct StdErrOutput;

impl StdErrOutput {
    pub fn new() -> Self {
        Self
    }
}

impl Output for StdErrOutput {
    fn print(&mut self, text: &str) {
        eprintln!("{}", text);
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

    pub fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            file: self.file.try_clone()?,
        })
    }
}

impl Output for FileOutput {
    fn print(&mut self, text: &str) {
        let _ = writeln!(self.file, "{}", text);
    }

    fn is_redirected(&self) -> bool {
        true
    }
}

pub struct OutputStreams {
    pub stdout: Box<dyn Output>,
    pub stderr: Box<dyn Output>,
}

impl OutputStreams {
    pub fn new(stdout: Box<dyn Output>, stderr: Box<dyn Output>) -> Self {
        Self { stdout, stderr }
    }

    pub fn default() -> Self {
        Self {
            stdout: Box::new(StdOutput::new()),
            stderr: Box::new(StdErrOutput::new()),
        }
    }
}
