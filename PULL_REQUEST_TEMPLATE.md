# Summary

Describe the change and why it is needed. Link related issues.

# Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Refactor (no behavior change)
- [ ] Documentation or CI
- [ ] Breaking change (protocol, config, DB schema, or daemon behavior)

# Checklist

- [ ] `sh scripts/check.sh` passes (fmt, clippy -D warnings, tests)
- [ ] New or changed behavior is covered by tests (unit `#[cfg(test)]` or tests/)
- [ ] Config/protocol/schema changes are documented (CONFIG.md, docs/protocol.md,
      docs/configuration.md) and AGENTS.md stays accurate
- [ ] UI strings and keybindings remain ASCII; no hardcoded keys in the UI layer
- [ ] No new runtime dependencies; rust-toolchain.toml still matches CI
