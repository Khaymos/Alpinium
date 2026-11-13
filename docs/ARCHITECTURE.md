# Alpinium architecture (initial design)

## Scope

Alpinium is one Wayland compositor with two selectable global layout modes:

- **Tiling:** the compositor arranges managed windows according to the active tiling policy.
- **Floating:** windows are positioned and resized freely, with focus and stacking handled by the compositor.

The selected mode is loaded from **mode tiling** or **mode floating** in the config. The same rendering, input, protocol, configuration, and effects infrastructure is shared by both modes.

## Planned layers

~~~text
Alpinium process
├── Configuration
│   ├── file discovery and precedence
│   ├── parser and validation
│   └── runtime settings
├── Wayland backend (planned: Smithay)
│   ├── session and device management
│   ├── input events
│   ├── outputs and rendering
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

## Backend choice

The current plan is to use Smithay as the low-level Wayland compositor library. Smithay provides protocol and system-level building blocks; it does not provide Alpinium's window-management policy or drawing logic. Those remain Alpinium responsibilities.

The first backend milestone should run nested under an existing Wayland compositor for safer development. Direct DRM/TTY session support should come after nested rendering and input work reliably.

## Design constraints

- Keep tiling and floating policies independent from the Wayland backend.
- Keep visual effects shared between both layout modes.
- Make expensive effects individually disableable.
- Keep configuration parsing separate from runtime state.
- Treat missing, malformed, or unsupported configuration as a clear diagnostic, not a silent fallback.
- Avoid requiring systemd; KhaymOS uses OpenRC.

## Not implemented yet

This document is the target architecture, not a claim that those subsystems already exist. The current code implements the first config model/parser and a validation CLI only.
