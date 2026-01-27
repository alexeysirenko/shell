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
}
