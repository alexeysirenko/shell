use anyhow::Result;

#[derive(Default)]
pub struct History {
    pub items: Vec<String>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_history_item(&mut self, line: &str) -> Result<()> {
        self.items.push(line.to_string());
        Ok(())
    }
}
