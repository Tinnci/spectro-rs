//! ColorMunki Spectrometer CLI Application.
//!
//! This is the interactive command-line interface for the spectro-rs library.

use dialoguer::{theme::ColorfulTheme, Select};
use spectro_rs::{
    colorimetry::XYZ, device::DevicePosition, discover, i18n, t, MeasurementMode, Result,
};

fn main() -> Result<()> {
    i18n::init_i18n();

    // --- Original CLI Logic ---
    println!("{}", t!("welcome"));
    println!("{}", t!("scanning"));

    // Use the simplified discovery API
    let mut device = match discover() {
        Ok(dev) => dev,
        Err(e) => {
            println!("{}", t!("no-device"));
            return Err(e);
        }
    };

    // Print device info
    let info = device.info()?;
    println!("\n\x1b[32m{}\x1b[0m", t!("target-found"));
    println!("  Model: {}", info.model);
    println!("  Serial: {}", info.serial);
    println!("  Firmware: {}", info.firmware);

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
            0..=2 => {
                let mode = match selection {
                    0 => MeasurementMode::Reflective,
                    1 => MeasurementMode::Emissive,
                    _ => MeasurementMode::Ambient,
                };

                // Check dial position for ambient mode
                if mode == MeasurementMode::Ambient {
                    let status = device.status()?;
                    if status.position != DevicePosition::Ambient
                        && status.position != DevicePosition::Surface
                    {
                        println!(
                            "\n\x1b[33m[Notice]\x1b[0m Please turn the dial to the \x1b[1mAmbient/Diffuser\x1b[0m position."
                        );
                    }
                }

                // Check calibration for reflective mode
                if mode == MeasurementMode::Reflective && !device.is_calibrated(mode) {
                    println!("\n\x1b[31m[Warning]\x1b[0m Reflective mode needs calibration first.");
                    continue;
                }

                match device.measure(mode) {
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
                        let wp = XYZ {
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
                            println!("\x1b[36mSpectral Centroid:\x1b[0m {:.1} nm", centroid);

                            // Peak detection (skip noise below 420nm)
                            let peak_idx = spec
                                .values
                                .iter()
                                .enumerate()
                                .skip(4)
                                .max_by(|a, b| {
                                    a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                                })
                                .map(|(i, _)| i)
                                .unwrap_or(0);
                            println!("\x1b[36mPeak Wavelength:\x1b[0m {} nm", 380 + peak_idx * 10);

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
                                    "{:3}nm \x1b[90m{}█{}\x1b[0m",
                                    wl,
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
                println!("{}", t!("step-white"));

                match device.calibrate() {
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
