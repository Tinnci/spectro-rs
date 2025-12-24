# spectro-rs

A high-performance Rust driver for X-Rite ColorMunki spectrometers.

## Features
- [x] USB Communication (rusb)
- [x] EEPROM Parsing & Checksum Validation
- [x] Spectral Measurement (Reflective)
- [x] **Interactive Calibration (Dark/White)**
- [x] **Multi-language CLI (EN/ZH)**
- [x] **Colorimetry Engine (XYZ, Lab)**
- [x] **Emissive Mode Support (Monitor measurement)**

## Usage
```bash
cargo run
```

## Progress
- [x] Phase 1-4: Core hardware communication.
- [x] Phase 5: Calibration & Matrix Processing.
- [x] Phase 6: Interactive Menu & Color Computation.
- [wip] Phase 7: Verification & Display measurement testing.

## Credits
Based on the logic from ArgyllCMS.
