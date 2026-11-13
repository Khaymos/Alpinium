//! Pure helpers for tracking XDG toplevel buffer lifecycle transitions.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferTransition {
    Unchanged,
    Attached,
    Detached,
}

/// Classify whether a toplevel's renderer buffer attachment changed.
pub fn buffer_transition(was_attached: bool, has_buffer: bool) -> BufferTransition {
    match (was_attached, has_buffer) {
        (false, true) => BufferTransition::Attached,
        (true, false) => BufferTransition::Detached,
        _ => BufferTransition::Unchanged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_bufferless_commit_is_not_an_unmap() {
        assert_eq!(buffer_transition(false, false), BufferTransition::Unchanged);
    }

    #[test]
    fn first_buffer_attachment_maps_the_toplevel() {
        assert_eq!(buffer_transition(false, true), BufferTransition::Attached);
    }

    #[test]
    fn removing_an_attached_buffer_unmaps_the_toplevel() {
        assert_eq!(buffer_transition(true, false), BufferTransition::Detached);
    }

    #[test]
    fn steady_buffer_states_do_not_trigger_lifecycle_changes() {
        assert_eq!(buffer_transition(true, true), BufferTransition::Unchanged);
    }
}
