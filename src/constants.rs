use std::collections::HashMap;

pub const STATUS_OK: u32 = 0;
pub const STATUS_RETURN: u32 = 200;
pub const STATUS_STOP: u32 = 201;
pub const STATUS_NEXT: u32 = 202;
pub const STATUS_LAST: u32 = 203;
pub const STATUS_DONE: u32 = 204;
pub const STATUS_DECLINED: u32 = 205;
pub const STATUS_ERROR: u32 = 255;

pub const ERROR_RETURN: &str = "return";
pub const ERROR_FILE: &str = "file";
pub const ERROR_VIEW: &str = "view";
pub const ERROR_UNDEF: &str = "undef";
pub const ERROR_PERL: &str = "perl";
pub const ERROR_FILTER: &str = "filter";
pub const ERROR_PLUGIN: &str = "plugin";

pub const CHOMP_NONE: u32 = 0;
pub const CHOMP_ONE: u32 = 1;
pub const CHOMP_ALL: u32 = 1;
pub const CHOMP_COLLAPSE: u32 = 2;
pub const CHOMP_GREEDY: u32 = 3;

pub const DEBUG_OFF: u32 = 0;
pub const DEBUG_ON: u32 = 1;
pub const DEBUG_UNDEF: u32 = 2;
pub const DEBUG_VARS: u32 = 4;
pub const DEBUG_DIRS: u32 = 8;
pub const DEBUG_STASH: u32 = 16;
pub const DEBUG_CONTEXT: u32 = 32;
pub const DEBUG_PARSER: u32 = 64;
pub const DEBUG_PROVIDER: u32 = 128;
pub const DEBUG_PLUGINS: u32 = 256;
pub const DEBUG_FILTERS: u32 = 512;
pub const DEBUG_SERVICE: u32 = 1024;
pub const DEBUG_ALL: u32 = 2047;
pub const DEBUG_CALLER: u32 = 4096;
pub const DEBUG_FLAGS: u32 = 4096;

pub const DEBUG_OPTIONS: &[(&str, u32)] = &[
    ("off", DEBUG_OFF),
    ("on", DEBUG_ON),
    ("undef", DEBUG_UNDEF),
    ("vars", DEBUG_VARS),
    ("dirs", DEBUG_DIRS),
    ("stash", DEBUG_STASH),
    ("context", DEBUG_CONTEXT),
    ("parser", DEBUG_PARSER),
    ("provider", DEBUG_PROVIDER),
    ("plugins", DEBUG_PLUGINS),
    ("filters", DEBUG_FILTERS),
    ("service", DEBUG_SERVICE),
    ("all", DEBUG_ALL),
    ("caller", DEBUG_CALLER),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugFlagsInput {
    Number(u32),
    Text(String),
}

impl From<u32> for DebugFlagsInput {
    fn from(n: u32) -> Self {
        DebugFlagsInput::Number(n)
    }
}

impl From<&str> for DebugFlagsInput {
    fn from(s: &str) -> Self {
        if let Ok(n) = s.trim().parse::<u32>() {
            DebugFlagsInput::Number(n)
        } else {
            DebugFlagsInput::Text(s.to_string())
        }
    }
}

impl From<String> for DebugFlagsInput {
    fn from(s: String) -> Self {
        DebugFlagsInput::from(s.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugFlagsOutput {
    Number(u32),
    Text(String),
}

impl DebugFlagsOutput {
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            DebugFlagsOutput::Number(n) => Some(*n),
            DebugFlagsOutput::Text(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            DebugFlagsOutput::Text(s) => Some(s.as_str()),
            DebugFlagsOutput::Number(_) => None,
        }
    }
}

pub fn debug_flags<I: Into<DebugFlagsInput>>(input: I) -> Result<DebugFlagsOutput, String> {
    let map: HashMap<&str, u32> = DEBUG_OPTIONS.iter().copied().collect();
    match input.into() {
        DebugFlagsInput::Number(mut num) => {
            let mut flags = Vec::new();
            for &(name, flag) in DEBUG_OPTIONS {
                // Don't report 'on' or 'all' when converting numeric mask back to string
                if name == "off" || name == "on" || name == "all" {
                    continue;
                }
                if (num & flag) != 0 {
                    num &= !flag;
                    flags.push(name);
                }
            }
            Ok(DebugFlagsOutput::Text(flags.join(", ")))
        }
        DebugFlagsInput::Text(text) => {
            let mut result = 0u32;
            for part in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if part.is_empty() {
                    continue;
                }
                let lower = part.to_ascii_lowercase();
                if let Some(&val) = map.get(lower.as_str()) {
                    result |= val;
                } else {
                    return Err(format!("unknown debug flag: {}", part));
                }
            }
            Ok(DebugFlagsOutput::Number(result))
        }
    }
}
