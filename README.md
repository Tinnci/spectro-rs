# spectro-rs

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg?style=flat-square)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-GPLv3-blue.svg?style=flat-square)](https://www.gnu.org/licenses/gpl-3.0)
[![Crates.io](https://img.shields.io/crates/v/spectro-core.svg?style=flat-square)](https://crates.io/crates/spectro-core)
[![Docs.rs](https://docs.rs/spectro-core/badge.svg?style=flat-square)](https://docs.rs/spectro-core)
[![Build Status](https://github.com/Tinnci/spectro-rs/actions/workflows/ci.yml/badge.svg?style=flat-square)](https://github.com/Tinnci/spectro-rs/actions)

[中文文档 (Traditional Technical Style)](./README_zh.md)

`spectro-rs` is a high-performance Rust implementation of driver logic and measurement algorithms for X-Rite ColorMunki spectrometers. The project provides a safe, low-latency interface for spectral data acquisition, enabling precise colorimetry, display calibration, and ambient light analysis across Windows, macOS, and Linux environments.

---

## Table of Contents
- [Core Functionality](#core-functionality)
- [Installation and Setup](#installation-and-setup)
- [Operational Procedures](#operational-procedures)
- [Technical Specifications](#technical-specifications)
- [High-Precision Spectral Algorithms](#high-precision-spectral-algorithms)
- [Architecture](#architecture)
- [Development and Maintenance](#development-and-maintenance)
- [License](#license)

---

## Core Functionality

**Cross-platform Support:** Full hardware abstraction layer for Windows, macOS, and Linux via standard USB communication protocols. Includes **Native CJK font support** by dynamically loading system fonts (PingFang, YaHei, Noto Sans CJK).

**Multimodal Measurement Capabilities:**
- **Reflective Mode:** Implements automated dark-current and white-tile reference calibration for surface colorimetry.
- **Emissive Mode:** Utilizes optimized spectral transformation matrices for high-accuracy display and monitor characterization.
- **Ambient Mode:** Acquisition of Spectral Power Distribution (SPD) using the integrated diffuser dome.

**Advanced Colorimetry Engine:**
- **Real-time Computation:** Deterministic calculation of CIE XYZ, Chromaticity (x, y), and CIE L*a*b* coordinates.
- **Spectral Estimation:** Automated derivation of Correlated Color Temperature (CCT) and Spectral Centroid.
- **Standardized Profiles:** Support for industry-standard Illuminants (D65, D50, A, F-series) and Observer functions (CIE 2°, 10°).

---

## Installation and Setup

### Prerequisites
A functioning [Rust toolchain](https://rust-lang.org) (Stable or Nightly) is required for compilation.

### Build and Execution
Integrate the suite by cloning the repository and utilizing the Cargo workspace handlers:

```bash
git clone https://github.com/Tinnci/spectro-rs.git
cd spectro-rs
```

**Command Line Interface (CLI):** Optimized for automation and headless environments.
```bash
cargo run -p spectro-core
```

**Graphical User Interface (GUI):** Advanced visualization suite for interactive spectral analysis.
```bash
cargo run -p spectro-gui
```

*Note: On Windows systems, the generic `WinUSB` driver must be assigned to the device via [Zadig](https://zadig.akeo.ie/) if the hardware is not natively addressed.*

---

## Operational Procedures

### Calibration Protocol
To maintain measurement integrity, a calibration sequence is mandatory before each session:
1. Rotate the device dial to the **Reference Position (White Dot / Position 2)**.
2. The core driver executes a dual-phase calibration: Dark Frame acquisition (sensor noise baseline) followed by White Tile normalization.

### Measurement Execution
- **Display Emissive:** Position the dial at **Position 4** and secure the device against the target surface.
- **Ambient Light:** Position the dial at **Position 1** with the diffuser dome engaged.

---

## Technical Specifications

The implementation is derived from the established logic of the **ArgyllCMS** project, with specific enhancements for the Rust ownership model:
- **EEPROM Serialization:** Full mapping of hardware-stored linearization polynomials and factory-shipped correction matrices.
- **Spectral Mapping:** High-fidelity transposition of 128 raw sensor bins to 36 standardized 10nm spectral bands (380nm to 730nm).
- **Performance:** Zero-cost abstractions ensure minimal overhead during high-frequency spectral sampling.
- **Reference Accuracy:** Achieves reference-grade white point accuracy (e.g., $L^* > 96$, $a^* \approx 0$, $b^* \approx 0$) on standard tiles.

---

## High-Precision Spectral Algorithms

`spectro-rs` includes a custom digital signal processing (DSP) suite tailored for aging hardware recovery and scientific precision:

- **Hardware-Level Oversampling:** Implements 8x multi-frame averaging to minimize shot noise and electronic jitter, providing a 2.8x SNR improvement.
- **High Dynamic Range (HDR) Exposure:** Forces high sensor counts (10,500+) and minimum integration time (0.015s) to maximize signal in the low-sensitivity UV and Red spectral tails.
- **Signal-Gated Dynamic Extrapolation:** Automatically detects the valid sensor band range using a dynamic peak-based threshold (25%), filtering out noisy dark zones.
- **Stable Anchor Selection:** Advanced "interior-step" anchor logic (e.g., using `first + 1` bands) to avoid unstable LED emission slopes, eliminating false yellow/blue color casts.
- **Spectral Smoothing:** Integrated 3-point Boxcar filter `[0.25, 0.5, 0.25]` applied post-extrapolation for maximum colorimetric stability and repeatability.
- **Physics-Based Linearizer:** Corrects sensor non-linearity using a robust dead-zone and saturation-shaping model.

---

## Architecture

- **`crates/spectro-core`:** The foundational driver library. Manages low-level USB I/O, EEPROM parsing, and the core mathematical engine.
- **`crates/spectro-gui`:** A front-end implementation utilizing the `egui` framework for real-time data visualization.

---

## Development and Maintenance

### CI/CD Pipeline
Continuous Integration is managed via GitHub Actions, enforcing strict linting (`clippy`) and unit testing on every push to the `main` branch. Automated deployment to `crates.io` is triggered by semantic version tagging.

### Local Compliance
Maintainers are required to install the provided pre-commit hooks to ensure code style consistency:
```bash
pre-commit install
```

---

## License

This project is licensed under the **[GNU General Public License v3.0](https://www.gnu.org/licenses/gpl-3.0.html)**.

---

## Maintainers

Current project maintenance is handled by me. For bug reports or feature expansion requests regarding new spectrophotometer hardware, please utilize the GitHub Issue tracker.
