use crate::shared::{DeviceCommand, ExtendedDeviceInfo, UIUpdate};
use crate::t;
use crossbeam_channel::{Receiver, Sender};
use spectro_rs::{BoxedSpectrometer, MeasurementMode, discover, tm30::calculate_tm30};
use std::thread;

pub fn spawn_backend_thread(cmd_rx: Receiver<DeviceCommand>, update_tx: Sender<UIUpdate>) {
    thread::spawn(move || {
        let mut device: Option<BoxedSpectrometer> = None;

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                DeviceCommand::Connect => {
                    update_tx
                        .send(UIUpdate::Status("🔍 Searching for device...".into()))
                        .ok();

                    match discover() {
                        Ok(d) => {
                            let basic_info = d.info().ok();
                            let ext_info = ExtendedDeviceInfo {
                                basic: basic_info,
                                cal_version: Some(0x0100),
                                white_ref: None,
                                emis_coef: None,
                                amb_coef: None,
                                lin_normal: None,
                                lin_high: None,
                            };

                            device = Some(d);
                            update_tx.send(UIUpdate::Connected(ext_info)).ok();
                            update_tx
                                .send(UIUpdate::Status(t!("gui-status-connected")))
                                .ok();
                        }
                        Err(_e) => {
                            update_tx
                                .send(UIUpdate::Error(t!("gui-error-no-device")))
                                .ok();
                        }
                    }
                }

                DeviceCommand::Calibrate => {
                    if let Some(ref mut d) = device {
                        update_tx
                            .send(UIUpdate::Status(t!("gui-status-calibrating")))
                            .ok();

                        match d.calibrate() {
                            Ok(_) => {
                                update_tx
                                    .send(UIUpdate::Status(t!("gui-status-calibration-ok")))
                                    .ok();
                            }
                            Err(e) => {
                                update_tx.send(UIUpdate::Error(e.to_string())).ok();
                            }
                        }
                    } else {
                        update_tx
                            .send(UIUpdate::Error(t!("gui-error-no-device-short")))
                            .ok();
                    }
                }

                DeviceCommand::Measure(mode) => {
                    if let Some(ref mut d) = device {
                        update_tx
                            .send(UIUpdate::Status(t!("gui-status-measuring")))
                            .ok();

                        match d.measure(mode) {
                            Ok(data) => {
                                let tm30 = if mode == MeasurementMode::Emissive {
                                    Some(Box::new(calculate_tm30(&data)))
                                } else {
                                    None
                                };
                                let result = data.to_result();
                                update_tx.send(UIUpdate::Result(result, tm30)).ok();
                                update_tx
                                    .send(UIUpdate::Status("✅ Measurement complete".into()))
                                    .ok();
                            }
                            Err(e) => {
                                let err_str = format!("{}", e);
                                if err_str.contains("USB") || err_str.contains("timeout") {
                                    device = None;
                                    update_tx.send(UIUpdate::Disconnected).ok();
                                }
                                update_tx.send(UIUpdate::Error(err_str)).ok();
                            }
                        }
                    } else {
                        update_tx
                            .send(UIUpdate::Error(t!("gui-error-no-device-short")))
                            .ok();
                    }
                }

                DeviceCommand::TestSensor => {
                    if let Some(ref mut d) = device {
                        update_tx
                            .send(UIUpdate::Status("Diagnostic Running...".into()))
                            .ok();

                        match d.test_sensor() {
                            Ok(report) => {
                                update_tx.send(UIUpdate::TestResult(report)).ok();
                                update_tx
                                    .send(UIUpdate::Status("✅ Diagnostic Complete".into()))
                                    .ok();
                            }
                            Err(e) => {
                                update_tx.send(UIUpdate::Error(e.to_string())).ok();
                            }
                        }
                    } else {
                        update_tx
                            .send(UIUpdate::Error(t!("gui-error-no-device-short")))
                            .ok();
                    }
                }
            }
        }
    });
}
