# AGENTS.md

## Project

rmp - a simple Rust TUI media player. Backed by mpv (audio-only). Songs are
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

- `rmp --daemon`: headless long-running server. Owns the mpv subprocess, the
  SQLite library, and the queue/playback engine. Exposes a JSON protocol over a
  Unix socket at `~/.config/rmp2/rmp.sock`. Survives TUI exit (background play).
- `rmp` (default): spawns the daemon if absent, then runs the ratatui TUI. The
  TUI is a thin client: it forwards keybindings and renders daemon state
  snapshots. `Shift+q` detaches only; `q`/`esc` (after confirm) shuts the
  daemon down.

Everything lives under `~/.config/rmp2`:
- `config.toml` - user config (keybinds, paths, defaults)
- `library.sqlite3` - media library DB
- `rmp.sock` - daemon IPC socket
- `rmp.pid` - daemon pid file
- `last-state.json` - persisted volume/repeat/shuffle/active tags/theme
- `rmp.log` - daemon log

## Module layout

Library + thin binary split (`src/lib.rs` declares the public modules;
`src/main.rs` is argv/wiring only). Module directories put shared types in
`mod.rs` and re-export siblings:

- `src/lib.rs` - module roots; binary consumes `rmp::`
- `src/main.rs` - entry, argument handling, daemon spawn/attach
- `src/error.rs`, `src/logging.rs` - shared error enum, epoch-ms/file logging
- `src/app/action.rs` - Action enum, name/group/label tables
- `src/commands/` - ex-command parser (`:` mode) and builtins list
- `src/config/` - settings/mod+loader+validation+defaults for config.toml
- `src/storage/` - paths (incl. `Paths::for_root` test seam), last-state
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

## Lua plugins (planned)

Users will write plugins in Lua. No stub directories exist yet by design;
the reserved integration points are `src/commands/` (registration of new
ex-commands next to builtins) and the daemon dispatcher path. See
CONTRIBUTING.md for the design expectations.

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
- `:` opens Vim-style command mode: `:q` quits and stops the daemon,
  `:checkhealth` lists `mpv`/`yt-dlp` status in a dialog.
- `f10` opens the settings dialog (colorscheme dark/light, transparency on/off);
  choices live in daemon state and `last-state.json`, not config.toml.

## Testing

Keep queue/tag filtering, repeat/shuffle state machine, and protocol
serialization pure so they are unit-testable; unit tests sit next to the code.
Integration suites under `tests/`: engine, protocol, config, storage,
database, daemon (real socket e2e via `Daemon::serve` on temp paths; the
cross-process test uses the built binary), and `mpv_integration`
(self-skips when mpv is missing). Shared fixtures live in
`tests/common/mod.rs`. TUI rendering is not unit-tested.