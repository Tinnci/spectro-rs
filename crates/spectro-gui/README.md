# 🎨 spectro-gui

[![Crates.io](https://img.shields.io/crates/v/spectro-gui.svg)](https://crates.io/crates/spectro-gui)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

A modern graphical interface for **X-Rite ColorMunki** spectrometers, built with [egui](https://github.com/emilk/egui). Visualize spectral power distribution, analyze colors, and calibrate your display.

## ✨ Features

- **📊 Live Spectral Plot**: Real-time SPD visualization
- **🎨 Color Swatch**: Accurate sRGB rendering of measured colors
- **📈 Spectral Analysis**: Peak wavelength, centroid, and CCT
- **🔄 Multi-Mode**: Reflective, Emissive (Monitor), and Ambient measurement
- **✓ Auto-Calibration**: Remembers calibration data per device
- **🌐 Internationalization**: English and Chinese (Simplified) with runtime switching
- **🎭 Theme Support**: Light and Dark mode with automatic UI adaptation
- **⚙️ Colorimetry Settings**: Configurable Illuminant and Observer

## 📦 Installation

### Option 1: Install from Crates.io
```bash
cargo install spectro-gui
```

### Option 2: Download Pre-built Binary
Download the latest `spectro-gui.exe` from [GitHub Releases](https://github.com/Tinnci/spectro-rs/releases).

### Option 3: Build from Source
```bash
git clone https://github.com/Tinnci/spectro-rs.git
cd spectro-rs
cargo run -p spectro-gui
```

## 🚀 Usage

1. Connect your ColorMunki device
2. Launch `spectro-gui`
3. Select measurement mode (Reflective/Emissive/Ambient)
4. Click **Calibrate** (required for reflective mode)
5. Click **Measure** to capture spectrum

## ⚠️ Driver Setup

- **Windows**: Use [Zadig](https://zadig.akeo.ie/) to install the WinUSB driver if the device is not detected.
- **Linux**: Ensure proper udev rules are configured.

## 🔗 Related

- **[spectro-rs](https://crates.io/crates/spectro-rs)**: The underlying spectrometer library

## ⚖️ License

Licensed under the [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.html).
