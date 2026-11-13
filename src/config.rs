use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Tiling,
    Floating,
}

impl fmt::Display for LayoutMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tiling => f.write_str("tiling"),
            Self::Floating => f.write_str("floating"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub mode: LayoutMode,
    pub animations: bool,
    pub animation_duration_ms: u32,
    pub inner_gap: u32,
    pub outer_gap: u32,
    pub corner_radius: u32,
    pub shadows: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: LayoutMode::Tiling,
            animations: true,
            animation_duration_ms: 200,
            inner_gap: 5,
            outer_gap: 5,
            corner_radius: 8,
            shadows: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub line: Option<usize>,
    pub message: String,
}

impl ConfigError {
    fn at_line(line: usize, message: impl Into<String>) -> Self {
        Self {
            line: Some(line),
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "line {line}: {}", self.message),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Parse Alpinium's initial command-style configuration syntax.
    ///
    /// Supported directives are intentionally small in this first milestone.
    /// Unknown directives fail loudly rather than being silently ignored.
    pub fn parse(input: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default();

        for (index, raw_line) in input.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let fields: Vec<&str> = line.split_whitespace().collect();
            match fields.as_slice() {
                ["mode", value] => {
                    config.mode = match *value {
                        "tiling" => LayoutMode::Tiling,
                        "floating" => LayoutMode::Floating,
                        _ => {
                            return Err(ConfigError::at_line(
                                line_number,
                                format!("unsupported mode '{value}'; expected 'tiling' or 'floating'"),
                            ));
                        }
                    };
                }
                ["animations", value] => {
                    config.animations = parse_enabled(value).ok_or_else(|| {
                        ConfigError::at_line(
                            line_number,
                            "expected 'enabled' or 'disabled' for animations",
                        )
                    })?;
                }
                ["animation_duration", value] => {
                    let duration = value.strip_suffix("ms").ok_or_else(|| {
                        ConfigError::at_line(
                            line_number,
                            "animation_duration must use milliseconds, for example 200ms",
                        )
                    })?;
                    config.animation_duration_ms =
                        parse_u32(duration, line_number, "animation duration")?;
                }
                ["gaps", "inner", value] => {
                    config.inner_gap = parse_u32(value, line_number, "inner gap")?;
                }
                ["gaps", "outer", value] => {
                    config.outer_gap = parse_u32(value, line_number, "outer gap")?;
                }
                ["corner_radius", value] => {
                    config.corner_radius = parse_u32(value, line_number, "corner radius")?;
                }
                ["shadows", value] => {
                    config.shadows = parse_enabled(value).ok_or_else(|| {
                        ConfigError::at_line(
                            line_number,
                            "expected 'enabled' or 'disabled' for shadows",
                        )
                    })?;
                }
                [directive, ..] => {
                    return Err(ConfigError::at_line(
                        line_number,
                        format!("unknown or malformed directive '{directive}'"),
                    ));
                }
                [] => {}
            }
        }

        Ok(config)
    }

    pub fn load(explicit_path: Option<&Path>) -> Result<(Self, Option<PathBuf>), String> {
        let path = if let Some(path) = explicit_path {
            if !path.is_file() {
                return Err(format!("configuration file does not exist: {}", path.display()));
            }
            Some(path.to_path_buf())
        } else {
            find_default_config()
        };

        let Some(path) = path else {
            return Ok((Self::default(), None));
        };

        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let config = Self::parse(&contents)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        Ok((config, Some(path)))
    }
}

fn parse_enabled(value: &str) -> Option<bool> {
    match value {
        "enabled" => Some(true),
        "disabled" => Some(false),
        _ => None,
    }
}

fn parse_u32(value: &str, line: usize, label: &str) -> Result<u32, ConfigError> {
    value
        .parse::<u32>()
        .map_err(|_| ConfigError::at_line(line, format!("{label} must be a non-negative integer")))
}

fn config_candidates(
    xdg_config_home: Option<&std::ffi::OsStr>,
    home: Option<&std::ffi::OsStr>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(xdg_config_home) = xdg_config_home {
        let xdg_config = PathBuf::from(xdg_config_home)
            .join("alpinium")
            .join("config");
        if xdg_config.is_absolute() {
            candidates.push(xdg_config);
        }
    }

    if let Some(home) = home {
        candidates.push(
            PathBuf::from(home)
                .join(".config")
                .join("alpinium")
                .join("config"),
        );
    }

    candidates.push(PathBuf::from("/etc/alpinium/config"));
    candidates
}

fn find_default_config() -> Option<PathBuf> {
    let xdg_config_home = std::env::var_os("XDG_CONFIG_HOME");
    let home = std::env::var_os("HOME");

    config_candidates(xdg_config_home.as_deref(), home.as_deref())
        .into_iter()
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_tiling_and_expected_visual_defaults() {
        let config = Config::parse("").expect("empty config should use defaults");
        assert_eq!(config, Config::default());
        assert_eq!(config.mode, LayoutMode::Tiling);
    }

    #[test]
    fn parses_floating_mode_and_visual_settings() {
        let config = Config::parse(
            "mode floating\nanimations disabled\nanimation_duration 350ms\ngaps inner 7\ngaps outer 11\ncorner_radius 12\nshadows disabled\n",
        )
        .expect("configuration should parse");

        assert_eq!(config.mode, LayoutMode::Floating);
        assert!(!config.animations);
        assert_eq!(config.animation_duration_ms, 350);
        assert_eq!(config.inner_gap, 7);
        assert_eq!(config.outer_gap, 11);
        assert_eq!(config.corner_radius, 12);
        assert!(!config.shadows);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let config = Config::parse("# comment\n\nmode floating\n").unwrap();
        assert_eq!(config.mode, LayoutMode::Floating);
    }

    #[test]
    fn rejects_unknown_mode() {
        let error = Config::parse("mode hybrid").unwrap_err();
        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("tiling"));
    }

    #[test]
    fn rejects_unknown_directives() {
        let error = Config::parse("modes tiling").unwrap_err();
        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("unknown"));
    }

    #[test]
    fn rejects_invalid_duration_unit() {
        let error = Config::parse("animation_duration 200s").unwrap_err();
        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("milliseconds"));
    }

    #[test]
    fn config_lookup_checks_xdg_then_home_then_system() {
        use std::ffi::OsStr;

        let candidates = config_candidates(
            Some(OsStr::new("/tmp/alpinium-xdg")),
            Some(OsStr::new("/home/tester")),
        );

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/tmp/alpinium-xdg/alpinium/config"),
                PathBuf::from("/home/tester/.config/alpinium/config"),
                PathBuf::from("/etc/alpinium/config"),
            ]
        );
    }

    #[test]
    fn home_config_is_checked_when_xdg_config_home_is_missing() {
        use std::ffi::OsStr;

        let candidates = config_candidates(None, Some(OsStr::new("/home/tester")));

        assert_eq!(
            candidates,
            vec![
                PathBuf::from("/home/tester/.config/alpinium/config"),
                PathBuf::from("/etc/alpinium/config"),
            ]
        );
    }

    #[test]
    fn rejects_negative_gaps() {
        let error = Config::parse("gaps inner -1").unwrap_err();
        assert_eq!(error.line, Some(1));
    }
}
