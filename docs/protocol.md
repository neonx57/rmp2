# Protocol reference

The daemon and the TUI exchange newline-delimited JSON over the Unix socket at
`$XDG_RUNTIME_DIR/vmp/vmp.sock`. Types are defined in `src/proto.rs`; this
document describes the wire behavior pinned by `tests/protocol.rs`.

## Commands (TUI -> daemon)

Requests are `Command` values serialized with an intrusive tag:

```json
{"action": "select", "id": 5}
{"action": "toggle_tag", "tag": "rock"}
{"action": "add", "uri": "/music/a.flac", "title": "A", "tags": ["rock"]}
```

All variants and their `snake_case` action names:

| action            | payload                              | notes                                   |
| ----------------- | -------------------------------------- | ---------------------------------------- |
| `play_pause`      | -                                     | toggles or starts the first queued track |
| `next` / `prev`   | -                                     | walks the filtered queue                 |
| `stop`            | -                                     | stops playback and unloads the track     |
| `play`            | `id: i64`                             |                                          |
| `select`          | `id: i64`                             | marks the highlighted row                |
| `seek`            | `delta: f64`                          | relative seconds                         |
| `volume`          | `delta: i32`                          | clamped 0..130                           |
| `set_volume`      | `volume: i32`                         | clamped 0..130                           |
| `repeat_cycle`    | -                                     | Off -> All -> One -> Off                 |
| `set_repeat`      | `mode: "Off"\|"All"\|"One"`           | sets the repeat mode directly            |
| `shuffle_toggle`  | -                                     |                                          |
| `set_shuffle`     | `on: bool`                            | sets shuffle directly                    |
| `toggle_tag`      | `tag: string`                         | lowercased before matching               |
| `clear_tags`      | -                                     |                                          |
| `set_search`      | `pattern: string?`                    | empty string clears                      |
| `add`             | `uri, title?, tags`                   | offline uris must exist on disk          |
| `update`          | `id, title?, artist?, tags`           | replaces tags wholesale                  |
| `delete`          | `id: i64`                             |                                          |
| `add_mini`        | `id: i64`                             | moves to back of mini queue              |
| `remove_mini`     | `id: i64`                             |                                          |
| `mini_move`       | `id: i64, delta: i64`                 | clamped swap                             |
| `toggle_favorite` | `id: i64`                             | reserved `favorite` tag                  |
| `set_theme`       | `theme: "dark"|"light"`               |                                          |
| `set_transparency`| `on: bool`                            |                                          |
| `shutdown`        | -                                     | daemon stops playing and exits           |

The parser is strict: unknown actions and missing required fields are
rejected (`{"action":"play"}` without `id` does not parse).

## Snapshots (daemon -> TUI)

The daemon streams full `Snapshot` objects, one JSON object per line, plus a
separate immediate snapshot on each client connect:

```json
{
  "theme": "dark", "transparent": false,
  "all_media": [ ... MediaInfo ... ],
  "queue": [1, 2], "mini_queue": [2],
  "tags": [ {"name": "rock", "checked": true, "count": 42} ],
  "search": "one", "selected": { ... MediaInfo ... },
  "now": { "id": 2, "position": 12.25, "duration": 181.5, "paused": false, "loading": true },
  "volume": 65, "repeat": "All", "shuffle": false,
  "notify": "added: One", "error": null,
  "config": {
    "mpv_binary": "mpv",
    "titles": { "filter": "Filter", "queue": "Queue", "mini": "Mini Queue", "info": "Info" },
    "volume_step": 5,
    "seek_step": 5.0,
    "map": { "move_down": ["j", "down"], "add_mini": ["A"] }
  }
}
```

Field notes:

- `config` carries the daemon's effective settings (`proto::ConfigState`,
  evaluated once from `init.lua` at daemon start). The TUI rebuilds its
  `Keymap` and pane titles from this block whenever it changes and never
  evaluates Lua itself. Older snapshot payloads without the field still
  parse - it defaults (`#[serde(default)]`).
- `repeat` is PascalCase (`"Off"`, `"All"`, `"One"`) unlike `theme`, which is
  lowercase - a deliberate-but-quirky difference kept for compatibility and
  covered by tests.
- `MediaInfo.tags` always includes the `favorite` tag when set
  (`m.favorite` is derived from it).
- `selected` is echoed only when the id resolves in the library.
- `now` is `null` when nothing is loaded; `paused` mirrors mpv's pause state
  and `loading` covers both initial loads and buffer pauses.
- `notify` and `error` ride exactly one push each: the daemon clears its copy
  (`take()`) right after building the snapshot, so the following push carries
  `null` for both.

## Push semantics

- On connect the daemon immediately sends a snapshot.
- Subsequent pushes dedupe against the previous snapshot; the event loop
  throttles continuous position updates to ~350ms. Commands that change state
  push immediately.
- `config` changes only when the daemon restarts with different settings, so
  the block rides the dedupe-aware full snapshot without extra traffic.
- A `notify`/`error` string is washed out once by the daemon (`take()`), so
  the very next dedup push will not repeat it.

## Constants

- `FAVORITE_TAG = "favorite"` (exported from `proto`).
- The socket lives in the runtime dir and other paths come from the XDG
  layout (`VMP_CONFIG_DIR` / `VMP_DATA_DIR` / `VMP_STATE_DIR` / `VMP_RUN_DIR`
  override), see [docs/configuration.md](configuration.md).
