//! Configurable keyboard bindings and their runtime actions.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub logo: bool,
}

impl Modifiers {
    fn parse(name: &str) -> Result<Self, String> {
        let mut modifiers = Self::default();
        for token in name.split('+') {
            let normalized = token.to_ascii_lowercase();
            let target = match normalized.as_str() {
                "ctrl" | "control" => &mut modifiers.ctrl,
                "alt" | "mod1" => &mut modifiers.alt,
                "shift" => &mut modifiers.shift,
                "super" | "logo" | "mod4" => &mut modifiers.logo,
                other => return Err(format!("unsupported modifier '{other}'; use Ctrl, Alt/Mod1, Shift, or Super/Mod4")),
            };
            if *target {
                return Err(format!("modifier '{token}' was specified more than once"));
            }
            *target = true;
        }
        Ok(modifiers)
    }

    pub fn matches(self, ctrl: bool, alt: bool, shift: bool, logo: bool) -> bool {
        self.ctrl == ctrl && self.alt == alt && self.shift == shift && self.logo == logo
    }

    fn format(self) -> Vec<&'static str> {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl"); }
        if self.alt { parts.push("Mod1"); }
        if self.shift { parts.push("Shift"); }
        if self.logo { parts.push("Mod4"); }
        parts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingAction {
    Execute(String),
    FocusNext,
    FocusPrevious,
    CloseFocused,
    ExitCompositor,
}

impl BindingAction {
    pub fn parse(command: &str) -> Result<Self, String> {
        let command = command.trim();
        let (verb, rest) = command
            .split_once(char::is_whitespace)
            .map(|(verb, rest)| (verb, rest.trim_start()))
            .unwrap_or((command, ""));

        match verb {
            "exec" if !rest.is_empty() => Ok(Self::Execute(rest.to_string())),
            "exec" => Err("exec requires a command".to_string()),
            "focus" if rest == "next" => Ok(Self::FocusNext),
            "focus" if rest == "prev" || rest == "previous" => Ok(Self::FocusPrevious),
            "focus" => Err("focus expects 'next' or 'prev'".to_string()),
            "kill" | "close" if rest.is_empty() => Ok(Self::CloseFocused),
            "exit" if rest.is_empty() => Ok(Self::ExitCompositor),
            "kill" | "close" | "exit" => Err(format!("unexpected arguments for '{verb}'")),
            "" => Err("missing binding action".to_string()),
            other => Err(format!("unknown binding action '{other}'")),
        }
    }

    pub fn as_config(&self) -> String {
        match self {
            Self::Execute(command) => format!("exec {command}"),
            Self::FocusNext => "focus next".to_string(),
            Self::FocusPrevious => "focus prev".to_string(),
            Self::CloseFocused => "kill".to_string(),
            Self::ExitCompositor => "exit".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub modifiers: Modifiers,
    pub key: String,
    pub action: BindingAction,
}

impl Binding {
    pub fn parse(combination: &str, command: &str) -> Result<Self, String> {
        let parts: Vec<&str> = combination.split('+').collect();
        let Some(key) = parts.last().copied() else {
            return Err("binding is missing a key".to_string());
        };
        if key.is_empty() || key.chars().any(char::is_whitespace) {
            return Err("binding has an invalid or empty key name".to_string());
        }

        let modifier_text = parts[..parts.len().saturating_sub(1)].join("+");
        let modifiers = if modifier_text.is_empty() {
            Modifiers::default()
        } else {
            Modifiers::parse(&modifier_text)?
        };

        Ok(Self {
            modifiers,
            key: key.to_string(),
            action: BindingAction::parse(command)?,
        })
    }

    pub fn matches(
        &self,
        ctrl: bool,
        alt: bool,
        shift: bool,
        logo: bool,
        key_name: &str,
    ) -> bool {
        self.modifiers.matches(ctrl, alt, shift, logo)
            && self.key.eq_ignore_ascii_case(key_name)
    }

    pub(crate) fn same_trigger_for_config(&self, other: &Self) -> bool {
        self.modifiers == other.modifiers && self.key.eq_ignore_ascii_case(&other.key)
    }

    pub fn as_config(&self) -> String {
        let mut parts: Vec<String> = self.modifiers.format().into_iter().map(str::to_string).collect();
        parts.push(self.key.clone());
        format!("bindsym {} {}", parts.join("+"), self.action.as_config())
    }
}

pub fn default_bindings(modifier: &str) -> Vec<Binding> {
    [
        (format!("{modifier}+Return"), "exec alacritty"),
        (format!("{modifier}+Tab"), "focus next"),
        (format!("{modifier}+Shift+Tab"), "focus prev"),
        (format!("{modifier}+q"), "kill"),
        (format!("{modifier}+Shift+e"), "exit"),
    ]
    .into_iter()
    .map(|(combination, action)| {
        Binding::parse(&combination, action).expect("built-in keyboard binding must be valid")
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sway_style_modifiers_and_exec_action() {
        let binding = Binding::parse("Mod4+Shift+Return", "exec alacritty --option value").unwrap();
        assert!(binding.modifiers.logo);
        assert!(binding.modifiers.shift);
        assert_eq!(binding.key, "Return");
        assert_eq!(
            binding.action,
            BindingAction::Execute("alacritty --option value".to_string())
        );
    }

    #[test]
    fn accepts_friendly_modifier_aliases() {
        let binding = Binding::parse("Super+Alt+Ctrl+x", "focus next").unwrap();
        assert!(binding.modifiers.logo);
        assert!(binding.modifiers.alt);
        assert!(binding.modifiers.ctrl);
    }

    #[test]
    fn matches_key_names_without_case_sensitivity() {
        let binding = Binding::parse("Mod4+q", "kill").unwrap();
        assert!(binding.matches(false, false, false, true, "Q"));
        assert!(!binding.matches(false, false, true, true, "q"));
    }

    #[test]
    fn rejects_unknown_modifiers() {
        let error = Binding::parse("Mod3+q", "kill").unwrap_err();
        assert!(error.contains("unsupported modifier"));
    }

    #[test]
    fn rejects_duplicate_modifiers() {
        let error = Binding::parse("Mod4+Super+q", "kill").unwrap_err();
        assert!(error.contains("more than once"));
    }

    #[test]
    fn rejects_unknown_actions() {
        let error = Binding::parse("Mod4+q", "restart").unwrap_err();
        assert!(error.contains("unknown binding action"));
    }

    #[test]
    fn rejects_exec_without_a_command() {
        assert!(Binding::parse("Mod4+Return", "exec").is_err());
    }

    #[test]
    fn built_in_bindings_use_the_selected_modifier() {
        let bindings = default_bindings("Mod1");
        assert_eq!(bindings.len(), 5);
        assert!(bindings.iter().all(|binding| binding.modifiers.alt));
        assert!(bindings.iter().all(|binding| !binding.modifiers.logo));
    }
}
