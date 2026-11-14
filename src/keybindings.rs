//! Default compositor keyboard shortcuts.
//!
//! Key symbols are converted by the Smithay input layer; this module keeps
//! shortcut policy small and independently testable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKey {
    Logo,
    Tab,
    Q,
    E,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Consume the modifier event so it is not sent to application clients.
    Consume,
    FocusNext,
    FocusPrevious,
    CloseFocused,
    ExitCompositor,
}

/// Resolve an Alpinium shortcut.
///
/// The Logo modifier is the Super/Windows key. Ordinary keys are forwarded
/// to the focused client; only the shortcuts listed here are intercepted.
pub fn resolve(
    logo: bool,
    shift: bool,
    pressed: bool,
    key: ShortcutKey,
) -> Option<Action> {
    if key == ShortcutKey::Logo {
        return Some(Action::Consume);
    }

    if !pressed || !logo {
        return None;
    }

    match key {
        ShortcutKey::Tab if shift => Some(Action::FocusPrevious),
        ShortcutKey::Tab => Some(Action::FocusNext),
        ShortcutKey::Q => Some(Action::CloseFocused),
        ShortcutKey::E if shift => Some(Action::ExitCompositor),
        ShortcutKey::Logo | ShortcutKey::E | ShortcutKey::Other => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn super_tab_focuses_next_window() {
        assert_eq!(resolve(true, false, true, ShortcutKey::Tab), Some(Action::FocusNext));
    }

    #[test]
    fn super_shift_tab_focuses_previous_window() {
        assert_eq!(resolve(true, true, true, ShortcutKey::Tab), Some(Action::FocusPrevious));
    }

    #[test]
    fn super_q_requests_close_for_the_focused_window() {
        assert_eq!(resolve(true, false, true, ShortcutKey::Q), Some(Action::CloseFocused));
    }

    #[test]
    fn super_shift_e_exits_the_compositor() {
        assert_eq!(resolve(true, true, true, ShortcutKey::E), Some(Action::ExitCompositor));
    }

    #[test]
    fn super_e_without_shift_is_forwarded() {
        assert_eq!(resolve(true, false, true, ShortcutKey::E), None);
    }

    #[test]
    fn super_shift_e_key_release_is_forwarded() {
        assert_eq!(resolve(true, true, false, ShortcutKey::E), None);
    }

    #[test]
    fn logo_modifier_is_consumed_on_press_and_release() {
        assert_eq!(resolve(true, false, true, ShortcutKey::Logo), Some(Action::Consume));
        assert_eq!(resolve(false, false, false, ShortcutKey::Logo), Some(Action::Consume));
    }

    #[test]
    fn shortcuts_without_super_are_forwarded() {
        assert_eq!(resolve(false, false, true, ShortcutKey::Tab), None);
        assert_eq!(resolve(false, false, true, ShortcutKey::Q), None);
    }

    #[test]
    fn normal_keys_and_key_releases_are_forwarded() {
        assert_eq!(resolve(true, false, true, ShortcutKey::Other), None);
        assert_eq!(resolve(true, false, false, ShortcutKey::Tab), None);
    }
}
