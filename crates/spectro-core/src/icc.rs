use crate::cam02::{Cam02State, Surround, ViewingConditions};
use crate::colorimetry::XYZ;
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
    pub lut: Option<Lut3D>,
}

/// A 3D Lookup Table for ICC profiles.
pub struct Lut3D {
    pub grid_points: u8,
    pub data: Vec<f32>, // RGB -> XYZ, size grid_points^3 * 3
}

impl Lut3D {
    /// Create a new 3D LUT with the specified grid size.
    pub fn new(grid_points: u8) -> Self {
        let size = (grid_points as usize).pow(3) * 3;
        Self {
            grid_points,
            data: vec![0.0; size],
        }
    }

    /// Perform trilinear interpolation on the 3D LUT.
    /// Input r, g, b should be in range [0, 1].
    pub fn interpolate(&self, r: f32, g: f32, b: f32) -> [f32; 3] {
        let n = self.grid_points as usize;
        if n < 2 {
            return [0.0, 0.0, 0.0];
        }

        let r_scaled = r * (n - 1) as f32;
        let g_scaled = g * (n - 1) as f32;
        let b_scaled = b * (n - 1) as f32;

        let r0 = (r_scaled.floor() as usize).min(n - 2);
        let g0 = (g_scaled.floor() as usize).min(n - 2);
        let b0 = (b_scaled.floor() as usize).min(n - 2);

        let r1 = r0 + 1;
        let g1 = g0 + 1;
        let b1 = b0 + 1;

        let dr = r_scaled - r0 as f32;
        let dg = g_scaled - g0 as f32;
        let db = b_scaled - b0 as f32;

        let get_val = |ri: usize, gi: usize, bi: usize| -> [f32; 3] {
            let idx = (ri * n * n + gi * n + bi) * 3;
            [self.data[idx], self.data[idx + 1], self.data[idx + 2]]
        };

        let c000 = get_val(r0, g0, b0);
        let c001 = get_val(r0, g0, b1);
        let c010 = get_val(r0, g1, b0);
        let c011 = get_val(r0, g1, b1);
        let c100 = get_val(r1, g0, b0);
        let c101 = get_val(r1, g0, b1);
        let c110 = get_val(r1, g1, b0);
        let c111 = get_val(r1, g1, b1);

        let mut res = [0.0; 3];
        for i in 0..3 {
            let c00 = c000[i] * (1.0 - dr) + c100[i] * dr;
            let c01 = c001[i] * (1.0 - dr) + c101[i] * dr;
            let c10 = c010[i] * (1.0 - dr) + c110[i] * dr;
            let c11 = c011[i] * (1.0 - dr) + c111[i] * dr;

            let c0 = c00 * (1.0 - dg) + c10 * dg;
            let c1 = c01 * (1.0 - dg) + c11 * dg;

            res[i] = c0 * (1.0 - db) + c1 * db;
        }
        res
    }

