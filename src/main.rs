use dialoguer::{Select, theme::ColorfulTheme};
use rusb::{Context, UsbContext};
use spectro_rs::munki::Munki;
use spectro_rs::{MeasurementMode, Result, i18n, t};

fn main() -> Result<()> {
    // 1. Initialize i18n
    i18n::init_i18n();
    println!("{}", t!("welcome"));

    // 2. Scan for devices
    let context = Context::new()?;
    let devices = context.devices()?;

    println!("{}", t!("scanning"));

    let mut found_device = None;
    for device in devices.iter() {
        let device_desc = device.device_descriptor()?;
        let vid = device_desc.vendor_id();
        let pid = device_desc.product_id();

        if (vid == 0x0765 || vid == 0x0971) && pid == 0x2007 {
            found_device = Some(device);
            break;
        }
    }

    let device = found_device.ok_or_else(|| {
        println!("{}", t!("no-device"));
        spectro_rs::SpectroError::Device("No device found".into())
    })?;

    println!("{}", t!("target-found"));

    let handle = device.open()?;
    handle.claim_interface(0)?;

    let mut munki = Munki::new(handle);

    // Initial Setup
    let fw = munki.get_firmware_info()?;
    let size = munki.get_calibration_size()?;
    let data = munki.read_eeprom(0, size)?;
    let config = munki.parse_eeprom(&data)?;
    munki.set_config(config);

    let tick_sec = fw.tick_duration as f64 * 1e-6;
    let min_int_sec = (fw.min_int_count * fw.tick_duration) as f64 * 1e-6;

    // Interactive Loop
    loop {
        let selections = &[
            t!("menu-measure").to_string(),
            t!("menu-measure-emissive").to_string(),
            t!("menu-calibrate").to_string(),
            t!("menu-exit").to_string(),
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(t!("menu-title").to_string())
            .default(0)
            .items(&selections[..])
            .interact()
            .unwrap();

        match selection {
            0 | 1 => {
                let mode = if selection == 0 {
                    MeasurementMode::Reflective
                } else {
                    MeasurementMode::Emissive
                };

                // Warning for calibration
                if mode == MeasurementMode::Reflective && munki.white_cal_factors.is_none() {
                    println!("\n\x1b[31m[Warning]\x1b[0m Reflective mode needs calibration.");
                }

                let (lamp, high_gain) = if mode == MeasurementMode::Reflective {
                    (true, false)
                } else {
                    (false, true)
                };

                // Measure
                match munki.measure_spot(min_int_sec, tick_sec, lamp, high_gain) {
                    Ok(raw_data) => {
                        match munki.process_spectrum(&raw_data, min_int_sec, high_gain, mode) {
                            Ok(spec) => {
                                println!("\n\x1b[32m{}\x1b[0m", t!("spectral-success"));

                                // Colorimetry
                                let norm_xyz = spec.to_xyz();
                                // D50 White point
                                let wp = if mode == MeasurementMode::Reflective {
                                    spectro_rs::colorimetry::XYZ {
                                        x: 96.42,
                                        y: 100.0,
                                        z: 82.49,
                                    }
                                } else {
                                    // For Emissive, users often use D65 or D50. We'll stick to D50 for now or add D65 later.
                                    spectro_rs::colorimetry::XYZ {
                                        x: 96.42,
                                        y: 100.0,
                                        z: 82.49,
                                    }
                                };

                                let lab = norm_xyz.to_lab(wp);

                                println!(
                                    "\x1b[33mCIE XYZ:\x1b[0m X:{:.2}, Y:{:.2}, Z:{:.2}",
                                    norm_xyz.x, norm_xyz.y, norm_xyz.z
                                );
                                println!(
                                    "\x1b[35mCIE L*a*b*:\x1b[0m L:{:.2}, a:{:.2}, b:{:.2}\n",
                                    lab.l, lab.a, lab.b
                                );
                            }
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    Err(e) => println!("Measurement failed: {}", e),
                }
            }
            2 => {
                // Calibrate
                println!("\n{}", t!("calibration-required"));
                println!("{}", t!("dial-white-dot"));
                println!("{}", t!("press-enter"));
                let mut input = String::new();
                let _ = std::io::stdin().read_line(&mut input);

                println!("{}", t!("step-dark"));
                if let Ok(raw_dark) = munki.measure_spot(min_int_sec, tick_sec, false, false) {
                    munki.dark_ref = Some(raw_dark);
                }

                println!("{}", t!("step-white"));
                match munki.compute_white_calibration(min_int_sec, tick_sec) {
                    Ok(_) => println!("\x1b[32m{}\x1b[0m\n", t!("cal-success")),
                    Err(e) => println!("\x1b[31mError: {}\x1b[0m\n", e),
                }
            }
            3 => break,
            _ => unreachable!(),
        }
    }

    Ok(())
}
