# 🌈 spectro-rs Wiki

Welcome to the **spectro-rs** documentation wiki! This is your comprehensive guide to using the ColorMunki spectrometer with Rust.

## 📖 Quick Navigation

### 🎯 Getting Started
- **[Installation Guide](Installation)** - Setup for Windows, macOS, and Linux
- **[First Steps with GUI](Getting-Started)** - Quick introduction to spectro-gui
- **[Driver Setup](Driver-Setup)** - WinUSB and udev configuration

### 📏 Measurement Guides
- **[Calibration](Calibration)** - How and when to calibrate your device
- **[Measurement Modes](Measurement-Modes)** - Reflective, Emissive, and Ambient modes explained
- **[Color Analysis](Color-Analysis)** - Understanding Lab, CCT, and Delta E values

### 🔧 For Developers
- **[API Reference](API-Reference)** - Using spectro-rs as a library
- **[Architecture](Architecture)** - Project structure and design patterns
- **[Contributing](Contributing)** - How to contribute to the project

### ⚙️ Configuration
- **[Language Settings](Language-Settings)** - Switching between English and Chinese
- **[Theme Settings](Theme-Settings)** - Light and Dark mode configuration
- **[Colorimetry Standards](Colorimetry-Standards)** - Illuminant and Observer selection

---

## 🔗 Quick Links

| Resource | Link |
|----------|------|
| **Core Library** | [![Crates.io](https://img.shields.io/crates/v/spectro-rs.svg)](https://crates.io/crates/spectro-rs) |
| **GUI Application** | [![Crates.io](https://img.shields.io/crates/v/spectro-gui.svg)](https://crates.io/crates/spectro-gui) |
| **API Documentation** | [docs.rs/spectro-rs](https://docs.rs/spectro-rs) |
| **GitHub Releases** | [Download Binaries](https://github.com/Tinnci/spectro-rs/releases) |
| **Source Code** | [github.com/Tinnci/spectro-rs](https://github.com/Tinnci/spectro-rs) |

---

## 📣 Recent Updates

### v0.3.4 (December 2025)
- ✨ **Internationalization**: Added runtime language switching (English/Chinese)
- 🎨 **Theme Support**: Light and Dark mode with automatic UI adaptation
- 🔧 **Colorimetry Settings**: Configurable Illuminant and Observer selection

### v0.3.3
- ⚙️ Added Illuminant (D65, D50, A, F2, F7, F11) selection
- 📐 Added Observer (2°, 10°) selection
- 🎯 Improved calibration wizard

### v0.3.0
- 🏗️ Refactored to Spectrometer trait architecture
- 📦 Split into `spectro-rs` (core) and `spectro-gui` (GUI) crates
- 🚀 Published to crates.io

---

## 💡 Need Help?

- **Bug Reports**: [Open an Issue](https://github.com/Tinnci/spectro-rs/issues/new)
- **Feature Requests**: [Open an Issue](https://github.com/Tinnci/spectro-rs/issues/new)
- **Pull Requests**: [Contributing Guide](Contributing)

---

> **Note**: This wiki is maintained alongside the codebase. Feel free to suggest improvements!
