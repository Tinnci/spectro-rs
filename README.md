# spectro-rs

A Rust-based driver for color measurement instruments, ported from ArgyllCMS.

## Current Progress & Status

### ColorMunki (Original) - Phase 2 Complete
- [x] **Device Discovery**: Successfully identifies X-Rite/GretagMacbeth devices (VID: `0x0971`, PID: `0x2007`).
- [x] **USB Communication**: Stable connection using `rusb` (WinUSB/UsbDk recommended on Windows).
- [x] **Firmware Info**: Correctly retrieves version string and firmware parameters.
- [x] **EEPROM Parsing**: 
    - Full Checksum validation (32-bit).
    - Extraction of Calibration Matrices (36 wavelengths).
    - Linearization parameters (Normal/High Gain).
    - Reference coefficients (White, Emission, Ambient).

## Usage
Ensure your device is connected and the appropriate driver (WinUSB) is installed via Zadig.

```bash
cargo run
```

## Upcoming Goals
- [ ] **Phase 4**: Spectral measurement implementation.
- [ ] **Phase 5**: Integration-time optimization and black level compensation.