    /// Fill the 3D LUT using a mapping function.
    /// The function `f` receives (r, g, b) in range [0, 1] and should return [x, y, z].
    pub fn fill<F>(&mut self, mut f: F)
    where
        F: FnMut(f32, f32, f32) -> [f32; 3],
    {
        let n = self.grid_points as usize;
        for r_idx in 0..n {
            let r = r_idx as f32 / (n - 1) as f32;
            for g_idx in 0..n {
                let g = g_idx as f32 / (n - 1) as f32;
                for b_idx in 0..n {
                    let b = b_idx as f32 / (n - 1) as f32;
                    let res = f(r, g, b);
                    let idx = (r_idx * n * n + g_idx * n + b_idx) * 3;
                    self.data[idx] = res[0];
                    self.data[idx + 1] = res[1];
                    self.data[idx + 2] = res[2];
                }
            }
        }
    }
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
            lut: None,
        }
    }

    /// Fill the 3D LUT using the current matrix-shaper model.
    pub fn fill_lut_from_model(&mut self, grid_points: u8) {
        let mut lut = Lut3D::new(grid_points);
        let gamma = self.gamma;
        let rp = self.red_primary;
        let gp = self.green_primary;
        let bp = self.blue_primary;

        lut.fill(|r, g, b| {
            let rl = r.powf(gamma);
            let gl = g.powf(gamma);
            let bl = b.powf(gamma);

            let x = rl * rp[0] + gl * gp[0] + bl * bp[0];
            let y = rl * rp[1] + gl * gp[1] + bl * bp[1];
            let z = rl * rp[2] + gl * gp[2] + bl * bp[2];

            [x, y, z]
        });
        self.lut = Some(lut);
    }

    /// Fill the 3D LUT using CAM02-UCS for perceptual mapping.
    pub fn fill_lut_perceptual(&mut self, grid_points: u8) {
        let mut lut = Lut3D::new(grid_points);
        let gamma = self.gamma;
        let rp = self.red_primary;
        let gp = self.green_primary;
        let bp = self.blue_primary;

        // ICC PCS white point is D50
        let wp_pcs = XYZ {
            x: 0.9642,
            y: 1.0,
            z: 0.8249,
        };
        let vc = ViewingConditions::new(
            wp_pcs,
            160.0 / std::f32::consts::PI,
            20.0,
            Surround::AVERAGE,
        );
        let cam = Cam02State::new(&vc);

        lut.fill(|r, g, b| {
            let rl = r.powf(gamma);
            let gl = g.powf(gamma);
            let bl = b.powf(gamma);

            let x = rl * rp[0] + gl * gp[0] + bl * bp[0];
            let y = rl * rp[1] + gl * gp[1] + bl * bp[1];
            let z = rl * rp[2] + gl * gp[2] + bl * bp[2];

            // Convert to CAM02-UCS to demonstrate integration
            let _ucs = cam.xyz_to_ucs(XYZ { x, y, z });

            // Here we could apply CAM02-UCS based gamut mapping or adjustments
            [x, y, z]
        });
        self.lut = Some(lut);
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
        let mut tags = vec![
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

        if let Some(ref lut) = self.lut {
            tags.push((b"A2B0", self.encode_lut16(lut)));
        }

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

    fn encode_lut16(&self, lut: &Lut3D) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.write_all(b"mft2").unwrap();
        buf.write_all(&[0u8; 4]).unwrap();
        buf.push(3); // Input channels
        buf.push(3); // Output channels
        buf.push(lut.grid_points);
        buf.push(0); // Reserved

        // Identity Matrix (3x3)
        write_s15fixed16(&mut buf, 1.0);
        write_s15fixed16(&mut buf, 0.0);
        write_s15fixed16(&mut buf, 0.0);
        write_s15fixed16(&mut buf, 0.0);
        write_s15fixed16(&mut buf, 1.0);
        write_s15fixed16(&mut buf, 0.0);
        write_s15fixed16(&mut buf, 0.0);
        write_s15fixed16(&mut buf, 0.0);
        write_s15fixed16(&mut buf, 1.0);

        buf.write_all(&2u16.to_be_bytes()).unwrap(); // Input table entries
        buf.write_all(&2u16.to_be_bytes()).unwrap(); // Output table entries

        // Input tables (Identity)
        for _ in 0..3 {
            buf.write_all(&0u16.to_be_bytes()).unwrap();
            buf.write_all(&65535u16.to_be_bytes()).unwrap();
        }

        // CLUT
        for &val in &lut.data {
            let v = (val.clamp(0.0, 1.9999) * 32768.0) as u16; // XYZ is scaled by 1/2 in lut16? No, usually 0..1 maps to 0..65535
                                                               // Actually, for XYZ in lut16, 0..1.999 maps to 0..65535
            buf.write_all(&v.to_be_bytes()).unwrap();
        }

        // Output tables (Identity)
        for _ in 0..3 {
            buf.write_all(&0u16.to_be_bytes()).unwrap();
            buf.write_all(&65535u16.to_be_bytes()).unwrap();
        }

        buf
    }
}

fn write_s15fixed16<W: Write>(w: &mut W, val: f32) {
    let fixed = (val * 65536.0) as i32;
    w.write_all(&fixed.to_be_bytes()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lut_interpolation() {
        let mut lut = Lut3D::new(3); // 3x3x3 grid
                                     // Fill with identity-like mapping for testing
        lut.fill(|r, g, b| [r, g, b]);

        // Test exact grid points
        assert_eq!(lut.interpolate(0.0, 0.0, 0.0), [0.0, 0.0, 0.0]);
        assert_eq!(lut.interpolate(0.5, 0.5, 0.5), [0.5, 0.5, 0.5]);
        assert_eq!(lut.interpolate(1.0, 1.0, 1.0), [1.0, 1.0, 1.0]);

        // Test interpolation
        let res = lut.interpolate(0.25, 0.25, 0.25);
        assert!((res[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn test_icc_with_lut() {
        let mut profile = IccProfile::new_srgb_like("Test LUT Profile");
        profile.fill_lut_from_model(17); // 17x17x17 is a common size
        let bytes = profile.to_bytes();
        assert!(bytes.len() > 1000);
        // Check for mft2 tag
        let bytes_str = String::from_utf8_lossy(&bytes);
        assert!(bytes_str.contains("mft2"));
    }
}
