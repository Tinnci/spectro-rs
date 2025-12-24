# 🚀 Release Guide

This project is configured to use `cargo-release` for version management and **GitHub Actions** for automated publishing to [crates.io](https://crates.io/crates/spectro-rs).

## 🛠️ Prerequisites

1.  **cargo-release**: Install if you haven't already:
    ```powershell
    cargo install cargo-release
    ```
2.  **Git State**: Ensure your working directory is clean (`git status`).

---

## 📦 How to Publish a New Version

You don't need to manually edit `Cargo.toml` or create tags. Follow these steps:

### 1. Preview the Release (Dry Run)
Always run a dry run first to see what changes will be made:
```powershell
# Bump patch version of core library
cargo release patch -p spectro-rs

# Or bump minor version
cargo release minor -p spectro-rs
```

### 2. Execute the Release
If the preview looks correct, execute the release:
```powershell
cargo release patch -p spectro-rs --execute
```

**What this command does automatically:**
1.  **Bumps version** in `Cargo.toml`.
2.  **Commits** the change as `chore: Release x.y.z`.
3.  **Tags** the commit as `vx.y.z`.
4.  **Pushes** the commit and the tag to GitHub.
5.  **GitHub Action** then detects the `v*` tag and publishes to `crates.io`.

---

## 🔍 Troubleshooting

- **Permissions**: If the GitHub Action fails, ensure `CRATES_IO_TOKEN` is correctly set in your GitHub Repository Secrets.
- **CI Failure**: The release tag push will trigger both the standard CI and the Publish workflow. If CI fails, the publish won't proceed (as defined in the workflow).
