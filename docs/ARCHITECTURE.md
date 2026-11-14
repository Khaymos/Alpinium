# Alpinium architecture (initial design)

## Scope

Alpinium is one Wayland compositor with two selectable global layout modes:

- **Tiling:** the compositor arranges managed windows according to the active tiling policy.
- **Floating:** windows are positioned and resized freely, with focus and stacking handled by the compositor.

The selected mode is loaded from the mode directive in the config. The same rendering, input, protocol, configuration, and effects infrastructure is shared by both modes.

## Current implementation

The codebase now has:
- A configuration model and parser.
- Configuration-file discovery and validation.
- An initial Smithay-based nested backend using the Winit host window.
- A Wayland display/socket and basic XDG shell, SHM, seat, output and data-device protocol state.
- Rendering of the nested output and forwarding of host input events.

The nested backend now includes a first working master-stack layout policy. The first mapped window occupies the left master area; remaining windows are stacked vertically on the right. Inner and outer gaps are applied from configuration. Floating mode leaves placement to pointer move/resize grabs, while tiling mode ignores those client requests.

The layout calculation is isolated in `src/layout.rs` and has unit tests independent of Smithay. The nested backend reapplies layout after window lifecycle changes and output redraw/resize events. This remains an early milestone and has not been verified on every target system.

The initial keyboard shortcut layer is isolated in `src/keybindings.rs`. Smithay's keyboard filter intercepts Super+Tab and Super+Shift+Tab for focus cycling, Super+Q to send an XDG toplevel close request, and Super+Shift+E to stop the compositor event loop cleanly. In the nested development backend, this closes Alpinium and returns to the host desktop; it does not log out the host session or forcibly terminate unrelated client processes. Other key events continue to the focused client. The last-focused window is retained while the host Winit window is unfocused.

A `--native-probe` diagnostic uses libseat and udev to open DRM devices and enumerate connected connectors/modes without modesetting. It validates session/device discovery before the native output backend is enabled.

A separate `--native-renderer-probe` diagnostic opens the first DRM device through libseat, initializes GBM, EGL, and the GLES renderer, and reports the renderer capabilities. It intentionally stops before creating a DRM output or committing any KMS state, allowing GPU-driver compatibility to be checked before display takeover.

The first native compositor runtime reuses the same Wayland state and input routing with libseat, libinput, GBM, EGL, GLES and Smithay DRM output management. It is deliberately limited to the verified internal eDP-1 connector on one DRM card; it handles VT pause/resume through libseat and supports launching a Wayland client with `--native --command PROGRAM`. External displays, connector hotplug, and multi-GPU support remain future work.\n\nThe nested backend also tracks XDG toplevel buffer transitions. A buffer detach removes the window from the visible space and triggers focus recovery when necessary; a subsequent bufferless commit completes the XDG initial-configure handshake before remapping. Remapped windows retain their prior position in floating mode. Destroyed toplevels are removed from compositor tracking, including the stable creation-order list. The pure buffer-transition classifier is covered by unit tests in `src/window_lifecycle.rs`. A standalone client in `examples/xdg_lifecycle_probe.rs` exercises the actual configure, detach, leave, remap, and enter sequence; see `docs/LIFECYCLE_TESTING.md` for instructions.

Pointer input is routed to the surface beneath the cursor for both absolute and relative motion; relative motion is accumulated and clamped to the active output. Button presses implement click-to-focus: managed windows are raised and receive keyboard focus, clicks on empty space clear focus, and active move/resize grabs retain their existing focus. Focus changes update the XDG toplevel `activated` state, including host-window focus loss and restoration. The click-focus policy is covered by unit tests in `src/pointer_focus.rs`.

## Planned layers

~~~text
Alpinium process
├── Configuration
│   ├── file discovery and precedence
│   ├── parser and validation
│   └── runtime settings
├── Wayland backend (Smithay)
│   ├── nested Winit backend (initial)
│   ├── single-output DRM/session backend (initial eDP-1 support)
│   ├── input and output management
│   └── Wayland protocol globals
├── Compositor core
│   ├── clients and surfaces
│   ├── focus and seat state
│   ├── output/workspace state
│   └── window lifecycle
├── Layout engine
│   ├── master-stack tiling policy (initial)
│   └── floating move/resize policy (initial)
├── Visual system
│   ├── decorations and borders
│   ├── animations and transitions
│   ├── shadows and rounded corners
│   └── opacity and blur (where supported)
└── IPC and integration (planned)
    ├── control socket / command client
    ├── session startup
    └── desktop portal and XWayland integration
~~~

## Design constraints

- Keep tiling and floating policies independent from the Wayland backend.
- Keep visual effects shared between both layout modes.
- Make expensive effects individually disableable.
- Keep configuration parsing separate from runtime state.
- Treat missing, malformed, or unsupported configuration as a clear diagnostic, not a silent fallback.
- Avoid requiring systemd; KhaymOS uses OpenRC.

## Development strategy

Develop and test the compositor nested inside an existing Wayland session first. Direct DRM/TTY session support should come after nested rendering, input, and window management work reliably.
