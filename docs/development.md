# Development guide

## Requirements

- Rust (edition 2024), pinned via `rust-toolchain.toml`
- [mpv](https://mpv.io) at runtime (and `yt-dlp` for online streams)
- Optional: mpv on PATH also enables the mpv integration tests
- Lua is embedded (mlua, vendored Lua 5.4); no system lua needed to build or
  run. Config is `init.lua` - see [docs/configuration.md](configuration.md).

## Common commands

| Command                                 | Purpose                                        |
| --------------------------------------- | ---------------------------------------------- |
| `cargo build`                           | debug binary                                   |
| `cargo check --all-targets`             | fast type check of lib, bin, tests, bench      |
| `cargo clippy --all-targets -- -D warnings` | lint gate (CI fails on warnings)           |
| `cargo test`                            | unit tests + integration suites                |
| `cargo fmt` / `cargo fmt --all -- --check` | format / verify formatting                  |
| `cargo build --release`                 | optimized binary at `target/release/vmp`       |
| `cargo bench`                           | run `benches/engine.rs` timings                |

Wrappers:

```sh
sh scripts/check.sh   # fmt --check + check + clippy -D warnings + test
sh scripts/test.sh    # cargo test + mpv integration notice + release build
```

## Layout conventions

- The binary is thin: `src/main.rs` only parses argv, attaches/spawns the
  daemon, relays daemon startup errors from `daemon-err`, and calls
  `vmp::ui::run`. Keep logic in the library so
  integration tests can exercise it (`src/lib.rs` declares the public modules).
- Modules are directories with a `mod.rs` re-exporting sibling content
  (`daemon/`, `mpv/`, `db/`, `config/`, `ui/panes/`). Shared helpers live in
  the `mod.rs`; visibility favors `pub(super)` over `pub` when only sibling
  files need an item, and the library surface stays minimal (pub only where
  tests or the binary need it).
- `Paths::for_root(root)` is the seam that decouples daemon/UI code from the
  real `~/.config/vmp` + XDG directories; tests always build isolated roots.

## Style rules

- No code comments unless explicitly requested; keep code self-explanatory.
- ASCII only in the UI
- No unicode/special symbols.
- Keybindings are configurable; never hardcode a key in the UI layer.
- `Shift+letter` keys arrive as uppercase from terminals (documented caveat).

## Working on the daemon

- `Daemon::serve(paths, config)` is the entry point tests use; it binds the
  socket, recovers a stale socket when the pid file no longer names a live
  process, and reports `daemon already running` for a live foreign daemon.
- Behavior strings are load-bearing (tests and UX assert them verbatim):
  `daemon already running`, `bind socket: {e}`, `vmp daemon: {e}`, `vmp: {e}`,
  `failed to load media`, `unknown command: {cmd} (try ...)`. Config problems
  all surface as `invalid config: ...` (including raw Lua errors). If you
  touch them, update the tests that pin them.
- Push semantics: snapshots are deduped against the previous snapshot and the
  event loop throttles position updates (~350ms); direct handler pushes
  (`handle_command`, mpv events) always go out immediately.

## Testing workflow for a change

1. Write/adjust the unit test next to the code (`#[cfg(test)] mod tests`).
2. If the change crosses process or module boundaries (protocol, daemon,
   engine), extend the matching tests/ suite - see
   [docs/testing.md](testing.md).
3. Run `cargo test`, then `sh scripts/check.sh`.
4. Update docs when behavior/doc-visible facts change (CONFIG.md for config,
   docs/protocol.md for wire messages, AGENTS.md module table for layout).
