# vmp Configuration

`vmp` reads its settings from a Lua file, `init.lua`, the same way nvim reads
`init.lua`. No config file is required: missing entries fall back to defaults.

## Files

vmp follows the XDG layout and never generates config files:

| Path                            | Notes                                    |
| ------------------------------- | ---------------------------------------- |
| `~/.config/vmp/init.lua`        | config entry point (user-created)        |
| `~/.config/vmp/lua/`            | root for `require()` from your config    |
| `~/.local/share/vmp/library.sqlite3` | media library database             |
| `~/.local/state/vmp/session.json`    | volume/repeat/shuffle/active tags/theme |
| `~/.local/state/vmp/vmp.log`    | daemon log                               |
| `$XDG_RUNTIME_DIR/vmp/vmp.sock` / `vmp.pid` | daemon IPC, pid file         |

Each location has an env override: `VMP_CONFIG_DIR`, `VMP_DATA_DIR`,
`VMP_STATE_DIR`, `VMP_RUN_DIR` (all absolute paths):

```sh
VMP_CONFIG_DIR=/mnt/media/vmp-config vmp
```

Minimal `init.lua` (everything starts from defaults, so change only what you
want):

```lua
vmp.opt.volume_step = 3
vmp.opt.titles.queue = "Playlist"
vmp.map.move_down = { "j", "down" }
```

Separator inside `require` usage: `local my = require("custom.opts")` loads
`~/.config/vmp/lua/custom/opts.lua`.

## Validation and errors

The config is Lua executed on every startup (both the daemon and the TUI run
it). On error, `vmp` refuses to start and prints `invalid config:` followed by
the problem, e.g.:

- `vmp.opt: unknown option 'volume_stp'` - typos are rejected like nvim.
- `vmp.map: unknown action 'frobnicate'` - the left-hand side is not a known
  action name.
- `vmp.opt.volume_step: expected integer, got lua type table` - wrong value.
- Lua syntax and runtime errors are reported with the line that failed.

Binding a key to nothing disables that action's default key(s):

```lua
vmp.map.delete = "none"   -- 'd' no longer deletes
```

## Full annotated example

```lua
-- Binary used for playback. Any mpv-compatible or yt-dlp command works.
vmp.opt.mpv_binary = "mpv"

-- Amount the volume changes per press of volume_up / volume_down.
vmp.opt.volume_step = 5

-- Seek length in seconds for seek_back / seek_fwd.
vmp.opt.seek_step = 5.0

-- Border titles of the four panes. Optional; shown values are defaults.
vmp.opt.titles.filter = "Filter"  -- tag filter pane (top-left)
vmp.opt.titles.queue  = "Queue"   -- main queue pane (right)
vmp.opt.titles.mini   = "Mini Queue" -- mini-queue pane (bottom-left)
vmp.opt.titles.info   = "Info"    -- media details pane

-- Controls. Format: vmp.map.<action> = "key" or { "key", "key" }
vmp.map.move_down        = { "j", "down" }
vmp.map.move_up          = { "k", "up" }
vmp.map.prev_section     = "h"
vmp.map.next_section     = "l"
vmp.map.cycle_focus      = "tab"
vmp.map.cycle_focus_back = "backtab"
vmp.map.activate         = "enter"
vmp.map.toggle           = "space"
vmp.map.next_track       = "n"
vmp.map.prev_track       = "p"
vmp.map.volume_up        = "K"        -- uppercase letter means Shift
vmp.map.volume_down      = "J"
vmp.map.seek_back        = { "H", "left" }
vmp.map.seek_fwd         = { "L", "right" }
vmp.map["repeat"]        = "r"        -- brackets required: repeat is a Lua keyword
vmp.map.shuffle          = "s"
vmp.map.add_media        = "a"
vmp.map.edit_media       = "e"
vmp.map.search           = "/"
vmp.map.favorite         = "f"
vmp.map.add_mini         = "A"
vmp.map.focus_mini       = "b"
vmp.map.delete           = "d"
vmp.map.mini_move_up     = "ctrl+k"
vmp.map.mini_move_down   = "ctrl+j"
vmp.map.confirm_quit     = { "q", "esc" }
vmp.map.detach           = "Q"
vmp.map.settings         = "f10"
vmp.map.help             = "?"
vmp.map.command          = ":"
-- Enter on the already-loaded selected track resumes it instead of reloading.
vmp.opt.enter_resume_loaded = true
```

## Options (`vmp.opt`)

| Option                | Type    | Default | Meaning                                        |
| --------------------- | ------- | ------- | ----------------------------------------------- |
| `mpv_binary`          | string  | `"mpv"` | Executable used for audio playback              |
| `volume_step`         | integer | `5`     | Volume change per `volume_up`/`volume_down`     |
| `seek_step`           | float   | `5.0`   | Seek length in seconds per `seek_back`/`seek_fwd` |
| `enter_resume_loaded` | boolean | `true`  | Resume instead of reload on Enter over the loaded track |
| `titles`              | table   | see above | Pane border titles: `filter`, `queue`, `mini`, `info` |

