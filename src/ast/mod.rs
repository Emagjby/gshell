#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCommand {
    pub argv: Vec<String>,
}

impl SimpleCommand {
    pub fn new(argv: Vec<String>) -> Self {
        Self { argv }
    }

    pub fn is_empty(&self) -> bool {
        self.argv.is_empty()
    }
}
