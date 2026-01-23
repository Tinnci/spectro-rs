use crate::shared::MeasurementEntry;
use std::path::Path;

/// Single responsibility trait for exporting history
pub trait HistoryExporter {
    fn name(&self) -> &str;
    fn extensions(&self) -> Vec<&str>;
    fn default_filename(&self) -> &str;
    fn export(&self, history: &[MeasurementEntry], path: &Path) -> std::io::Result<()>;
}

pub struct CsvExporter;
pub struct JsonExporter;
pub struct CgatsExporter;

impl HistoryExporter for CsvExporter {
    fn name(&self) -> &str {
        "CSV File"
    }

    fn extensions(&self) -> Vec<&str> {
        vec!["csv"]
    }

    fn default_filename(&self) -> &str {
        "measurements.csv"
    }

    fn export(&self, history: &[MeasurementEntry], path: &Path) -> std::io::Result<()> {
        let mut csv = String::from("Timestamp,Mode,L*,a*,b*,DeltaE\n");
        for entry in history {
            csv.push_str(&format!(
                "{},{:?},{:.4},{:.4},{:.4},{}\n",
                entry.timestamp,
                entry.mode,
                entry.result.lab.l,
                entry.result.lab.a,
                entry.result.lab.b,
                entry.delta_e.map(|e| e.to_string()).unwrap_or_default()
            ));
        }
        std::fs::write(path, csv)
    }
}

impl HistoryExporter for JsonExporter {
    fn name(&self) -> &str {
        "JSON File"
    }

    fn extensions(&self) -> Vec<&str> {
        vec!["json"]
    }

    fn default_filename(&self) -> &str {
        "measurements.json"
    }

    fn export(&self, history: &[MeasurementEntry], path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(history).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

impl HistoryExporter for CgatsExporter {
    fn name(&self) -> &str {
        "CGATS File"
    }

    fn extensions(&self) -> Vec<&str> {
        vec!["ti3", "txt"]
    }

    fn default_filename(&self) -> &str {
        "measurements.ti3"
    }

    fn export(&self, history: &[MeasurementEntry], path: &Path) -> std::io::Result<()> {
        let mut cgats = String::new();
        cgats.push_str("CTI3\n\n");
        cgats.push_str("DESCRIPTOR \"Argyll Device Measurement data\"\n");
        cgats.push_str("ORIGINATOR \"spectro-rs\"\n");
        cgats.push_str(&format!(
            "CREATED \"{}\"\n\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));

        // Define fields: ID, Lab, XYZ, and Spectral data
        cgats.push_str("NUMBER_OF_FIELDS 47\n");
        cgats.push_str("BEGIN_DATA_FORMAT\n");
        cgats.push_str("SAMPLE_ID SAMPLE_NAME LAB_L LAB_A LAB_B XYZ_X XYZ_Y XYZ_Z ");
        for wl in (380..=780).step_by(10) {
            cgats.push_str(&format!("SPEC_{} ", wl));
        }
        cgats.push_str("\nEND_DATA_FORMAT\n\n");

        cgats.push_str(&format!("NUMBER_OF_SETS {}\n", history.len()));
        cgats.push_str("BEGIN_DATA\n");

        for (i, entry) in history.iter().enumerate() {
            cgats.push_str(&format!(
                "{} \"{}\" {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} ",
                i + 1,
                entry.timestamp,
                entry.result.lab.l,
                entry.result.lab.a,
                entry.result.lab.b,
                entry.result.xyz.x,
                entry.result.xyz.y,
                entry.result.xyz.z
            ));

            for val in &entry.result.spectrum.values {
                cgats.push_str(&format!("{:.6} ", val));
            }
            cgats.push('\n');
        }

        cgats.push_str("END_DATA\n");
        std::fs::write(path, cgats)
    }
}
