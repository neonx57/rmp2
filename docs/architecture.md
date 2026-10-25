# Architecture

vmp is a Rust TUI media player backed by mpv (audio-only). It ships as a
single binary with two modes. Songs are categorized by a tag system; the
checked tags define the play queue. Offline files and online streams both work
(mpv/yt-dlp handles the stream).

## Process model

- `vmp --daemon` (headless server): owns the mpv subprocess, the SQLite
  library, and the queue/playback engine. It binds
  `$XDG_RUNTIME_DIR/vmp/vmp.sock`, exposes the JSON protocol from
  [docs/protocol.md](protocol.md), writes `vmp.pid` / `vmp.log` /
  `session.json`, and survives TUI exit so playback continues in the
  background.
- `vmp` (default): spawns the daemon if none is running, then runs the ratatui
  TUI. The TUI is a thin client: it sends protocol `Command`s and renders
  `Snapshot`s pulled from the daemon. `Shift+q` detaches only; a
  confirmed `q`/`esc` shuts the daemon down.

## Storage layout (XDG)

| Directory                        | Contents                                    |
| -------------------------------- | -------------------------------------------- |
| `~/.config/vmp/`                 | `init.lua` (user-created), `lua/` (require() root) |
| `~/.local/share/vmp/`            | `library.sqlite3`                            |
| `~/.local/state/vmp/`            | `session.json`, `vmp.log`                    |
| `$XDG_RUNTIME_DIR/vmp/`          | `vmp.sock`, `vmp.pid`, `mpv.sock`, probe sockets |

Each directory can be overridden with `VMP_CONFIG_DIR`, `VMP_DATA_DIR`,
`VMP_STATE_DIR`, and `VMP_RUN_DIR`; a spawned daemon inherits all four so the
TUI and daemon always agree on locations. Tests use the `Paths::for_root` seam
instead of touching real XDG directories.

## Module map

| Path                            | Responsibility                                                          |
| ------------------------------- | ----------------------------------------------------------------------- |
| `src/main.rs`                   | argv handling, daemon spawn/attach, wiring only                         |
| `src/lib.rs`                    | library root; the binary consumes this via `vmp::`                      |
| `src/error.rs`                  | shared `Error` enum (io/config/daemon) with verbatim message text       |
| `src/logging.rs`                | epoch-ms timestamps and append-only log file                            |
| `src/app/`                      | `action.rs` - action enum, name/group/label tables                      |
| `src/commands/`                 | Vim-style ex-commands: `mod.rs`, `parser.rs` (`ExCommand`), `builtins.rs`|
| `src/config/`                   | Lua config - `lua_host.rs` (embedded Lua 5.4 host: bootstrap, seed, harvest), `loader.rs` `Config::load`, `validation.rs` key/action checks + binding merge, `defaults.rs` bindings + `default_init_lua` text, `settings.rs` structs |
| `src/storage/`                  | `paths.rs` XDG `Paths` (incl. the `for_root` test seam), `state.rs` persisted session state |
| `src/db/`                       | SQLite library - `mod.rs` types, `connection.rs`, `schema.rs`, `migrations.rs`, `queries.rs` |
| `src/engine.rs`                 | pure queue/tag filtering, repeat/shuffle state machine, `Lcg`           |
| `src/mpv/`                      | mpv control - `mod.rs` handle, `process.rs` spawn/drop, `ipc.rs` request/reply, `events.rs` reader loop, `commands.rs`, `properties.rs` |
| `src/daemon/`                   | daemon - `mod.rs` `Daemon` struct + run/serve/event loop, `server.rs` socket accept, `client.rs` `Connection` (TUI side), `dispatcher.rs` command/event handling, `snapshot.rs` snapshot + push semantics, `lifecycle.rs` spawn/pid/signals/logging |
| `src/proto.rs`                  | JSON protocol messages shared by daemon and TUI                         |
| `src/health.rs`                 | `:checkhealth` dependency probes                                        |
| `src/ui/`                       | ratatui app - `app.rs` input/state machine + `run`, `input.rs` `Keymap`/`LineInput`, `layout.rs` grid, `render.rs` draw tree, `panes/` filter/library/mini-queue/status renderers, `dialog.rs` modals, `theme.rs` palettes + settings options |
| `benches/engine.rs`             | std-clock micro-bench for the queue engine (no bench harness)           |
| `tests/`                        | integration suites; see [docs/testing.md](testing.md)                   |
| `scripts/`                      | `check.sh` (fmt+check+clippy+test), `test.sh` (test+build)               |

