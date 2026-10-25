# Configuration reference

rmp reads `~/.config/rmp2/config.toml` (override the directory with the
`RMP2_DIR` env var; see [CONFIG.md](../CONFIG.md) for the user-facing guide and
history). This page is the deeper reference for how the pieces work.

## Load order and migration

1. On first run `Config::load` writes a fully commented default file (see
   `config/defaults.rs`) and uses the in-code defaults.
2. Legacy configs in the old `key = action` format are detected and rewritten
   to the current `action = key(s)` format in place, merging default bindings
   and honoring `none`-disabled keys.
3. New-format files with actions written as keys are left untouched.

## Schema

```toml
mpv_binary = "mpv"
volume_step = 5
seek_step = 5.0

[titles]                # pane border titles
filter = "Filter"
queue = "Queue"
mini = "Mini Queue"
info = "Info"

[keybindings]
# action = key, or action = ["key", "key"]
move_down = ["j", "down"]
```

- Unknown actions or invalid key names fail with `invalid config:` followed by
  one line per problem (including duplicate-key detection across actions:
  `key "q" bound to multiple actions`). Errors print verbatim from
  `Error::Config`.
- `"none"` disables the default binding for that action (the legacy migration
  maps a `key = "none"` entry to dropping that key from every action).
- Keys: single printable characters (`Shift` letters written uppercase),
  `ctrl+<char>`, `f1`..`f24`, and the named keys listed in the generated
  comment block (`space`, `enter`, `esc`, `tab`, `backtab`, arrows, `home`,
  `end`, `pageup`, `pagedown`, `insert`, `delete`, `backspace`).
- Keybinding semantics (`validation.rs` + `input.rs::key_string`): `Shift+char`
  maps to the uppercase character, `Ctrl+j` is written `ctrl+j`, and
  repeating/unmodified keystrokes only count on key press, not release/repeat
  events.

## Settings dialog vs config.toml

`f10` toggles colorscheme (dark/light) and transparency (on/off). These live
in daemon memory and `last-state.json` ("theme" serializes lowercase:
`"light"`), deliberately outside config.toml.

## last-state.json

Persisted across runs and restored on daemon start: theme, transparency,
volume, repeat mode (PascalCase `Off`/`All`/`One` on the JSON wire), shuffle,
active tags, media id, position, and whether playback should resume. Missing
or corrupt files fall back to defaults.

## Runtime directory

| File               | Producer                                  |
| ------------------ | ------------------------------------------ |
| `config.toml`      | first-run generation, user edits           |
| `library.sqlite3`  | daemon (SQLite, rusqlite bundled)          |
| `rmp.sock`         | daemon (Unix socket, removed on shutdown)  |
| `rmp.pid`          | daemon pid; referenced for stale recovery  |
| `last-state.json`  | daemon `persist()` on settings changes     |
| `rmp.log`          | daemon append log with epoch-ms timestamps |
| `mpv.sock`         | mpv `--input-ipc-server` (owned by daemon) |
