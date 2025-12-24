use rusb::{Context, UsbContext};
use spectro_rs::Result;
use spectro_rs::munki::Munki;

fn main() -> Result<()> {
    let context = Context::new()?;
    let devices = context.devices()?;

    println!("Scanning for devices...");

    for device in devices.iter() {
        let device_desc = device.device_descriptor()?;
        let vid = device_desc.vendor_id();
        let pid = device_desc.product_id();

        if (vid == 0x0765 || vid == 0x0971) && pid == 0x2007 {
            println!("\x1b[32mTarget Device Found: ColorMunki (Original)\x1b[0m");

            let handle = device.open()?;
            handle.claim_interface(0)?;

            let mut munki = Munki::new(handle);

            println!("--- Device Info ---");
            if let Ok(v) = munki.get_version_string() {
                println!("  - Version: {}", v);
            }

            let fw = munki.get_firmware_info()?;
            println!(
                "  - Firmware Revision: {}.{}",
                fw.fw_rev_major, fw.fw_rev_minor
            );

            if let Ok(status) = munki.get_status() {
                println!(
                    "  - Status: {} (Raw: {})",
                    status.position_name(),
                    status.sensor_position
                );
            }

            println!("--- EEPROM Data ---");
            let size = munki.get_calibration_size()?;
            let data = munki.read_eeprom(0, size)?;
            let config = munki.parse_eeprom(&data)?;
            println!("  - Serial Number: {}", config.serial_number);

            munki.set_config(config);

            println!("\x1b[33m--- Calibration Required ---\x1b[0m");
            println!("  1. Turn the dial to the \x1b[1mWhite Dot\x1b[0m.");
            println!("  2. Press Enter to start...");
            let mut input = String::new();
            let _ = std::io::stdin().read_line(&mut input);

            let tick_sec = fw.tick_duration as f64 * 1e-6;
            let min_int_sec = (fw.min_int_count * fw.tick_duration) as f64 * 1e-6;

            println!("  - Step 1/2: Dark Frame subtraction...");
            if let Ok(raw_dark) = munki.measure_spot(min_int_sec, tick_sec, false, false) {
                munki.dark_ref = Some(raw_dark);
            }

            println!("  - Step 2/2: White tile calibration...");
            munki
                .compute_white_calibration(min_int_sec, tick_sec)
                .map_err(|e| e)?;

            println!("\x1b[32m  Calibration Success!\x1b[0m");

            println!("--- Measurement Test (Calibrated) ---");
            let raw_data = munki.measure_spot(min_int_sec, tick_sec, true, false)?;
            let spec = munki.process_spectrum(&raw_data, min_int_sec, false)?;
            println!("{}", spec);

            return Ok(());
        }
    }

    println!("No ColorMunki found.");
    Ok(())
}
