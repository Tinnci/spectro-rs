use dialoguer::{Select, theme::ColorfulTheme};
use rusb::{Context, UsbContext};
use spectro_rs::munki::Munki;
use spectro_rs::{MeasurementMode, Result, i18n, t};

fn main() -> Result<()> {
    i18n::init_i18n();
    println!("{}", t!("welcome"));

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
    let fw = munki.get_firmware_info()?;
    let size = munki.get_calibration_size()?;
    let data = munki.read_eeprom(0, size)?;
    let config = munki.parse_eeprom(&data)?;
    munki.set_config(config);

    let tick_sec = fw.tick_duration as f64 * 1e-6;
    let min_int_sec = (fw.min_int_count * fw.tick_duration) as f64 * 1e-6;

    loop {
        let selections = &[
            t!("menu-measure").to_string(),
            t!("menu-measure-emissive").to_string(),
            t!("menu-measure-ambient").to_string(),
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
            0 | 1 | 2 => {
                let mode = match selection {
                    0 => MeasurementMode::Reflective,
                    1 => MeasurementMode::Emissive,
                    _ => MeasurementMode::Ambient,
                };

                if mode == MeasurementMode::Ambient {
                    let status = munki.get_status()?;
                    if status.sensor_position != 1 {
                        println!(
                            "\n\x1b[33m[Notice]\x1b[0m Please turn the dial to the \x1b[1mAmbient/Diffuser\x1b[0m position."
                        );
                    }
                }

                if mode == MeasurementMode::Reflective && munki.white_cal_factors.is_none() {
                    println!("\n\x1b[31m[Warning]\x1b[0m Reflective mode needs calibration first.");
                }

                let (lamp, high_gain) = if mode == MeasurementMode::Reflective {
                    (true, false)
                } else {
                    (false, mode == MeasurementMode::Emissive)
                };

                match munki.measure_spot(min_int_sec, tick_sec, lamp, high_gain) {
                    Ok(raw_data) => {
                        match munki.process_spectrum(&raw_data, min_int_sec, high_gain, mode) {
                            Ok(spec) => {
                                println!("\n\x1b[32m{}\x1b[0m", t!("spectral-success"));

                                // Colorimetry
                                let mut norm_xyz = spec.to_xyz();

                                // Apply scaling for absolute modes (Emissive/Ambient)
                                if mode != MeasurementMode::Reflective {
                                    norm_xyz.x *= 0.00025;
                                    norm_xyz.y *= 0.00025;
                                    norm_xyz.z *= 0.00025;
                                }

                                // Reference White (D50)
                                let wp = spectro_rs::colorimetry::XYZ {
                                    x: 96.42,
                                    y: 100.0,
                                    z: 82.49,
                                };
                                let lab = norm_xyz.to_lab(wp);

                                if mode == MeasurementMode::Emissive {
                                    println!(
                                        "\x1b[36mMonitor Mode:\x1b[0m Screen brightness (nits): {:.2} cd/m²",
                                        norm_xyz.y
                                    );
                                } else if mode == MeasurementMode::Ambient {
                                    println!(
                                        "\x1b[36mAmbient Mode:\x1b[0m Lighting intensity (relative): {:.2}",
                                        norm_xyz.y
                                    );
                                }

                                println!(
                                    "\x1b[33mCIE XYZ:\x1b[0m X:{:.2}, Y:{:.2}, Z:{:.2}",
                                    norm_xyz.x, norm_xyz.y, norm_xyz.z
                                );
                                let (x_coord, y_coord) = norm_xyz.to_chromaticity();
                                println!(
                                    "\x1b[33mChromaticity:\x1b[0m x:{:.4}, y:{:.4}",
                                    x_coord, y_coord
                                );
                                println!(
                                    "\x1b[35mCIE L*a*b*:\x1b[0m L:{:.2}, a:{:.2}, b:{:.2}\n",
                                    lab.l, lab.a, lab.b
                                );

                                // Advanced spectral analysis for light sources
                                if mode != MeasurementMode::Reflective {
                                    let cct = norm_xyz.to_cct();
                                    println!("\x1b[36mEstimated CCT:\x1b[0m {:.0} K", cct);

                                    // Spectral Centroid (weighted average wavelength)
                                    let total_power: f32 = spec.values.iter().skip(4).sum();
                                    let centroid: f32 = spec
                                        .values
                                        .iter()
                                        .enumerate()
                                        .skip(4) // Start from 420nm
                                        .map(|(i, v)| (380 + i * 10) as f32 * v)
                                        .sum::<f32>()
                                        / total_power.max(1e-6);
                                    println!(
                                        "\x1b[36mSpectral Centroid:\x1b[0m {:.1} nm",
                                        centroid
                                    );

                                    // Peak detection (skip noise below 420nm)
                                    let peak_idx = spec
                                        .values
                                        .iter()
                                        .enumerate()
                                        .skip(4)
                                        .max_by(|a, b| {
                                            a.1.partial_cmp(b.1)
                                                .unwrap_or(std::cmp::Ordering::Equal)
                                        })
                                        .map(|(i, _)| i)
                                        .unwrap_or(0);
                                    println!(
                                        "\x1b[36mPeak Wavelength:\x1b[0m {} nm",
                                        380 + peak_idx * 10
                                    );

                                    // Simple ASCII spectrum visualization
                                    println!("\n\x1b[90mSpectrum (420-730nm):\x1b[0m");
                                    let max_val =
                                        spec.values.iter().skip(4).cloned().fold(0.0f32, f32::max);
                                    for (i, v) in spec.values.iter().enumerate().skip(4) {
                                        let bar_len = ((v / max_val.max(1e-6)) * 30.0) as usize;
                                        let wl = 380 + i * 10;
                                        let color = match wl {
                                            420..=450 => "\x1b[34m",       // Blue
                                            451..=500 => "\x1b[36m",       // Cyan
                                            501..=560 => "\x1b[32m",       // Green
                                            561..=590 => "\x1b[33m",       // Yellow
                                            591..=620 => "\x1b[38;5;208m", // Orange
                                            _ => "\x1b[31m",               // Red
                                        };
                                        println!(
                                            "{:3}nm {}{}█{}\x1b[0m",
                                            wl,
                                            "\x1b[90m",
                                            color,
                                            "█".repeat(bar_len.min(30))
                                        );
                                    }
                                    println!();
                                }
                            }
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    Err(e) => println!("Measurement failed: {}", e),
                }
            }
            3 => {
                // Calibrate
                println!("\n{}", t!("calibration-required"));
                println!("{}", t!("dial-white-dot"));
                println!(
                    "(This position is light-tight, perfect for both Dark and White calibration)"
                );
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
            4 => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}
