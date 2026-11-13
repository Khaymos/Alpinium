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

Both modes share the same compositor, configuration system, decorations, and visual-effects engine. Alpinium is one compositor with two selectable modes — not two separate window managers.

> **Development status:** this repository currently contains the Rust project scaffold and the first configuration-loader milestone. It is not yet a usable Wayland compositor. Rendering, input handling, window management, and desktop integration are still under development.

## Design goals

- **Two layout modes:** choose **tiling** or **floating** with one configuration directive.
- **User-configurable:** user config at **~/.config/alpinium/config**; system default at **/etc/alpinium/config**.
- **Shared visual system:** animations, rounded corners, shadows, gaps, and future effects behave consistently in either mode.
- **Native Wayland:** built as a Wayland compositor, not an X11-only window manager.
- **KhaymOS integration:** first-party KhaymOS desktop component, with portable configuration.
- **Performance-aware:** visual effects can be disabled individually.

## Configuration

Alpinium uses a Sway-inspired, line-oriented configuration format. The first parser milestone supports a deliberately small set of directives:

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

1. **--config PATH**, when explicitly supplied on the command line.
2. **$XDG_CONFIG_HOME/alpinium/config**, when set to an absolute path.
3. **~/.config/alpinium/config**.
4. **/etc/alpinium/config**.
5. Built-in defaults if no file exists.

The first matching file is loaded as a complete configuration; user and system configs are not merged in this milestone. An explicitly supplied but missing or invalid file produces an error rather than silently falling back.

The default config in this repository is **config/alpinium.default.conf**. KhaymOS packaging will install it as **/etc/alpinium/config**.

See [docs/CONFIGURATION.md](docs/CONFIGURATION.md) for supported directives and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the planned compositor architecture.

## Build the current scaffold

Requirements: Rust 1.87 or newer and Cargo.

~~~sh
git clone https://github.com/Khaymos/Alpinium.git
cd Alpinium
cargo test
cargo run -- --check-config
cargo run -- --print-config
~~~

Validate a specific config:

~~~sh
cargo run -- --config ./config/alpinium.default.conf --check-config
~~~

The current executable validates and displays configuration only. It does not start a Wayland session yet.

## Planned technology

The compositor backend is planned around [Smithay](https://github.com/Smithay/smithay), a Rust library providing Wayland compositor building blocks and system backends. Alpinium will implement its own window-management policy, tiling/floating layouts, decorations, animation engine, and configuration semantics.

The Smithay dependency will be introduced with the first backend milestone, after the required backend features and KhaymOS runtime dependencies are selected.

## Roadmap

- [x] Rust project scaffold and configuration model.
- [x] **mode tiling** and **mode floating** parser.
- [x] Configuration validation CLI and unit tests.
- [ ] Smithay-based nested development backend.
- [ ] Display and manage basic Wayland client surfaces.
- [ ] Implement tiling and floating layout engines.
- [ ] Add keybindings, workspaces, and output management.
- [ ] Add decorations, animations, shadows, and rounded corners.
- [ ] Integrate Alpinium into KhaymOS profiles and installer.

## Contributing

Alpinium is part of the KhaymOS project. Contributions, design discussions, bug reports, and testing are welcome. Please keep proposals aligned with the goals above and document configuration changes.

## License

Alpinium is licensed under the [MIT License](LICENSE).
