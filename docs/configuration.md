# Configuration reference

vmp reads `~/.config/vmp/init.lua` (override the directory with the
`VMP_CONFIG_DIR` env var; see [CONFIG.md](../CONFIG.md) for the user-facing
guide). This page is the deeper reference for how the pieces work. The layout
follows XDG: `~/.config/vmp/` for config, `~/.local/share/vmp/` for the
library DB, `~/.local/state/vmp/` for `session.json` + `vmp.log`, and
`$XDG_RUNTIME_DIR/vmp/` for sockets and the pid file.

## Load order

1. `Config::load` reads `init.lua` only if the user created it. A missing file
   means pure defaults — vmp never generates the file.
2. Lua 5.4 is embedded (vendored via mlua) and boots first with a small
   bootstrap chunk that installs the `vmp` global with validating metatables.
3. Rust seeds the known option/action tables and the default values, then
   `init.lua` executes top to bottom.
4. Assignments are harvested with type checks and merged over the defaults in
   `validation.rs`.
5. `require()` inside `init.lua` resolves modules under
   `~/.config/vmp/lua/` (`lua/?.lua;lua/?/init.lua`), so e.g.
   `require("custom.opts")` loads `lua/custom/opts.lua`.
6. Upgrading from the old rmp2 layout: move `library.sqlite3` into
   `~/.local/share/vmp/` yourself; there is no automatic migration and the old
   `~/.config/rmp2/config.toml` is simply ignored.

## API

```lua
-- vmp.opt.<key> = value   plain assignment; unknown options are errors
vmp.opt.mpv_binary = "mpv"
vmp.opt.volume_step = 5          -- integer
vmp.opt.seek_step = 5.0          -- number (seconds)
vmp.opt.enter_resume_loaded = true

-- pane border titles, per-field or as one table
vmp.opt.titles.filter = "Filter"
vmp.opt.titles = { filter = "Filter", queue = "Queue", mini = "Mini Queue", info = "Info" }

-- vmp.map.<action> = key string or list of keys
vmp.map.move_down = { "j", "down" }
vmp.map["repeat"] = "r"          -- brackets needed: repeat is a Lua keyword
vmp.map.move_up = "none"         -- "none" disables the default binding
```

- Unknown options (`vmp.opt.enter_resume_load = true`) and unknown actions
  (`vmp.map.frobnicate = "q"`) are rejected, as are wrong value types.
- Keybindings merge with defaults: only what you assign changes; untouched
  actions keep their default keys.
- Duplicate keys across actions fail (`key "q" bound to multiple actions`).
- All failures - syntax errors, runtime errors, type errors, unknown names -
  print starting with `invalid config:` followed by the Lua-side message.
- Keys: single printable characters (`Shift` letters written uppercase),
  `ctrl+<char>`, `f1`..`f24`, and the named keys (`space`, `enter`, `esc`,
  `tab`, `backtab`, arrows, `home`, `end`, `pageup`, `pagedown`, `insert`,
  `delete`, `backspace`).
- Keybinding semantics (`validation.rs` + `input.rs::key_string`): `Shift+char`
  maps to the uppercase character, `Ctrl+j` is written `ctrl+j`, and
  repeating/unmodified keystrokes only count on key press, not release/repeat
  events.

## Options

- `mpv_binary` (string, default `"mpv"`): executable the daemon spawns.
- `volume_step` (integer, default `5`): percent jump for `volume_up`/`volume_down`.
- `seek_step` (number, default `5.0`): seconds for `seek_fwd`/`seek_back`.
- `enter_resume_loaded` (boolean, default `true`): when `Enter` is pressed on
  the track that is still loaded (`now_id` unchanged), the daemon unpauses
  instead of re-issuing `loadfile`, so playback continues at the current
  position with no loading screen. Setting it to `false` restores the
  always-reload behavior (every `Enter` restarts the selected track from
  zero). `Next`/`Prev`, tag-driven auto-advance, and mini-queue playback keep
  the reload semantics.
- `titles` (table): pane border titles `filter`, `queue`, `mini`, `info`.

## Settings dialog vs init.lua

`f10` toggles colorscheme (dark/light) and transparency (on/off). These live
in daemon memory and `session.json` (`"theme"` serializes lowercase:
`"light"`), deliberately outside `init.lua`.

## session.json

Persisted across runs and restored on daemon start: theme, transparency,
volume, repeat mode (PascalCase `Off`/`All`/`One` on the JSON wire), shuffle,
active tags, media id, position, and whether playback should resume. Missing
or corrupt files fall back to defaults.

## Files on disk

| File                          | Directory        | Producer                                    |
| ----------------------------- | ---------------- | -------------------------------------------- |
| `init.lua`                    | config           | user-created only; missing = defaults        |
| `lua/**`                      | config           | user modules (`require()` root)              |
| `library.sqlite3`             | data             | daemon (SQLite, rusqlite bundled)            |
| `vmp.sock`                    | runtime          | daemon (Unix socket, removed on shutdown)    |
| `vmp.pid`                     | runtime          | daemon pid; referenced for stale recovery    |
| `mpv.sock`                    | runtime          | mpv `--input-ipc-server` (owned by daemon)   |
| `session.json`                | state            | daemon `persist()` on settings changes       |
| `vmp.log`                     | state            | daemon append log with epoch-ms timestamps   |
