# XDG lifecycle probe

Alpinium includes a small Wayland client that exercises the XDG toplevel map/unmap/remap sequence.

## Build

From the repository root:

```sh
cargo check --all-targets
cargo run --example xdg_lifecycle_probe
```

The example must connect to an Alpinium nested Wayland socket. Start Alpinium in one terminal without `--command`:

```sh
cargo run -- --nested --config config/alpinium.default.conf
```

Note the Wayland socket name printed by Alpinium (for example, `wayland-1`). In another terminal, use that name:

```sh
WAYLAND_DISPLAY=wayland-1 cargo run --example xdg_lifecycle_probe
```

Keep the same `XDG_RUNTIME_DIR` as the terminal running Alpinium. Replace `wayland-1` with the actual socket name printed by the compositor.

## What the probe checks

The probe creates a real XDG toplevel and a visible shared-memory buffer, then:

1. Performs the initial bufferless commit and acknowledges the initial configure.
2. Attaches a buffer to map the surface.
3. Detaches the buffer and checks for `wl_surface.leave`.
4. Performs the required post-unmap bufferless commit and checks that a fresh XDG configure arrives.
5. Acknowledges the configure, attaches the buffer again, and checks for `wl_surface.enter`.
6. Destroys the toplevel cleanly.

A successful run prints PASS lines for the initial configure, unmap, fresh configure, remap, and complete lifecycle sequence.

The probe is a manual runtime test: CI compiles it through `cargo check --all-targets` but does not run it because it requires a live Wayland compositor.