## Settings dialog

Press `f10` (rebindable via the `settings` action) to open the settings
dialog. It has two options:

| Option         | Values       | Meaning                                              |
| -------------- | ------------ | ---------------------------------------------------- |
| `colorscheme`  | dark / light | `light` paints panes black-on-white; `dark` keeps the terminal default |
| `transparency` | off / on     | `on` skips pane background painting so the terminal's own background shows through |

Changes apply immediately, broadcast to every connected client, and are
persisted in `session.json` so they survive daemon restarts. Navigate with
`up`/`down`, change a value with `left`/`right` or `enter`, close with `esc`.

## Commands

Press `:` (rebindable via the `command` action) to open the command line at the
bottom of the screen. `Esc` cancels without running anything.

Commands resolve nvim-style: the full name works, and so does any unambiguous
prefix (`:vol` -> `:volume`, `:nex` -> `:next`, `:cle` -> `:clear_tags`). An
ambiguous prefix lists the candidates (`:t` -> `tag, tags, theme, toggle,
transparency`). Names are lowercase; `!` after a name forces the action
(`:delete!` skips the confirm dialog). `:help commands` shows the full list in
the UI, and `:help NAME` describes one command.

While the command line is open, `Tab`/`Down` and `Shift+Tab`/`Up` open and
cycle a borderless vertical completion popup above it, nvim-style. It completes
command names (`:rep<Tab>` -> `:repeat `) and enumerated arguments:
repeat/shuffle/theme/transparency values, `:pane` and `:mini` words, `:help`
topics, and known tag names for `:tag`. Typing filters the list live.

| Command                                  | Action                                                        |
| ---------------------------------------- | ------------------------------------------------------------- |
| `play [n]`                               | Play the selected track, or the n-th queue item                |
| `pause` / `resume` / `toggle`            | Pause, resume, or play/pause                                   |
| `stop`                                   | Stop playback and unload the track                             |
| `next` (`n`) / `prev`                    | Next / previous track                                          |
| `seek [+N\|-N\|M:SS]`                    | Seek relative or absolute; bare shows the position             |
| `volume [+N\|-N\|N]` (`vol`)             | Change or set the volume; bare shows the level                 |
| `repeat [off\|all\|one]`                 | Cycle or set the repeat mode                                   |
| `shuffle [on\|off]`                      | Toggle or set shuffle                                          |
| `tag NAME`                               | Toggle a tag in the filter                                     |
| `tags`                                   | List tags with counts and checked marks                        |
| `clear_tags`                             | Uncheck every tag                                              |
| `search PATTERN`                         | Set a regex filter; bare clears it                             |
| `noh`                                    | Clear the search filter                                        |
| `add [URI [TAGS]]`                       | Add media (quote a uri with spaces); bare opens the dialog     |
| `edit`                                   | Edit the selected media                                        |
| `delete`                                 | Delete the selected media (`delete!` skips the confirm dialog) |
| `favorite`                               | Toggle favorite on the selected media                          |
| `mini [add\|remove\|up\|down]`           | Pin, unpin, or move the selected media in the mini queue       |
| `up` / `down`                            | Move the cursor in the focused pane                            |
| `pane [next\|prev\|filter\|mini\|queue\|info]` | Focus a pane                                             |
| `help [keys\|commands\|NAME]` (`h`)      | Keybindings, the command list, or one command's usage          |
| `settings`                               | Open the settings dialog (same as `f10`)                       |
| `theme [dark\|light]`                    | Toggle or set the colorscheme                                  |
| `transparency [on\|off]`                 | Toggle or set transparency                                     |
| `detach`                                 | Detach the TUI; playback continues (same as `Shift+q`)         |
| `q`, `quit`                              | Quit the app and stop the daemon                               |
| `checkhealth`                            | List dependencies (`mpv`, `yt-dlp`) and report their status    |

`checkhealth` runs `mpv --version` (using `mpv_binary`) and `yt-dlp --version`.
`mpv` is required; a missing `yt-dlp` is reported as a warning because it is
only needed for online streams. Unknown or ambiguous commands are reported in
the status line.

Examples: `:volume -10`, `:seek 1:30`, `:add ~/music/a.flac rock live`,
`:repeat one`, `:pane queue`, `:tag rock`, `:delete!`.

## Titles (`vmp.opt.titles`)

Renames the four pane borders. Useful if you show different content or want a
different language. Assign a single field (`vmp.opt.titles.info = "Now"`) and
the rest fall back to defaults, or assign the whole table at once.

## Keybindings (`vmp.map`)

Every line binds exactly one action to a key (or a list of keys):

```lua
vmp.map.toggle = "space"              -- one key
vmp.map.confirm_quit = { "q", "esc" } -- several keys trigger the same action
vmp.map["repeat"] = "none"            -- unbind
vmp.map.search = "ctrl+s"             -- a control chord
```

Rules:

