---
description: How to release a new version of spectro-rs
---

1. Ensure the working directory is clean and CI is passing.
2. The core library (`spectro-rs`) must be released first, followed by the GUI (`spectro-gui`).
3. Run a dry run for the core library:
   ```bash
   cargo release patch -p spectro-rs --dry-run
   ```
4. Execute the release (bumps version, tags, and pushes):
   ```bash
   # Use --registry crates-io if you have custom registries configured
   cargo release patch -p spectro-rs --execute --registry crates-io
   ```
5. Update `spectro-gui` to match the version and dependency:
   - Update `version` in `crates/spectro-gui/Cargo.toml`.
   - Update `spectro-rs` dependency version.
   ```bash
   cargo release patch -p spectro-gui --execute --registry crates-io
   ```
6. Verify the GitHub Action progress for binary releases:
   ```bash
   gh run list --workflow publish.yml
   ```
