use std::io::{Cursor, Write};

/// A simple ICC Matrix-Shaper profile generator.
/// Supports creating V2.4 display profiles with a single gamma value.
pub struct IccProfile {
    pub description: String,
    pub white_point: [f32; 3],   // XYZ
    pub red_primary: [f32; 3],   // XYZ
    pub green_primary: [f32; 3], // XYZ
    pub blue_primary: [f32; 3],  // XYZ
    pub gamma: f32,
}

impl IccProfile {
    pub fn new_srgb_like(description: &str) -> Self {
        Self {
            description: description.to_string(),
            white_point: [0.9642, 1.0, 0.8249], // D50
            red_primary: [0.4360657, 0.2224884, 0.0139160],
            green_primary: [0.3851471, 0.7168732, 0.0970764],
            blue_primary: [0.1430664, 0.0606079, 0.7140961],
            gamma: 2.2,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());

        // Placeholder for size
        buf.write_all(&[0u8; 4]).unwrap();

        // Header (128 bytes)
        buf.write_all(b"scrs").unwrap(); // CMM Type
        buf.write_all(&[0x02, 0x40, 0x00, 0x00]).unwrap(); // Version 2.4
        buf.write_all(b"mntr").unwrap(); // Device Class
        buf.write_all(b"RGB ").unwrap(); // Color Space
        buf.write_all(b"XYZ ").unwrap(); // PCS

        // Date/Time (placeholder)
        buf.write_all(&[0u8; 12]).unwrap();

        buf.write_all(b"acsp").unwrap(); // Magic
        buf.write_all(b"APPL").unwrap(); // Platform
        buf.write_all(&[0u8; 4]).unwrap(); // Flags
        buf.write_all(b"none").unwrap(); // Manufacturer
        buf.write_all(b"none").unwrap(); // Model
        buf.write_all(&[0u8; 8]).unwrap(); // Attributes
        buf.write_all(&[0u8; 4]).unwrap(); // Rendering Intent

        // Illuminant (D50)
        write_s15fixed16(&mut buf, 0.9642);
        write_s15fixed16(&mut buf, 1.0);
        write_s15fixed16(&mut buf, 0.8249);

        buf.write_all(b"scrs").unwrap(); // Creator
        buf.write_all(&[0u8; 44]).unwrap(); // Reserved

        // Tag Table
        let tags = [
            (b"desc", self.encode_desc()),
            (b"wtpt", self.encode_xyz(self.white_point)),
            (b"rXYZ", self.encode_xyz(self.red_primary)),
            (b"gXYZ", self.encode_xyz(self.green_primary)),
            (b"bXYZ", self.encode_xyz(self.blue_primary)),
            (b"rTRC", self.encode_trc()),
            (b"gTRC", self.encode_trc()),
            (b"bTRC", self.encode_trc()),
            (b"cprt", self.encode_text("Copyright (c) 2025 spectro-rs")),
        ];

        let tag_count = tags.len() as u32;
        buf.write_all(&tag_count.to_be_bytes()).unwrap();

        let mut offset = 128 + 4 + tags.len() as u32 * 12;
        for (sig, data) in &tags {
            buf.write_all(*sig).unwrap();
            buf.write_all(&offset.to_be_bytes()).unwrap();
            buf.write_all(&(data.len() as u32).to_be_bytes()).unwrap();
            offset += data.len() as u32;
        }

        // Tag Data
        for (_, data) in &tags {
            buf.write_all(data).unwrap();
        }

        let mut result = buf.into_inner();
        let size = result.len() as u32;
        result[0..4].copy_from_slice(&size.to_be_bytes());
        result
    }

    fn encode_desc(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_all(b"desc").unwrap();
        buf.write_all(&[0u8; 4]).unwrap();
        let len = (self.description.len() + 1) as u32;
        buf.write_all(&len.to_be_bytes()).unwrap();
        buf.write_all(self.description.as_bytes()).unwrap();
        buf.push(0); // Null terminator
                     // Padding for other fields in desc tag (V2)
        buf.write_all(&[0u8; 67]).unwrap();
        buf
    }

    fn encode_text(&self, text: &str) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_all(b"text").unwrap();
        buf.write_all(&[0u8; 4]).unwrap();
        buf.write_all(text.as_bytes()).unwrap();
        buf.push(0);
        buf
    }

    fn encode_xyz(&self, xyz: [f32; 3]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_all(b"XYZ ").unwrap();
        buf.write_all(&[0u8; 4]).unwrap();
        write_s15fixed16(&mut buf, xyz[0]);
        write_s15fixed16(&mut buf, xyz[1]);
        write_s15fixed16(&mut buf, xyz[2]);
        buf
    }

    fn encode_trc(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_all(b"curv").unwrap();
        buf.write_all(&[0u8; 4]).unwrap();
        buf.write_all(&1u32.to_be_bytes()).unwrap(); // 1 entry for gamma
        let g = (self.gamma * 256.0) as u16;
        buf.write_all(&g.to_be_bytes()).unwrap();
        buf.write_all(&[0u8; 2]).unwrap(); // Padding
        buf
    }
}

fn write_s15fixed16<W: Write>(w: &mut W, val: f32) {
    let fixed = (val * 65536.0) as i32;
    w.write_all(&fixed.to_be_bytes()).unwrap();
}
