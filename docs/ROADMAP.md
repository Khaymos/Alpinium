# Roadmap

## Milestone 0 — project and configuration foundation

- [x] Rust project scaffold.
- [x] mode tiling / mode floating configuration model.
- [x] User/system configuration lookup.
- [x] Configuration validation CLI.
- [x] Unit tests for valid and invalid directives.

## Milestone 1 — nested Wayland backend

- [ ] Select and pin the Smithay revision/features.
- [ ] Add a nested development backend.
- [ ] Start an Alpinium Wayland display.
- [ ] Render a basic background and output.
- [ ] Accept a simple Wayland client surface.

## Milestone 2 — compositor basics

- [ ] Input devices and keyboard focus.
- [ ] Surface map/unmap lifecycle.
- [ ] Move, resize, focus, and close requests.
- [ ] Basic command/IPC interface.

## Milestone 3 — layout modes

- [ ] Tiling layout engine.
- [ ] Floating layout engine.
- [ ] Configuration-selected global mode.
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
