# AGENTS.md

## Project

vmp - a simple Rust TUI media player. Backed by mpv (audio-only). Songs are
categorized by a tag system; checked tags define the play queue. Supports both
offline files and online streams (mpv/yt-dlp handles the stream).

## Commands

- Build: `cargo build`
- Check: `cargo check`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Test: `cargo test`
- Format: `cargo fmt --check` (format with `cargo fmt`)
- Bench: `cargo bench` (runs `benches/engine.rs`, no bench harness)
- Full local gate: `sh scripts/check.sh`; `sh scripts/test.sh` for
  test + release build

## Architecture

Single binary with two modes:

- `vmp --daemon`: headless long-running server. Owns the mpv subprocess, the
  SQLite library, the config/scripting evaluation, and the queue/playback
  engine. Exposes a JSON protocol over a
  Unix socket at `$XDG_RUNTIME_DIR/vmp/vmp.sock`. Survives TUI exit (background
  play).
- `vmp` (default): spawns the daemon if absent, then runs the ratatui TUI. The
  TUI is a thin, config-less client: it rebuilds keybindings/titles from the
  last snapshot's `config` block and renders daemon state
  snapshots. `Shift+q` detaches only; `q`/`esc` (after confirm) shuts the
  daemon down.

Storage follows XDG:
- `~/.config/vmp/` - `init.lua` (user config; never generated), `lua/` modules
- `~/.local/share/vmp/` - `library.sqlite3` (media library DB)
- `~/.local/state/vmp/` - `session.json` (persisted volume/repeat/shuffle/active tags/theme), `vmp.log` (daemon log)
- `$XDG_RUNTIME_DIR/vmp/` - `vmp.sock` (daemon IPC), `vmp.pid` (pid file), `mpv.sock`, `daemon-err`, probe sockets

`VMP_CONFIG_DIR` / `VMP_DATA_DIR` / `VMP_STATE_DIR` / `VMP_RUN_DIR` override
each directory; a spawned daemon inherits all four.

## Module layout

Library + thin binary split (`src/lib.rs` declares the public modules;
`src/main.rs` is argv/wiring only). Module directories put shared types in
`mod.rs` and re-export siblings:

- `src/lib.rs` - module roots; binary consumes `vmp::`
- `src/main.rs` - entry, argument handling, daemon spawn/attach
- `src/error.rs`, `src/logging.rs` - shared error enum, epoch-ms/file logging
- `src/app/action.rs` - Action enum, name/group/label tables
- `src/commands/` - ex-command parser (`:` mode) and builtins list
- `src/config/` - settings, loader (delegates Lua evaluation), validation,
  defaults; typed result of config only
- `src/scripting/` - Lua runtime ownership (`vmp.opt`/`vmp.map` host, eval
  once and drop the VM), `ScriptingError`; lands future daemon-owned plugin VM
- `src/storage/` - XDG paths (incl. `Paths::for_root` test seam), session state
- `src/db/` - SQLite schema, migrations, queries (rusqlite bundled)
- `src/engine.rs` - pure queue/tag filtering, repeat/shuffle, `Lcg`
- `src/daemon/` - mod (Daemon struct, run/serve/event loop), server, client
  (`Connection`, TUI side), dispatcher, snapshot, lifecycle
- `src/mpv/` - process spawn/drop, ipc request/reply, events, commands,
  properties (metadata from mpv)
- `src/proto.rs` - JSON protocol messages shared by daemon and TUI
- `src/health.rs` - `:checkhealth` dependency probes
- `src/ui/` - mod (re-exports), app (state machine + run), input (Keymap +
  LineInput), layout (grid), render (draw tree), panes/{filters,library,
  mini_queue,status}, dialog.rs, theme.rs
- `benches/engine.rs` - std-clock queue engine bench
- `tests/` - integration suites (see Testing)

## Config system

Config is an nvim-style Lua script, not TOML. Only the daemon evaluates it:
`Config::load(&paths.config, &paths.lua_dir())` reads the file and delegates
to `vmp::scripting::eval_config`, which boots embedded Lua 5.4 (mlua
vendored), installs `vmp.opt` and `vmp.map` with validating metatables, seeds
defaults, runs `init.lua`, harvests, and drops the VM; every failure (syntax,
runtime, type, unknown name) surfaces as `invalid config: ...`. Missing
`init.lua` = defaults, no file is written. `require()` resolves under
`~/.config/vmp/lua/`. Lua keywords need bracket syntax (`vmp.map["repeat"]`).
The effective settings ride every `Snapshot.config` (`proto::ConfigState`):
the TUI builds its `Keymap`/titles/steps from the snapshot and never runs Lua.
A spawned daemon's stderr is captured to `$XDG_RUNTIME_DIR/vmp/daemon-err`;
on early daemon exit the TUI relays that file verbatim before unlinking.

## Lua plugins (planned)

The config system already runs Lua; a fuller user plugin API comes later.
No stub directories exist yet by design; the reserved integration points are
`src/commands/` (registration of new ex-commands next to builtins) and the
daemon dispatcher path. See CONTRIBUTING.md for the design expectations.

## Conventions

- No code comments unless explicitly requested; keep code self-explanatory.
- ASCII only in the UI - no unicode/special symbols.
- Keybindings are configurable; never hardcode a key in the UI layer.
- Queue is newest-added-first; shuffle affects play order, not list display.
- Repeat (`r`) cycles off -> repeat-all -> repeat-one.
- `Space` toggles tags in the Filter pane and play/pause in the Queue pane.
- `Shift+letter` keys arrive as uppercase from terminals (documented caveat).

## Key details

- mpv is spawned with `--input-ipc-server` and controlled over JSON IPC. Pull
  metadata (title/artist/bitrate/duration) from mpv properties - no metadata
  crate.
- Active tag filter = media matching ALL checked tags.
- `/` search applies a regex over a precomposed lowercase searchable blob
  (title + artist + uri + tags).
- `f` toggles the reserved `favorite` tag.
- `Enter` plays the selected track; on the already-loaded track it resumes
  instead of reloading (default on; `vmp.opt.enter_resume_loaded` in
  init.lua; when the mpv handle is gone it falls back to a normal play).
- `:` opens Vim-style command mode: `:q` quits and stops the daemon,
  `:checkhealth` lists `mpv`/`yt-dlp` status in a dialog.
- `f10` opens the settings dialog (colorscheme dark/light, transparency on/off);
  choices live in daemon state and `session.json`, not init.lua.

## Testing

Keep queue/tag filtering, repeat/shuffle state machine, and protocol
serialization pure so they are unit-testable; unit tests sit next to the code.
Integration suites under `tests/`: engine, protocol, config (Lua host
behavior), storage (XDG paths + env overrides), database, daemon (real socket
e2e via `Daemon::serve` on temp paths; the cross-process test uses the built
`vmp` binary), and `mpv_integration` (self-skips when mpv is missing). Shared
fixtures live in `tests/common/mod.rs`. TUI rendering is not unit-tested.
