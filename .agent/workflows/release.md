---
description: How to release a new version of spectro-rs
---

## Release Process

The release is a two-step process. GitHub Actions handles the actual publishing.

### Step 1: Release Core Library (`spectro-rs`)

1. Run a dry run first:
   ```bash
   cargo release patch -p spectro-rs
   ```

2. Execute the release:
   ```bash
   cargo release patch -p spectro-rs --execute
   ```

3. Wait for GitHub Action to publish (~30 seconds for crates.io indexing).

### Step 2: Release GUI (`spectro-gui`)

1. Run a dry run:
   ```bash
   cargo release patch -p spectro-gui
   ```

2. Execute:
   ```bash
   cargo release patch -p spectro-gui --execute
   ```

### Verify

Check GitHub Action status:
```bash
gh run list --workflow publish.yml
```
