#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ExitCode(u8);

impl ExitCode {
    pub const SUCCESS: Self = Self(0);
    pub const FAILURE: Self = Self(1);

    pub fn new(code: u8) -> Self {
        Self(code)
    }

    pub fn as_u8(&self) -> u8 {
        self.0
    }

    pub fn is_success(&self) -> bool {
        self.0 == Self::SUCCESS.0
    }

    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }
}

impl From<u8> for ExitCode {
    fn from(code: u8) -> Self {
        Self(code)
    }
}

impl From<ExitCode> for u8 {
    fn from(exit_code: ExitCode) -> Self {
        exit_code.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: ExitCode,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutput {
    pub fn success() -> Self {
        Self {
            exit_code: ExitCode::SUCCESS,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    pub fn failure(exit_code: ExitCode, stderr: impl Into<String>) -> Self {
        Self {
            exit_code,
            stdout: String::new(),
            stderr: stderr.into(),
        }
    }
}

impl Default for CommandOutput {
    fn default() -> Self {
        Self::success()
    }
}
