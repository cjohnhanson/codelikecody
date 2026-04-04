# Attribution

This crate is a derivative work of
[agent-browser](https://github.com/vercel-labs/agent-browser) by Vercel Inc.,
originally licensed under the Apache License, Version 2.0. The full license
text is in [LICENSE](LICENSE).

## Origin

- **Upstream repository:** https://github.com/vercel-labs/agent-browser
- **Upstream path:** `cli/` (Rust CLI crate)
- **Original copyright:** Copyright 2025 Vercel Inc.
- **Original license:** Apache-2.0

## Modifications

The following changes have been made from the upstream source:

- Renamed the crate from `agent-browser` to `moose`
- Added `native/animation.rs` (new file, not present upstream)
- Removed `native/test_fixtures/` directory
- Removed dependencies: `hmac`, `hex`, `chrono`, `urlencoding`, `windows-sys`
- Removed release profile configuration
- Integrated into the codelikecody workspace

Per Apache-2.0 Section 4(b), all files carried over from upstream have been
modified as part of the vendoring process. This notice serves as the
prominent notice of changes required by the license.
