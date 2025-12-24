---
description: Develop Rust driver for ColorMunki
---

# Rust ColorMunki Driver Development Workflow

This workflow outlines the steps to develop a Rust-based driver for the X-Rite ColorMunki, mirroring the functionality of ArgyllCMS.

## 1. Project Setup
- [x] Create workspace with `spectro-rs` and `spectro-gui`.
- [x] Add dependencies (`rusb`, `hidapi`, `hex`, `egui`).
- [x] Implement device scanning.

## 2. Low-Level Communication
- [x] Implement `Munki` struct in `crates/spectro-rs/src/munki.rs`.
- [x] Implement Control Transfer wrappers (`read_control`, `write_control`).
- [x] Implement `get_version_string` (Cmd 0x85).
- [x] Implement `get_serial_number`.
- [x] Implement `get_firmware_info` (Cmd 0x86).
- [x] Implement `get_chip_id` (Cmd 0x8A).
- [x] Implement `get_status` (Cmd 0x87).

## 3. Initialization & Calibration
- [x] Port `munki_imp_init` logic.
- [x] Implement EEPROM reading and parsing.
- [x] Implement Dark/White calibration.
- [x] Implement calibration data persistence (json).

## 4. Colorimetry & Analysis
- [x] Implement XYZ and Lab conversion.
- [x] Implement 2-degree and 10-degree observers.
- [x] Implement CIEDE2000 and DeltaE76.
- [x] Implement CCT and Spectral Analysis (Peak/Centroid).
- [x] Implement Lab to sRGB conversion for UI.

## 5. GUI Integration
- [x] Create `spectro-gui` crate with `egui`.
- [x] Implement Real-time spectral plot.
- [x] Implement Device Worker thread for non-blocking UI.
- [x] Implement Measurement mode selection.

## 6. Infrastructure
- [x] Set up Cargo Workspace.
- [x] GitHub Actions CI for all workspace members.
- [x] GitHub Actions CD (Crates.io + Releases).
- [x] Multi-token security configuration.
