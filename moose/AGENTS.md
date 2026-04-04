# moose — vendored agent-browser

This crate is vendored from [vercel-labs/agent-browser](https://github.com/vercel-labs/agent-browser)
and licensed under Apache-2.0. See [ATTRIBUTION.md](ATTRIBUTION.md) for details.

## License obligations

When modifying this crate, the following must be maintained:

1. **LICENSE** must remain the unmodified Apache-2.0 license text from upstream.
2. **ATTRIBUTION.md** must be updated when:
   - Files are added that don't exist upstream (list them under Modifications)
   - Files are removed that exist upstream (list them under Modifications)
   - Dependencies are added or removed relative to upstream
3. **Cargo.toml** must keep `license = "Apache-2.0"`. Do not change this to MIT
   or any other license.

## Syncing from upstream

There is no automated sync process yet. When pulling changes from upstream:

1. Compare `cli/src/` in the upstream repo against `moose/src/` here
2. Apply changes, preserving any local modifications (e.g. `animation.rs`)
3. Update ATTRIBUTION.md if the file or dependency delta changed
4. Verify `moose/LICENSE` still matches upstream's `LICENSE`
