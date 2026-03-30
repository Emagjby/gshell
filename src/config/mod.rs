use nu_ansi_term::Color;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlighterConfig {
    command_color: Color,
    builtin_color: Color,
    argument_color: Color,
    flag_color: Color,
    hint_color: Color,
    operator_color: Color,
    redirect_color: Color,
}

impl Default for HighlighterConfig {
    fn default() -> Self {
        Self {
            command_color: Color::Cyan,
            builtin_color: Color::Cyan,
            argument_color: Color::Green,
            flag_color: Color::Blue,
            hint_color: Color::DarkGray,
            operator_color: Color::Purple,
            redirect_color: Color::Purple,
        }
    }
}

impl HighlighterConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(value) = std::env::var("GSHELL_HIGHLIGHT_COMMAND")
            && let Some(color) = parse_color(&value)
        {
            config.command_color = color;
        }

        if let Ok(value) = std::env::var("GSHELL_HIGHLIGHT_BUILTIN")
            && let Some(color) = parse_color(&value)
        {
            config.builtin_color = color;
        }

        if let Ok(value) = std::env::var("GSHELL_HIGHLIGHT_ARGUMENT")
            && let Some(color) = parse_color(&value)
        {
            config.argument_color = color;
        }

        if let Ok(value) = std::env::var("GSHELL_HIGHLIGHT_FLAG")
            && let Some(color) = parse_color(&value)
        {
            config.flag_color = color;
        }

        if let Ok(value) = std::env::var("GSHELL_HIGHLIGHT_HINT")
            && let Some(color) = parse_color(&value)
        {
            config.hint_color = color;
        }

        if let Ok(value) = std::env::var("GSHELL_HIGHLIGHT_OPERATOR")
            && let Some(color) = parse_color(&value)
        {
            config.operator_color = color;
        }

        if let Ok(value) = std::env::var("GSHELL_HIGHLIGHT_REDIRECT")
            && let Some(color) = parse_color(&value)
        {
            config.redirect_color = color;
        }

        config
    }

    pub fn command_color(&self) -> Color {
        self.command_color
    }

    pub fn builtin_color(&self) -> Color {
        self.builtin_color
    }

    pub fn operator_color(&self) -> Color {
        self.operator_color
    }

    pub fn argument_color(&self) -> Color {
        self.argument_color
    }

    pub fn hint_color(&self) -> Color {
        self.hint_color
    }

    pub fn flag_color(&self) -> Color {
        self.flag_color
    }

    pub fn redirect_color(&self) -> Color {
        self.redirect_color
    }
}

fn parse_color(value: &str) -> Option<Color> {
    let value = value.trim();

    if let Some(hex) = value.strip_prefix('#').or(Some(value))
        && hex.len() == 6
        && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some(Color::Rgb(red, green, blue));
    }

    match value.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "purple" | "magenta" => Some(Color::Purple),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "dark_gray" | "dark-gray" | "darkgray" => Some(Color::DarkGray),
        "light_red" | "light-red" | "lightred" => Some(Color::LightRed),
        "light_green" | "light-green" | "lightgreen" => Some(Color::LightGreen),
        "light_yellow" | "light-yellow" | "lightyellow" => Some(Color::LightYellow),
        "light_blue" | "light-blue" | "lightblue" => Some(Color::LightBlue),
        "light_purple" | "light-purple" | "lightpurple" => Some(Color::LightPurple),
        "light_cyan" | "light-cyan" | "lightcyan" => Some(Color::LightCyan),
        "light_gray" | "light-gray" | "lightgray" => Some(Color::LightGray),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use nu_ansi_term::Color;

    use super::parse_color;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(parse_color("#31748f"), Some(Color::Rgb(0x31, 0x74, 0x8f)));
        assert_eq!(parse_color("eb6f92"), Some(Color::Rgb(0xeb, 0x6f, 0x92)));
    }

    #[test]
    fn parses_named_colors() {
        assert_eq!(parse_color("blue"), Some(Color::Blue));
        assert_eq!(parse_color("light-purple"), Some(Color::LightPurple));
    }

    #[test]
    fn rejects_invalid_colors() {
        assert_eq!(parse_color("rose-pine"), None);
        assert_eq!(parse_color("#12345"), None);
    }
}
