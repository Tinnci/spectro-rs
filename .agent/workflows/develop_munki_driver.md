---
description: Develop Rust driver for ColorMunki
---

# Rust ColorMunki Driver Development Workflow

This workflow outlines the steps to develop a Rust-based driver for the X-Rite ColorMunki, mirroring the functionality of ArgyllCMS.

## 1. Project Setup
- [x] Create `spectro-rs` project.
- [x] Add dependencies (`rusb`, `hidapi`, `hex`).
- [x] Implement device scanning.

## 2. Low-Level Communication (Current Focus)
- [ ] Implement `Munki` struct in `src/munki.rs`.
- [ ] Implement Control Transfer wrappers (`read_control`, `write_control`).
- [ ] Implement `get_version_string` (Cmd 0x85).
- [ ] Implement `get_serial_number` (Cmd 0x8F ? No, 0x8F is MeaState).
- [ ] Implement `get_firmware_info` (Cmd 0x86).
- [ ] Implement `get_chip_id` (Cmd 0x8A).
- [ ] Implement `get_status` (Cmd 0x87).

## 3. Initialization & Unlock
- [ ] Port `munki_imp_init` logic.
- [ ] Implement EEPROM reading (Cmd 0x81 w/ 0x40 write).
- [ ] Parse EEPROM calibration data.

## 4. Measurement
- [ ] Implement measurement trigger and read loops.
- [ ] specific measurement commands.

## 5. Troubleshooting "Cannot Open"
- [ ] Verify `libusb-win32` driver installation using Zadig if necessary.
- [ ] Ensure no other software (Argyll, X-Rite) is using the device.

## 6. Integration
- [ ] Update `main.rs` to open device and print debug info.
