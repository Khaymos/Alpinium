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

This backend is an early milestone. It has not yet been verified on every target system, and window-management policies remain rudimentary.

## Planned layers

~~~text
Alpinium process
├── Configuration
│   ├── file discovery and precedence
│   ├── parser and validation
│   └── runtime settings
├── Wayland backend (Smithay)
│   ├── nested Winit backend (initial)
│   ├── future DRM/session backend
│   ├── input and output management
│   └── Wayland protocol globals
├── Compositor core
│   ├── clients and surfaces
│   ├── focus and seat state
│   ├── output/workspace state
│   └── window lifecycle
├── Layout engine
│   ├── tiling policy
│   └── floating policy
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
