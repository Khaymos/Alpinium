//! Policy for keyboard focus changes initiated by pointer button presses.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickFocusAction {
    /// The click targets a mapped toplevel and should focus it.
    FocusWindow,
    /// The click lands outside managed toplevels and should clear focus.
    ClearFocus,
    /// A release or active pointer grab must not change keyboard focus.
    KeepCurrent,
}

/// Decide whether a pointer button event should change keyboard focus.
pub fn click_focus_action(
    button_pressed: bool,
    pointer_grabbed: bool,
    pointer_over_window: bool,
) -> ClickFocusAction {
    if !button_pressed || pointer_grabbed {
        ClickFocusAction::KeepCurrent
    } else if pointer_over_window {
        ClickFocusAction::FocusWindow
    } else {
        ClickFocusAction::ClearFocus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_over_window_requests_focus() {
        assert_eq!(click_focus_action(true, false, true), ClickFocusAction::FocusWindow);
    }

    #[test]
    fn press_on_empty_space_clears_focus() {
        assert_eq!(click_focus_action(true, false, false), ClickFocusAction::ClearFocus);
    }

    #[test]
    fn button_release_preserves_focus() {
        assert_eq!(click_focus_action(false, false, true), ClickFocusAction::KeepCurrent);
    }

    #[test]
    fn active_pointer_grab_preserves_focus() {
        assert_eq!(click_focus_action(true, true, true), ClickFocusAction::KeepCurrent);
        assert_eq!(click_focus_action(true, true, false), ClickFocusAction::KeepCurrent);
    }
}
