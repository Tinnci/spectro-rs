# 🚀 Release Guide

This project uses `cargo-release` for version management and **GitHub Actions** for automated publishing to [crates.io](https://crates.io/crates/spectro-rs).

## 🛠️ Prerequisites

1.  **cargo-release**: Install if you haven't already:
    ```powershell
    cargo install cargo-release
    ```
2.  **Git State**: Ensure your working directory is clean (`git status`).

---

## 📦 How to Publish a New Version

### Release `spectro-rs` (Core Library)

1.  **Preview the Release (Dry Run)**:
    ```powershell
    cargo release patch -p spectro-rs
    ```

2.  **Execute the Release**:
    ```powershell
    cargo release patch -p spectro-rs --execute
    ```

### Release `spectro-gui` (GUI Application)

After `spectro-rs` is published and indexed on crates.io (~30 seconds), release the GUI:

1.  **Preview**:
    ```powershell
    cargo release patch -p spectro-gui
    ```

2.  **Execute**:
    ```powershell
    cargo release patch -p spectro-gui --execute
    ```

---

## ⚙️ What Happens Automatically

When you run `cargo release ... --execute`:

1.  **Bumps version** in `Cargo.toml`.
2.  **Commits** the change as `chore: Release`.
3.  **Tags** the commit (e.g., `spectro-rs-v0.3.1`).
4.  **Pushes** the commit and tag to GitHub.
5.  **GitHub Action** detects the tag and publishes to crates.io.

> **Note**: Local publishing is disabled (`publish = false` in `release.toml`).
> All publishing is handled by GitHub Actions for security and consistency.

---

## 🔍 Troubleshooting

-   **GitHub Action Fails**: Ensure `CRATES_IO_TOKEN_RS` and `CRATES_IO_TOKEN_GUI` are correctly set in GitHub Repository Secrets.
-   **Crate Not Found**: If `spectro-gui` fails because `spectro-rs` isn't indexed yet, wait ~30 seconds and retry.
-   **Check Action Status**:
    ```powershell
    gh run list --workflow publish.yml
    ```
