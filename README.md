# 🌈 spectro-rs

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Crates.io](https://img.shields.io/crates/v/spectro-rs.svg)](https://crates.io/crates/spectro-rs)

[中文文档 (Chinese)](./README_zh.md)

**spectro-rs** is a high-performance Rust driver for X-Rite ColorMunki (Original/Design) spectrometers. It provides a modern, safe, and easy-to-use cross-platform interface for color measurement, display calibration, and light analysis.

---

## ✨ Features

- **🚀 Cross-platform**: Supports Windows, macOS, and Linux.
- **📊 Multi-mode Measurement**:
    - **Reflective**: Measure paper, prints, and materials. Includes automated dark/white calibration.
    - **Emissive**: Optimized `emtx` matrix for accurate display/monitor measurement.
    - **Ambient**: Measure light source spectral power distribution (SPD).
- **🧪 Colorimetry Engine**:
    - Real-time calculation of **CIE XYZ**, **Chromaticity (x, y)**, and **CIE L*a*b***.
    - Estimated **CCT (Correlated Color Temperature)** and **Spectral Centroid**.
- **🎨 Spectral Visualization**: Live ANSI color spectrum chart in your terminal.
- **🌐 Internationalization**: Built-in English and Chinese (Simplified) support.

---

## 🛠️ Getting Started

### 1. Prerequisites
- [Rust toolchain](https://rust-lang.org).
- **Windows**: If the device is not detected, use [Zadig](https://zadig.akeo.ie/) to replace the driver with `WinUSB`.
- **Linux**: Ensure you have correct `udev` rules for USB access.

### 2. Run
```bash
cargo run
```

---

## 📖 Operational Guide

### 🔄 Calibration
Always run **Restart Calibration** before critical measurements:
1. Turn the dial to the **White Dot (Position 2)**.
2. The program will perform a **Dark Frame** (Lamp OFF) followed by **White Tile** (Lamp ON) calibration.

### 📱 Monitor Mode (Emissive)
1. Turn the dial to **Measurement (Position 4)**.
2. Place the device firmly against the screen.
3. Select **Measure Emissive (Monitor)**.

### 💡 Light Source (Ambient)
1. Turn the dial to **Ambient (Position 1)** (with the diffuser罩).
2. Point toward the light source.
3. Select **Measure Ambient (Light Source)**.

---

## 🏗️ Technical Background

Inspired by **ArgyllCMS**:
- **EEPROM Logic**: Replicates memory mapping for linearization polynomials and factory matrices.
- **Spectral Mapping**: Transposes 128 sensor readings to 36 standard 10nm bands (380nm-730nm).

---

## ⚖️ License

Licensed under **[GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.html)**.

---

## 🤝 Contributing

Contributions are welcome! Please open an issue for bugs or feature requests (e.g., support for i1Display Pro).