- Actions must use their exact names (see below).
- A key may appear in only one action.
- Defaults fill in for anything you do not set; you never need to write the
  whole map.

> Shift caveat: terminals report `Shift+letter` as the uppercase letter, so
> `vmp.map.volume_up = "K"` is bound to `Shift+k`. There is no separate "shift
> modifier" syntax.

> Lua keywords: action names that collide with Lua keywords must use bracket
> syntax (`vmp.map["repeat"]`). `repeat = "r"` is a syntax error.

## Key syntax

A key is written as one of:

| Form              | Example        | Notes                              |
| ----------------- | -------------- | ---------------------------------- |
| Single character  | `"j"`, `"/"`, `"A"` | letters, digits, symbols; uppercase = Shift |
| `ctrl+<char>`     | `"ctrl+k"`     | control chord                      |
| `f<N>`            | `"f5"`         | function keys `f1` .. `f24`        |
| Named keys        | `"space"`      | see list below                     |

Named keys: `space`, `enter`, `esc`, `tab`, `backtab`, `up`, `down`, `left`,
`right`, `home`, `end`, `pageup`, `pagedown`, `insert`, `delete`,
`backspace`.

## Actions

| Action            | What it does                                              |
| ----------------- | --------------------------------------------------------- |
| `move_up`         | Move the cursor up in the focused list                    |
| `move_down`       | Move the cursor down in the focused list                  |
| `prev_section`    | Focus the previous pane                                   |
| `next_section`    | Focus the next pane                                       |
| `cycle_focus`     | Focus the next pane (same as `next_section`)              |
| `cycle_focus_back`| Focus the previous pane                                   |
| `activate`        | Queue: play selected track; Filter: toggle the tag        |
| `toggle`          | Queue: play/pause; Filter: toggle the tag                 |
| `next_track`      | Skip to the next track in the queue                       |
| `prev_track`      | Return to the previous track                              |
| `volume_up`       | Raise volume by `volume_step`                             |
| `volume_down`     | Lower volume by `volume_step`                             |
| `seek_back`       | Seek backward by `seek_step` seconds                      |
| `seek_fwd`        | Seek forward by `seek_step` seconds                       |
| `repeat`          | Cycle repeat off -> all -> one                            |
| `shuffle`         | Toggle shuffle                                            |
| `add_media`       | Open a dialog to add a file, path, or stream URL          |
| `edit_media`      | Open a dialog to edit the selected media                  |
| `search`          | Open the regex search bar                                 |
| `favorite`        | Toggle the reserved `favorite` tag                        |
| `add_mini`        | Pin the selected track to the mini-queue (plays next)     |
| `focus_mini`      | Jump focus to/from the mini-queue                         |
| `delete`          | Delete the selected item (confirmed in the main queue)    |
| `mini_move_up`    | Move the selected mini-queue item up                      |
| `mini_move_down`  | Move the selected mini-queue item down                    |
| `confirm_quit`    | Quit the app (with confirmation); stops the daemon        |
| `detach`          | Detach the TUI; playback continues in the daemon          |
| `settings`        | Open the settings dialog (colorscheme, transparency)      |
| `help`            | Open the keybindings help dialog                          |
| `command`         | Open the Vim-style command line                           |

## Default keybindings

With no `vmp.map` assignments, these defaults apply:

| Action             | Keys                    |
| ------------------ | ----------------------- |
| `move_down`        | `j`, `down`             |
| `move_up`          | `k`, `up`               |
| `prev_section`     | `h`                     |
| `next_section`     | `l`                     |
| `cycle_focus`      | `tab`                   |
| `cycle_focus_back` | `backtab`               |
| `activate`         | `enter`                 |
| `toggle`           | `space`                 |
| `next_track`       | `n`                     |
| `prev_track`       | `p`                     |
| `volume_up`        | `K`                     |
| `volume_down`      | `J`                     |
| `seek_back`        | `H`, `left`             |
| `seek_fwd`         | `L`, `right`            |
| `repeat`           | `r`                     |
| `shuffle`          | `s`                     |
| `add_media`        | `a`                     |
| `edit_media`       | `e`                     |
| `search`           | `/`                     |
| `favorite`         | `f`                     |
| `add_mini`         | `A`                     |
| `focus_mini`       | `b`                     |
| `delete`           | `d`                     |
| `mini_move_up`     | `ctrl+k`                |
| `mini_move_down`   | `ctrl+j`                |
| `confirm_quit`     | `q`, `esc`              |
| `detach`           | `Q`                     |
| `settings`         | `f10`                   |
| `help`             | `?`                     |
| `command`          | `:`                     |

## Migrating from rmp2

vmp replaces the old rmp2 project and its TOML config. There is no automatic
migration: create `~/.config/vmp/init.lua` yourself and move your library DB:

```sh
mkdir -p ~/.local/share/vmp
mv ~/.config/rmp2/library.sqlite3 ~/.local/share/vmp/
```

Convert any `config.toml` overrides to the Lua form above by hand
(`action = "key"` becomes `vmp.map.<action> = "key"`).
