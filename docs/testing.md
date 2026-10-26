# Testing

## Where tests live

- **Unit tests** sit next to the code in `#[cfg(test)] mod tests` blocks:
  engine filtering/repeat/shuffle, protocol serde, db schema/queries, keymap
  and line input, scroll offset and status bar.
- **Integration tests** live under `tests/` and link against the `vmp` lib:

| Suite                       | Covers                                                              |
| --------------------------- | --------------------------------------------------------------------- |
| `tests/engine.rs`           | queue walking, repeat/shuffle (`tests/engine/{queue,filtering,shuffle}.rs`) |
| `tests/protocol.rs`         | `Command`/`Snapshot` JSON wire format, tagged action names, strict parsing |
| `tests/config.rs`           | Lua host eval/defaults merge, `vmp.opt`/`vmp.map` validation errors, `require()` over `lua/`, `none` unbind |
| `tests/storage.rs`          | XDG `Paths` layout, `VMP_*_DIR` env overrides, `session.json` roundtrip + corruption fallback |
| `tests/database.rs`         | library CRUD, uri upsert, tag/favorite semantics, legacy column migration |
| `tests/daemon.rs`           | real daemon e2e: socket snapshots, library add/select/queue, persisted state, shutdown, config delivery in `Snapshot.config`, invalid-config relay via `daemon-err`; plus a cross-process "daemon already running" test using the built binary |
| `tests/mpv_integration.rs`  | mpv probe of a generated WAV fixture + a live `Mpv` IPC session; self-skips (printing "skip") when mpv is missing |

A shared fixture module (`tests/common/mod.rs`) builds small PCM16 WAV files
in code so no binary assets sit in the repo.

## Running

```sh
cargo test                # everything; mpv tests self-skip without mpv installed
cargo test --test daemon  # one suite (engine, protocol, config, storage, database, daemon, mpv_integration)
cargo test --lib          # unit tests only
cargo test --test mpv_integration -- --nocapture   # see skip/progress messages
```

## Daemon e2e pattern

`tests/daemon.rs` runs `Daemon::serve(temp_paths, Config::default())` on a
thread, connects the real `Connection`, asserts snapshots with a polling
helper (the daemon pushes over the socket asynchronously), and finishes with
`Command::Shutdown`. The cross-process test spawns `env!("CARGO_BIN_EXE_vmp")
--daemon` with the four `VMP_*_DIR` env vars pointed at a temp root: it
verifies the `daemon already running` refusal, sends state-changing commands,
and waits for a clean exit after `Shutdown`. Config-delivery tests run the
daemon with a real `init.lua` (custom `vmp.opt`/`vmp.map`) and assert the
values arrive inside `Snapshot.config`; the relay test exercises the TUI
spawn path against a broken `init.lua` and pins the daemon's verbatim
`invalid config:` output plus `daemon-err` cleanup.

## Self-skipping mpv tests

`tests/mpv_integration.rs` checks `command -v mpv` semantics via
`Command::new("mpv").arg("--version")`; if unavailable the tests print a skip
notice and pass. When mpv is present they probe metadata for the generated
tone WAV (~0.3s, 8kHz mono) and drive a real `Mpv` spawn: loadfile, file-loaded
event (via an event-drainer thread so request replies keep flowing), duration
property, pause roundtrip, quit.

## CI

`.github/workflows/ci.yml` runs fmt check, `cargo check --all-targets`,
clippy `-D warnings`, `cargo test`, and a release build on ubuntu-latest, with
`actions-rust-lang/setup-rust-toolchain@v3` honoring `rust-toolchain.toml`.
The bench target is type-checked by those `-all-targets` gates and executed
locally with `cargo bench`; the std-clock output is informational, no
threshold assertions.
