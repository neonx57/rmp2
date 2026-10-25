---
name: Bug report
about: Report a problem in vmp or its daemon
title: ""
labels: bug
assignees: ""
---

**Describe the bug**
A clear description of what happened and what you expected instead.

**To reproduce**
Steps or keybindings that trigger the issue. For TUI bugs, note the pane and
focused section; for daemon bugs, attach `~/.local/state/vmp/vmp.log` (respecting
privacy).

**Environment**
- OS: (distro, terminal, WSL where applicable)
- vmp version or commit: (`git rev-parse --short HEAD`)
- mpv version: (`mpv --version`)
- yt-dlp present: (`command -v yt-dlp`)

**Logs**
`tail -40 ~/.local/state/vmp/vmp.log`

**Additional context**
Anything else that may matter (init.lua contents, error dialog text).
