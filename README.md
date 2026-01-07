# rdwm

rdwm is a small, minimalist dynamic window manager written in Rust using XCB.

## Features
- Minimal dynamic tiling layout (even horizontal tiling across visible windows)
- Multiple workspaces (configurable via `NUM_WORKSPACES`)
- Keyboard-driven actions: spawn apps, kill focused client, change focus, switch workspaces
- Lightweight: few dependencies (xcb, xkbcommon)

## Building
1. Install Rust toolchain (stable) and necessary X development libraries for `xcb` and `xkbcommon`.
2. Build in debug mode:

```bash
cargo build
```

3. Build a release binary:

```bash
cargo build --release
```

Running (preview)
- The repository includes `preview.sh` which builds and then uses `xinit` with `Xephyr` to run `rdwm` in a nested X server for testing.
- Example:

```bash
./preview.sh
```

Notes
- Configure key bindings and behavior in `src/config.rs` and `src/key_mapping.rs`.
- Logging uses the `log` and `env_logger` crates; run with `RUST_LOG=debug` to see debug output.
- This project is experimental — use in a nested session (Xephyr) for testing before using as your main window manager.

## TODO

- [x] Horizontal tiling with even spacing across visible windows
- [x] Multiple workspaces (configurable via `NUM_WORKSPACES`)
- [x] Keyboard-driven controls: spawn apps, kill focused client, focus next/previous, switch workspaces
- [x] Lightweight, minimal dependencies (`xcb`, `xkbcommon`) and simple configuration in `src/config.rs`
- [x] Basic logging via `log` + `env_logger`
- [x] Move windows around within a workspace 
- [x] Resize windows interactively
- [x] Move windows between workspaces
- [x] Add EWMH (NETWM) / ICCCM hints for better compatibility with external panels/status bars and desktop tools
- [ ] Create a custom status bar (or integration points) so you can build your own bar displaying workspaces, layout, and window titles
- [ ] Additional layouts (stacking, master-stack, dynamic layouts) and configurable gaps
- [ ] Improved multi-monitor support and per-monitor workspaces
- [ ] More robust error handling and configuration parsing
