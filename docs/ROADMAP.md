# Roadmap

## Milestone 0 — project and configuration foundation

- [x] Rust project scaffold.
- [x] mode tiling / mode floating configuration model.
- [x] User/system configuration lookup.
- [x] Configuration validation CLI.
- [x] Unit tests for valid and invalid directives.

## Milestone 1 — nested Wayland backend

- [x] Integrate Smithay's Winit backend and Wayland frontend.
- [x] Create a nested Wayland display/socket and output.
- [x] Add initial XDG shell, SHM, seat, and output protocol state.
- [x] Verify nested startup and render a real client on developer hardware.
- [ ] Harden client lifecycle and pointer handling.
- [x] Add initial keyboard focus cycling and close-request shortcuts.
- [x] Add a Super+Shift+E compositor exit shortcut.

## Milestone 2 — compositor basics

- [x] Initial XDG toplevel buffer map/unmap/remap tracking and destruction cleanup.
- [x] Recover keyboard focus when the focused toplevel unmaps or is destroyed.
- [x] Add a standalone XDG map/unmap/remap lifecycle probe.
- [x] Initial keyboard-driven focus and client close requests.
- [ ] Run the lifecycle probe against the nested compositor and harden unusual client/protocol sequences.
- [x] Route absolute and relative pointer motion to the surface beneath the cursor.
- [x] Implement click-to-focus, empty-space focus clearing, and active-grab focus preservation.
- [x] Keep XDG toplevel activation state synchronized with keyboard and host-window focus.
- [ ] Verify pointer enter/leave and click-to-focus behavior with multiple clients on the nested backend.
- [ ] Basic command/IPC interface.
- [ ] Output resize and hotplug handling.

## Milestone 3 — layout modes

- [x] Initial master-stack tiling layout engine with geometry unit tests.
- [x] Initial floating move/resize policy.
- [x] Apply the config-selected global mode to new and existing windows.
- [ ] Configurable master ratios, stack ordering, and additional layout variants.
- [ ] Workspaces and output management.

## Milestone 4 — visual system

- [ ] Decorations and configurable borders.
- [ ] Open/close and workspace animations.
- [ ] Rounded corners and shadows.
- [ ] Per-effect performance controls.
- [ ] Opacity and blur where supported by the renderer.

## Milestone 5 — KhaymOS integration

- [ ] KBuild recipe and runtime dependency list.
- [ ] OpenRC session integration.
- [ ] Default config installed to /etc/alpinium/config.
- [ ] Installer desktop choice integration.
- [ ] Hardware testing on KhaymOS target machines.
