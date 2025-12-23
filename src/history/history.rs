use anyhow::Result;

pub struct History {
    pub items: Vec<String>,
}

impl History {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_history_item(&mut self, line: &str) -> Result<()> {
        self.items.push(line.to_string());
        Ok(())
    }
}
