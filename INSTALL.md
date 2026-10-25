# Installing vmp

The project builds from source; there are no prebuilt packages yet. You only
need the Rust toolchain and the two runtime prerequisites, `mpv` and
`yt-dlp`. The compiled binary embeds Lua 5.4 for the config system, so there
is no system lua dependency.

## Prerequisites

- [Rust](https://rustup.rs) (edition 2024; a recent stable toolchain)
- [mpv](https://mpv.io) - playback engine
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) - required only for online streams

## Building from source

```sh
git clone https://github.com/neonx57/vmp.git
cd vmp
cargo build --release
```

The binary is produced at `target/release/vmp`. Install it somewhere on your
`PATH`:

```sh
install -Dm755 target/release/vmp ~/.local/bin/vmp   # per-user
# or, system-wide:
sudo install -Dm755 target/release/vmp /usr/local/bin/vmp
```

Verify the binary is on `PATH`:

```sh
command -v vmp
vmp          # starts the daemon on first use and opens the TUI
```

## Installing by operating system

### Linux

Install the runtime dependencies with your package manager, then follow the
build steps above.

Debian / Ubuntu:

```sh
sudo apt install mpv yt-dlp
```

Fedora:

```sh
sudo dnf install mpv yt-dlp
```

Arch / Manjaro:

```sh
sudo pacman -S mpv yt-dlp
```

If you do not have Rust yet: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### macOS

```sh
brew install mpv yt-dlp rust
```

Then build from source as above.

### Windows

Windows is not natively supported: the daemon talks to the TUI over Unix
domain sockets, which Windows does not provide. Use
[WSL](https://learn.microsoft.com/windows/wsl/install) (Windows Subsystem for
Linux). Inside WSL, follow the Linux instructions; run `vmp` from within the
WSL terminal (e.g. Windows Terminal).

## Config locations

First run creates the XDG directories automatically; `init.lua` itself is
yours to create (running without one uses all defaults):

| Directory                              | Contents                          |
| ---------------------------------------| ----------------------------------|
| `~/.config/vmp/`                        | `init.lua`, `lua/` modules        |
| `~/.local/share/vmp/`                   | `library.sqlite3`                 |
| `~/.local/state/vmp/`                   | `session.json`, `vmp.log`         |
| `$XDG_RUNTIME_DIR/vmp/`                 | `vmp.sock`, `vmp.pid`, `mpv.sock` |

Upgrading from the old rmp2 layout: move your library by hand - there is no
automatic migration:

```sh
mkdir -p ~/.local/share/vmp
mv ~/.config/rmp2/library.sqlite3 ~/.local/share/vmp/
```

## Optional: autostart the daemon

The daemon survives the TUI and keeps playing in the background. If you want
it to always be available, run it as a `systemd` user service.

```sh
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/vmp.service <<'EOF'
[Unit]
Description=vmp media player daemon

[Service]
ExecStart=%h/.local/bin/vmp --daemon
Restart=on-failure

[Install]
WantedBy=default.target
EOF
systemctl --user enable --now vmp.service
```

The TUI connects to the already-running daemon instead of spawning a new one,
and `q`/`esc` only shuts down `vmp` itself in that case.

## Uninstalling

```sh
rm ~/.local/bin/vmp                          # remove the binary
rm -rf ~/.config/vmp ~/.local/share/vmp ~/.local/state/vmp  # remove library + config
systemctl --user disable --now vmp.service   # if you added the service
```