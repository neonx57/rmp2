# Contributing

Thanks for improving rmp. This project is a small, opinionated Rust codebase
(master builds stay green: fmt, clippy `-D warnings`, tests).

## Getting started

```sh
git clone https://github.com/your-fork/rmp2.git
cd rmp2
cargo test          # 58 unit tests + integration suites (mpv tests self-skip)
sh scripts/check.sh # fmt --check + check + clippy -D warnings + test
```

## Ground rules

1. **No code comments** unless explicitly requested; keep code
   self-explanatory (names over comments).
2. **ASCII only in the UI** - no unicode/special symbols in rendered screens,
   dialogs, or status bar text.
3. **Keybindings are configurable**; never hardcode a key in the UI layer.
   New actions get an entry in `src/app/action.rs` plus a default binding in
   `src/config/defaults.rs`, and `AGENTS.md`'s keybinding docs follow.
4. **Zero behavior change refactors** are welcome; behavior changes need
   either a pinned test update (note why the string/semantics changed) or a
   deliberate protocol/config migration.
5. **Log/behavior strings are load-bearing.** Tests assert verbatim text such
   as `daemon already running`, `failed to load media`, or the
   `unknown command: ... (try ...)` hint. If you change one, find and update
   the pinning tests.
6. **Minimal `pub` surface**: expose library items only where `tests/` or the
   binary reach them; `pub(super)` is the default for shared items inside a
   module directory.
7. **No new runtime dependencies** without discussion; the bench is std-only
   by design.

## Testing expectations

- Unit tests next to the code for logic, integration suites under `tests/`
  for boundaries (engine, protocol, config, storage, database, daemon,
  mpv_integration). See [docs/testing.md](docs/testing.md).
- The daemon e2e suite runs the real `Daemon::serve`; keep it hermetic by
  always building `Paths::for_root(temp_dir)`.
- `tests/mpv_integration.rs` must keep its self-skip behavior (pass without
  mpv installed).

## Pull requests

Use `.github/PULL_REQUEST_TEMPLATE.md`. Keep PRs focused; run
`sh scripts/check.sh` before pushing.

## Planned direction: Lua plugins

A core roadmap item: **users will be able to write rmp plugins in Lua.**
There are no empty stub directories for it yet by design; the plan reserves
these touch points:

- New user commands will register alongside the builtins in `src/commands/`
  (`builtins.rs` for command discovery, `parser.rs` for line parsing). The
  `:` command path (`ui/app.rs::run_command`) already funnels everything
  through `commands::parse_ex_command`, so plugin commands integrate there.
- TUI-visible menu/dialog panels will hook into `src/ui/` like the existing
  help and health dialogs.
- Runtime state stays daemon-owned: plugin actions flow through
  `proto::Command` and `daemon/dispatcher.rs` rather than poking the SQLite
  library directly.

Design discussion for a Lua changeset is required before implementation
(incubator scripts first, no interpreter dependency landing until the plugin
API settles).