## Config pipeline

Config is an nvim-style Lua script, not a data format. `Config::load` only
reads `init.lua` when the user created it; a missing file is pure defaults and
vmp never writes the file itself.

1. `lua_host.rs` boots embedded Lua 5.4 (vendored via mlua, no system lua),
   sets `package.path` to the config dir's `lua/` so `require()` resolves
   user modules, and installs the `vmp` global from a Lua bootstrap chunk.
2. The bootstrap defines `vmp.opt` and `vmp.map` with metatables that reject
   unknown options/actions exactly like nvim; Rust then seeds the known
   option/action tables and the default values via `raw_set`.
3. `init.lua` runs. Assignments may be plain (`vmp.opt.volume_step = 3`,
   `vmp.map.move_down = { "j", "down" }`, `vmp.map.repeat = ...` is invalid —
   use `vmp.map["repeat"] = "r"` because `repeat` is a Lua keyword) and
   `"none"` still unbinds an action.
4. Harvesting walks the tables back out with type-checked getters; every
   config failure - syntax, runtime, type, unknown name - surfaces as
   `invalid config: ...`.
5. `validation.rs` merges defaults under user bindings (so partial configs
   keep untouched defaults) and rejects duplicate keys across actions.

## Data flow

1. The TUI resolves a KeyEvent through `ui/input.rs` `Keymap` (built from the
   `init.lua` bindings harvested by `lua_host.rs`) into an `app::Action`.
2. `ui/app.rs` maps the action to a `proto::Command` and pushes it over the
   `daemon/client.rs` `Connection` (writer thread).
3. `daemon/server.rs` accepts, `daemon/dispatcher.rs` applies the command,
   `daemon/snapshot.rs` builds a `Snapshot` and fans it out to clients.
4. The `Connection` reader thread receives raw JSON; the TUI keeps only the
   latest snapshot (`pull()` drains).
5. mpv events arrive on `daemon/mod.rs`'s event loop (`daemon/` owns the mpv
   reader thread from `mpv/events.rs`), driving pause/position/end-file state.

## Invariants worth knowing

- Snapshot `push()` dedupes against `last_snap` and throttles position
  updates to ~350ms in the event loop; handler-triggered pushes bypass the
  throttle.
- The queue is newest-added-first; shuffle affects play order, never list
  display.
- Active tag filter = media matching ALL checked tags.
- `/` search compiles a regex over a precomposed lowercase searchable blob
  (title + artist + uri + tags).
- `f` toggles the reserved `favorite` tag.
- `f10` settings (colorscheme/transparency) live in daemon state and
  `session.json`, not in `init.lua`.

## Plugins

The config system itself is already Lua: `init.lua` can `require()` modules
from `~/.config/vmp/lua/`, giving users a real scripting surface today.

A fuller plugin API is on the roadmap (see CONTRIBUTING.md): users will be
able to register new ex-commands and extend behavior in Lua. The landing
points are reserved but intentionally absent today - no empty stubs exist.

- Lua plugins will register new ex-commands next to the existing builtins in
  `src/commands/` (`builtins.rs` lists the builtins, `parser.rs` owns parsing);
  `ui/app.rs::run_command` already routes every `:` line through
  `commands::parse_ex_command`, so third-party commands will slot in there.
- The daemon's command dispatcher (`src/daemon/dispatcher.rs`) remains the
  single authority over protocol state; plugin-visible actions will travel the
  same `proto::Command` path.
