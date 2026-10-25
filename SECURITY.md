# Security policy

## Supported versions

Only the latest commit on the default branch is supported. Tagged releases
(`v*`) get fixes on a best-effort basis.

## Reporting a vulnerability

Do NOT open a public issue. Use GitHub's private vulnerability reporting for
this repository, or contact the maintainers directly via email (see commit
metadata for addresses). Include:

- rmp commit/revision
- how the daemon or TUI misbehaves
- any `rmp.log` excerpt, config snippet, or media URI (strip personal data)

## Threat model notes

- The daemon listens on a **Unix socket** under the user's config directory -
  local-user separation only; treat anything that can write to
  `~/.config/rmp2/` as the same trust domain (socket, pid file, database).
- Media URIs are passed to **mpv** and online streams go through **yt-dlp**;
  loading hostile media content is delegated to those tools, so keep both
  current.
- The library database executes parameterized SQL exclusively (rusqlite,
  bundled SQLite). Tag names are lowercased before storage/matching; regex
  search patterns are compiled with the vanilla `regex` crate (no backtracking
  engine).
- The daemon writes `last-state.json`, `rmp.log`, and pid/socket files in the
  config directory without strict permission enforcement; run it under your
  normal user, not root.
