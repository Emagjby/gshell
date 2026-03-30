#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptMode {
    Internal,
    Starship,
    #[default]
    Auto,
}

impl PromptMode {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "internal" => Self::Internal,
            "starship" => Self::Starship,
            "auto" => Self::Auto,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptConfig {
    mode: PromptMode,
    starship_binary: String,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            mode: PromptMode::Auto,
            starship_binary: "starship".to_string(),
        }
    }
}

impl PromptConfig {
    pub fn new(mode: PromptMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(mode) = std::env::var("GSHELL_PROMPT") {
            config.mode = PromptMode::parse(&mode);
        }

        if let Ok(binary) = std::env::var("GSHELL_STARSHIP_BIN")
            && !binary.trim().is_empty()
        {
            config.starship_binary = binary;
        }

        config
    }

    pub fn mode(&self) -> PromptMode {
        self.mode
    }

    pub fn starship_binary(&self) -> &str {
        &self.starship_binary
    }

    pub fn with_starship_binary(mut self, binary: impl Into<String>) -> Self {
        self.starship_binary = binary.into();
        self
    }
}
