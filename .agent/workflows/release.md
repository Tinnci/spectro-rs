---
description: How to release a new version of spectro-rs
---

1. Ensure the working directory is clean.
2. Run a dry run to check the version bump:
   ```bash
   cargo release patch -p spectro-rs
   ```
3. Execute the release (bumps version, commits, tags, and pushes):
   ```bash
   cargo release patch -p spectro-rs --execute
   ```
4. Verify the GitHub Action progress:
   ```bash
   gh run list --workflow publish.yml
   ```
