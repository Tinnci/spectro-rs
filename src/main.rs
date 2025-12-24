mod munki;

use rusb::{Context, DeviceDescriptor, UsbContext};
// use std::time::Duration;
use crate::munki::Munki;

const XRITE_VID: u16 = 0x0765;
const GRETAG_VID: u16 = 0x0971;
const COLORMUNKI_OLD_PID: u16 = 0x2007;

fn main() -> rusb::Result<()> {
    let context = Context::new()?;
    let devices = context.devices()?;

    println!("Scanning for devices...");

    for device in devices.iter() {
        let device_desc = device.device_descriptor()?;
        let vid = device_desc.vendor_id();
        let pid = device_desc.product_id();

        if vid == XRITE_VID || vid == GRETAG_VID {
            println!(
                "Found X-Rite/Gretag device: VID: {:04x}, PID: {:04x}",
                vid, pid
            );

            if pid == COLORMUNKI_OLD_PID {
                println!("\x1b[32mTarget Device Found: ColorMunki (Original)\x1b[0m");
                println!("--- Pre-Open Diagnostics ---");
                println!(
                    "Bus: {:03}, Address: {:03}",
                    device.bus_number(),
                    device.address()
                );

                // Print Configuration Descriptors (Doesn't require opening the device)
                if let Ok(config) = device.config_descriptor(0) {
                    println!("Config 0 has {} interface(s)", config.num_interfaces());
                    for (i, interface) in config.interfaces().enumerate() {
                        println!("  Interface {}:", i);
                        for (j, desc) in interface.descriptors().enumerate() {
                            println!(
                                "    Alt Setting {}: Class: 0x{:02x}, SubClass: 0x{:02x}, Protocol: 0x{:02x}",
                                j,
                                desc.class_code(),
                                desc.sub_class_code(),
                                desc.protocol_code()
                            );
                            println!("    Number of Endpoints: {}", desc.num_endpoints());
                        }
                    }
                }

                attempt_munki_connect(device, device_desc)?;
            }
        }
    }

    Ok(())
}

fn attempt_munki_connect<T: UsbContext>(
    device: rusb::Device<T>,
    _desc: DeviceDescriptor,
) -> rusb::Result<()> {
    println!("Attempting to open device...");
    match device.open() {
        Ok(handle) => {
            println!("Device opened successfully!");

            // On Windows, we might need to detach kernel driver (not usually for WinUSB)
            // handle.set_auto_detach_kernel_driver(true)?;

            // Claim interface 0
            match handle.claim_interface(0) {
                Ok(_) => println!("Interface 0 claimed."),
                Err(e) => {
                    println!("\x1b[31mFailed to claim interface 0: {}\x1b[0m", e);
                    // Usually implies another driver has it or it's not configured
                }
            }

            let munki = Munki::new(handle);

            println!("--- Device Info ---");
            if let Ok(v) = munki.get_version_string() {
                println!("  - Version: {}", v);
            }

            let firmware_res = munki.get_firmware_info();
            if let Ok(info) = &firmware_res {
                info.print_details();
            }

            if let Ok(id) = munki.get_chip_id() {
                println!("  - Chip ID: {:02X?}", id);
            }

            if let Ok(status) = munki.get_status() {
                println!(
                    "  - Status: {} (Raw: {})",
                    status.position_name(),
                    status.sensor_position
                );
                println!(
                    "  - Button: {}",
                    if status.button_state == 0 {
                        "Released"
                    } else {
                        "Pressed"
                    }
                );
            }

            println!("--- EEPROM Data ---");
            match munki.get_calibration_size() {
                Ok(size) => {
                    println!("  - Calibration Data Size: {} bytes", size);
                    if size > 0 && size <= 16384 {
                        match munki.read_eeprom(0, size) {
                            Ok(data) => match munki.parse_eeprom(&data) {
                                Ok(config) => {
                                    println!("  - EEPROM Parsed Successfully!");
                                    println!("  - Serial Number: {}", config.serial_number);
                                    println!("  - Calibration Version: {}", config.cal_version);
                                    println!(
                                        "  - Normal Gain Linearization: {:?}",
                                        config.lin_normal
                                    );
                                    println!("  - High Gain Linearization: {:?}", config.lin_high);
                                    println!(
                                        "  - White Reference (first 5): {:?}",
                                        &config.white_ref[0..5]
                                    );

                                    println!("--- Measurement Test (Dark) ---");
                                    if let Ok(fw) = &firmware_res {
                                        let tick_sec = fw.tick_duration as f64 * 1e-6;
                                        let min_int_sec =
                                            (fw.min_int_count * fw.tick_duration) as f64 * 1e-6;
                                        println!(
                                            "  - Triggering measurement (Int: {}ms)...",
                                            min_int_sec * 1000.0
                                        );
                                        match munki.measure_spot(
                                            min_int_sec,
                                            tick_sec,
                                            false,
                                            false,
                                        ) {
                                            Ok(raw_data) => {
                                                println!("  - Measurement Successful!");
                                                println!(
                                                    "  - Raw spectral data (137 bands, first 10): {:?}",
                                                    &raw_data[0..10]
                                                );
                                                let sum: u32 =
                                                    raw_data.iter().map(|&x| x as u32).sum();
                                                println!(
                                                    "  - Data Sum: {} (Avg: {:.2})",
                                                    sum,
                                                    sum as f64 / 137.0
                                                );
                                            }
                                            Err(e) => println!("  - Measurement Failed: {}", e),
                                        }
                                    } else {
                                        println!(
                                            "  - Cannot perform measurement: Firmware info not available."
                                        );
                                    }
                                }
                                Err(e) => println!("  - EEPROM Parsing Failed: {}", e),
                            },
                            Err(e) => println!("  - Failed to read full EEPROM data: {}", e),
                        }
                    }
                }
                Err(e) => println!("  - Failed to read calibration size: {}", e),
            }
        }
        Err(e) => {
            println!("\x1b[31mCannot open device: {}\x1b[0m", e);
            if cfg!(target_os = "windows") {
                println!(
                    "Hint: Ensure you have installed a compatible driver (e.g. libusb-win32) using Zadig."
                );
                println!(
                    "Also ensure no other software (like ArgyllCMS or X-Rite software) is using the device."
                );
            }
        }
    }
    Ok(())
}
