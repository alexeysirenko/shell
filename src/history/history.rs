use anyhow::Result;

#[derive(Default)]
pub struct History {
    pub items: Vec<String>,
    pub last_saved_index: usize,
    pub histfile: Option<String>,
}

impl History {
    pub fn new() -> Self {
        let histfile = std::env::var("HISTFILE").ok();
        Self {
            items: Vec::new(),
            last_saved_index: 0,
            histfile,
        }
    }

    pub fn add_history_item(&mut self, line: &str) -> Result<()> {
        self.items.push(line.to_string());
        Ok(())
    }

    pub fn load_from_file(&mut self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        for line in content.lines() {
            if !line.trim().is_empty() {
                self.items.push(line.to_string());
            }
        }
        self.last_saved_index = self.items.len();
        Ok(())
    }

    pub fn append_to_file(&mut self, path: &str) -> Result<()> {
        use std::fs::OpenOptions;
        use std::io::Write;

        let new_items = &self.items[self.last_saved_index..];
        if new_items.is_empty() {
            return Ok(());
        }
        let contents = new_items.join("\n") + "\n";

        let mut file = OpenOptions::new().create(true).append(false).open(path)?;
        write!(file, "{}", contents)?;
        self.last_saved_index = self.items.len();
        Ok(())
    }
}
