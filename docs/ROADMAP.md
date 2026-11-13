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
- [ ] Verify nested startup and render a real client on developer hardware.
- [ ] Harden client lifecycle, focus, pointer, and keyboard handling.

## Milestone 2 — compositor basics

- [ ] Surface map/unmap lifecycle and robust focus policy.
- [ ] Move, resize, focus, and close requests.
- [ ] Basic command/IPC interface.
- [ ] Output resize and hotplug handling.

## Milestone 3 — layout modes

- [ ] Tiling layout engine.
- [ ] Floating layout engine.
- [ ] Apply the config-selected global mode to new and existing windows.
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
