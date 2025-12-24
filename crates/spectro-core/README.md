# 🌈 spectro-rs

[![Crates.io](https://img.shields.io/crates/v/spectro-rs.svg)](https://crates.io/crates/spectro-rs)
[![Docs.rs](https://docs.rs/spectro-rs/badge.svg)](https://docs.rs/spectro-rs)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

A high-performance Rust driver for **X-Rite ColorMunki** spectrometers. This library provides a safe, cross-platform interface for color measurement, display calibration, and spectral analysis.

## ✨ Features

- **Cross-platform**: Windows, macOS, and Linux support
- **Multi-mode Measurement**: Reflective, Emissive (Monitor), and Ambient modes
- **Colorimetry Engine**: CIE XYZ, L\*a\*b\*, CCT, and Delta E (76/2000) calculations
- **Calibration Persistence**: Automatically saves and loads calibration data
- **Internationalization**: Built-in English and Chinese support

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
spectro-rs = "0.1"
```

## 🚀 Quick Start

```rust
use spectro_rs::{discover, MeasurementMode};

fn main() -> spectro_rs::Result<()> {
    // Auto-discover connected spectrometer
    let mut device = discover()?;
    
    // Print device info
    let info = device.info()?;
    println!("Found: {} ({})", info.model, info.serial);
    
    // Calibrate (required for reflective mode)
    device.calibrate()?;
    
    // Take a measurement
    let spectrum = device.measure(MeasurementMode::Reflective)?;
    
    // Convert to colorimetric values
    let xyz = spectrum.to_xyz();
    let lab = xyz.to_lab(spectro_rs::colorimetry::illuminant::D65_2);
    
    println!("L*={:.2}, a*={:.2}, b*={:.2}", lab.l, lab.a, lab.b);
    Ok(())
}
```

## 📖 Documentation

Full API documentation is available on [docs.rs](https://docs.rs/spectro-rs).

## ⚠️ Driver Setup

- **Windows**: Use [Zadig](https://zadig.akeo.ie/) to install the WinUSB driver.
- **Linux**: Add appropriate udev rules for USB access.

## 🔗 Related

- **[spectro-gui](https://crates.io/crates/spectro-gui)**: Graphical interface for this library

## ⚖️ License

Licensed under the [GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.html).
