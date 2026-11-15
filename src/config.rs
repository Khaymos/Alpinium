use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::animation::AnimationCurve;
use crate::keybindings::{Binding, default_bindings};

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
    pub animation_curve: AnimationCurve,
    pub window_move_animation: bool,
    pub window_resize_animation: bool,
    pub inner_gap: u32,
    pub outer_gap: u32,
    pub corner_radius: u32,
    pub shadows: bool,
    pub variables: BTreeMap<String, String>,
    pub bindings: Vec<Binding>,
}

impl Default for Config {
    fn default() -> Self {
        let variables = BTreeMap::from([
            ("mod".to_string(), "Mod4".to_string()),
            ("terminal".to_string(), "alacritty".to_string()),
        ]);
        Self {
            mode: LayoutMode::Tiling,
            animations: true,
            animation_duration_ms: 200,
            animation_curve: AnimationCurve::EaseOut,
            window_move_animation: true,
            window_resize_animation: true,
            inner_gap: 5,
            outer_gap: 5,
            corner_radius: 8,
            shadows: true,
            bindings: default_bindings("Mod4"),
            variables,
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
    /// Parse directives in order. Variables can reference variables declared earlier.
    pub fn parse(input: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let mut variables = config.variables.clone();
        let mut explicit_bindings = false;
        let mut parsed_bindings: Vec<Binding> = Vec::new();

        for (index, raw_line) in input.lines().enumerate() {
            let line_number = index + 1;
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with("set ") || line.starts_with("set\t") {
                let rest = line[3..].trim_start();
                let mut fields = rest.splitn(2, char::is_whitespace);
                let name = fields.next().unwrap_or_default();
                let value = fields.next().unwrap_or_default().trim();
                let variable = name.strip_prefix('$').ok_or_else(|| {
                    ConfigError::at_line(
                        line_number,
                        "set expects a variable name beginning with '$'",
                    )
                })?;
                if !valid_variable_name(variable) {
                    return Err(ConfigError::at_line(
                        line_number,
                        format!("invalid variable name '{name}'"),
                    ));
                }
                if value.is_empty() {
                    return Err(ConfigError::at_line(
                        line_number,
                        "set requires a variable value",
                    ));
                }
                let value = expand_variables(value, &variables, line_number)?;
                variables.insert(variable.to_string(), value);
                continue;
            }

            let expanded = expand_variables(line, &variables, line_number)?;

            if let Some(rest) = expanded.strip_prefix("bindsym ") {
                let mut fields = rest.splitn(2, char::is_whitespace);
                let combination = fields.next().unwrap_or_default();
                let command = fields.next().unwrap_or_default().trim_start();
                if combination.is_empty() || command.is_empty() {
                    return Err(ConfigError::at_line(
                        line_number,
                        "bindsym requires a key combination and an action",
                    ));
                }

                let binding = Binding::parse(combination, command)
                    .map_err(|message| ConfigError::at_line(line_number, message))?;
                if !explicit_bindings {
                    parsed_bindings.clear();
                    explicit_bindings = true;
                }
                if parsed_bindings
                    .iter()
                    .any(|existing| existing.same_trigger_for_config(&binding))
                {
                    return Err(ConfigError::at_line(
                        line_number,
                        format!("duplicate keybinding for '{combination}'"),
                    ));
                }
                parsed_bindings.push(binding);
                continue;
            }

            let fields: Vec<&str> = expanded.split_whitespace().collect();
            match fields.as_slice() {
                ["mode", value] => {
                    config.mode = match *value {
                        "tiling" => LayoutMode::Tiling,
                        "floating" => LayoutMode::Floating,
                        _ => {
                            return Err(ConfigError::at_line(
                                line_number,
                                format!(
                                    "unsupported mode '{value}'; expected 'tiling' or 'floating'"
                                ),
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
                ["animation_curve", value] => {
                    config.animation_curve = AnimationCurve::parse(value).ok_or_else(|| {
                        ConfigError::at_line(
                            line_number,
                            "animation_curve must be linear, ease-in, ease-out, or ease-in-out",
                        )
                    })?;
                }
                ["window_move_animation", value] => {
                    config.window_move_animation = parse_enabled(value).ok_or_else(|| {
                        ConfigError::at_line(
                            line_number,
                            "expected 'enabled' or 'disabled' for window_move_animation",
                        )
                    })?;
                }
                ["window_resize_animation", value] => {
                    config.window_resize_animation = parse_enabled(value).ok_or_else(|| {
                        ConfigError::at_line(
                            line_number,
                            "expected 'enabled' or 'disabled' for window_resize_animation",
                        )
                    })?;
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

        let default_modifier = variables.get("mod").map(String::as_str).unwrap_or("Mod4");
        config.bindings = if explicit_bindings {
            parsed_bindings
        } else {
            default_bindings(default_modifier)
        };
        config.variables = variables;
        Ok(config)
    }

    pub fn load(explicit_path: Option<&Path>) -> Result<(Self, Option<PathBuf>), String> {
        let path = if let Some(path) = explicit_path {
            if !path.is_file() {
                return Err(format!(
                    "configuration file does not exist: {}",
                    path.display()
                ));
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
        let config =
            Self::parse(&contents).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok((config, Some(path)))
    }
}

fn valid_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn expand_variables(
    source: &str,
    variables: &BTreeMap<String, String>,
    line: usize,
) -> Result<String, ConfigError> {
    let chars: Vec<char> = source.chars().collect();
    let mut output = String::new();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] != '$' {
            output.push(chars[index]);
            index += 1;
            continue;
        }

        index += 1;
        if index < chars.len() && chars[index] == '$' {
            output.push('$');
            index += 1;
            continue;
        }

        let name = if index < chars.len() && chars[index] == '{' {
            index += 1;
            let start = index;
            while index < chars.len() && chars[index] != '}' {
                index += 1;
            }
            if index == chars.len() {
                return Err(ConfigError::at_line(
                    line,
                    "unterminated variable reference",
                ));
            }
            let name: String = chars[start..index].iter().collect();
            index += 1;
            name
        } else {
            let start = index;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            chars[start..index].iter().collect()
        };

        if !valid_variable_name(&name) {
            return Err(ConfigError::at_line(
                line,
                "invalid or empty variable reference",
            ));
        }
        let value = variables
            .get(&name)
            .ok_or_else(|| ConfigError::at_line(line, format!("undefined variable '${name}'")))?;
        output.push_str(value);
    }

    Ok(output)
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
    use crate::keybindings::BindingAction;

    #[test]
    fn defaults_to_tiling_and_expected_visual_defaults() {
        let config = Config::parse("").expect("empty config should use defaults");
        assert_eq!(config, Config::default());
        assert_eq!(config.mode, LayoutMode::Tiling);
        assert_eq!(config.bindings.len(), 6);
    }

    #[test]
    fn parses_floating_mode_and_visual_settings() {
        let config = Config::parse(
            "mode floating\nanimations disabled\nanimation_duration 350ms\ngaps inner 7\ngaps outer 11\ncorner_radius 12\nshadows disabled\n",
        ).expect("configuration should parse");
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
    fn parses_animation_curve_and_individual_geometry_toggles() {
        let config = Config::parse(
            "animation_curve ease-in-out\nwindow_move_animation disabled\nwindow_resize_animation enabled\n",
        ).unwrap();
        assert_eq!(config.animation_curve, AnimationCurve::EaseInOut);
        assert!(!config.window_move_animation);
        assert!(config.window_resize_animation);
    }

    #[test]
    fn rejects_unknown_animation_curve() {
        let error = Config::parse("animation_curve bounce").unwrap_err();
        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("ease-in-out"));
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

    #[test]
    fn expands_variables_in_binding_combinations_and_commands() {
        let config = Config::parse(
            "set $mod Mod1\nset $terminal alacritty\nbindsym $mod+Return exec $terminal --option value\n",
        ).unwrap();
        let binding = &config.bindings[0];
        assert!(binding.modifiers.alt);
        assert_eq!(binding.key, "Return");
        assert_eq!(
            binding.action,
            BindingAction::Execute("alacritty --option value".to_string())
        );
        assert_eq!(config.variables["mod"], "Mod1");
    }

    #[test]
    fn variables_can_reference_earlier_variables() {
        let config =
            Config::parse("set $base Mod4\nset $mod $base\nbindsym $mod+Return exec alacritty\n")
                .unwrap();
        assert!(config.bindings[0].modifiers.logo);
    }

    #[test]
    fn undefined_variables_report_the_line_number() {
        let error = Config::parse("bindsym $missing+Return exec alacritty").unwrap_err();
        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("undefined variable"));
    }

    #[test]
    fn unknown_binding_actions_are_rejected() {
        let error = Config::parse("bindsym Mod4+q restart").unwrap_err();
        assert_eq!(error.line, Some(1));
        assert!(error.message.contains("unknown binding action"));
    }

    #[test]
    fn custom_bindings_replace_the_builtin_set() {
        let config = Config::parse("bindsym Mod1+Return exec alacritty\n").unwrap();
        assert_eq!(config.bindings.len(), 1);
        assert!(config.bindings[0].modifiers.alt);
    }

    #[test]
    fn changing_mod_variable_changes_default_bindings() {
        let config = Config::parse("set $mod Mod1\n").unwrap();
        assert!(config.bindings.iter().all(|binding| binding.modifiers.alt));
    }
}
