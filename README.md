<div align="center">
  <h1>Alpinium</h1>
  <p><strong>A configurable Wayland compositor for KhaymOS.</strong></p>
  <p>One compositor. Two global window-management modes. One shared visual-effects system.</p>
  <p>
    <img src="https://img.shields.io/badge/status-early%20development-orange" alt="Early development">
    <img src="https://img.shields.io/badge/language-Rust-dea584" alt="Rust">
    <img src="https://img.shields.io/badge/display%20protocol-Wayland-4584b6" alt="Wayland">
    <img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT license">
  </p>
</div>

---

## What is Alpinium?

Alpinium is the planned native Wayland compositor for the KhaymOS ecosystem. It will support both tiling and floating window management, selected globally through the user's configuration file.

Both modes will share the same compositor, configuration system, decorations, and visual-effects engine. Alpinium is one compositor with two selectable modes — not two separate window managers.

> **Development status:** the repository contains a configuration parser, a Smithay-based nested Wayland backend, and the first working master-stack tiling/floating policies. This is still an early development compositor; decorations, animations, advanced layout controls, and production session support are not complete.

## Design goals

- **Two layout modes:** choose tiling or floating with one configuration directive.
- **User-configurable:** user config at ~/.config/alpinium/config; system default at /etc/alpinium/config.
- **Shared visual system:** animations, rounded corners, shadows, gaps, and future effects behave consistently in either mode.
- **Native Wayland:** built as a Wayland compositor, not an X11-only window manager.
- **KhaymOS integration:** first-party KhaymOS desktop component, with portable configuration.
- **Performance-aware:** visual effects can be disabled individually.

## Configuration

Alpinium uses a Sway-inspired, line-oriented configuration format.

~~~conf
# ~/.config/alpinium/config

# Choose one global window-management mode.
mode tiling
# mode floating

animations enabled
animation_duration 200ms
shadows enabled
corner_radius 8
gaps inner 5
gaps outer 5
~~~

### Configuration lookup

Alpinium checks the following locations in order:

1. --config PATH, when explicitly supplied on the command line.
2. $XDG_CONFIG_HOME/alpinium/config, when set to an absolute path.
3. ~/.config/alpinium/config.
4. /etc/alpinium/config.
5. Built-in defaults if no file is found.

The first matching file is loaded as a complete configuration; user and system configs are not merged in this milestone. An explicitly supplied but missing or invalid file produces an error rather than silently falling back.

The default config in this repository is config/alpinium.default.conf. KhaymOS packaging will install it as /etc/alpinium/config.

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for supported directives and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the planned compositor architecture.

## Build and test

Requirements: Rust 1.88 or newer, Cargo, and the native Wayland/EGL/X11 development libraries needed by Smithay's nested backend.

~~~sh
git clone https://github.com/Khaymos/Alpinium.git
cd Alpinium
cargo test
cargo run -- --check-config
cargo run -- --config config/alpinium.default.conf --check-config
~~~

## Run the nested compositor

The first backend uses Smithay's Winit backend to open Alpinium in a normal desktop window. It does not replace your current session.

To launch Alpinium with Alacritty as a nested Wayland client:

~~~sh
cargo run -- --nested --config config/alpinium.default.conf --command alacritty
~~~

The nested backend creates its own Wayland socket under the current user's runtime directory and launches the requested client with that socket in WAYLAND_DISPLAY.

The nested backend currently supports two global modes:
- **Tiling:** the first mapped window is the master on the left; later windows share a vertical stack on the right. Inner and outer gaps come from the config. Client move/resize requests are ignored while tiling owns the geometry.
- **Floating:** windows can be moved and resized freely using the existing pointer grabs.

The first layout policy is intentionally simple. Workspaces, configurable master ratios, keyboard-driven window management, decorations, and effects are still planned.

## Planned technology

Alpinium uses [Smithay](https://github.com/Smithay/smithay) as its low-level Wayland compositor library. Smithay provides protocol and system-level building blocks; Alpinium implements its own window-management policy, tiling/floating layouts, decorations, animation engine, and configuration semantics.

The nested development backend is based on the structure of Smithay's Smallvil example. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Roadmap

- [x] Rust project scaffold and configuration parser.
- [x] mode tiling and mode floating config support.
- [x] User/system configuration lookup and validation CLI.
- [x] Initial Smithay nested backend integration.
- [ ] Build and verify the nested backend on developer machines and CI.
- [ ] Display and manage Wayland client surfaces reliably.
- [x] Implement the first master-stack tiling and floating layout policies.
- [ ] Add keybindings, workspaces, and output management.
- [ ] Add decorations, animations, shadows, and rounded corners.
- [ ] Integrate Alpinium into KhaymOS profiles and installer.

## Contributing

Alpinium is part of the KhaymOS project. Contributions, design discussions, bug reports, and testing are welcome. Please keep proposals aligned with the goals above and document configuration changes.

## License

Alpinium is licensed under the MIT License (see LICENSE).
