/// CIE 1931 2-degree Standard Observer CMFs (380-730nm, 10nm steps)
pub const X_BAR_2: [f32; 36] = [
    0.0014, 0.0042, 0.0143, 0.0435, 0.1344, 0.2839, 0.3483, 0.3362, 0.2908, 0.1954, 0.0956, 0.0320,
    0.0049, 0.0093, 0.0633, 0.1655, 0.2904, 0.4334, 0.5945, 0.7621, 0.9163, 1.0263, 1.0622, 1.0026,
    0.8524, 0.6424, 0.4479, 0.2835, 0.1649, 0.0874, 0.0468, 0.0227, 0.0114, 0.0058, 0.0029, 0.0014,
];

pub const Y_BAR_2: [f32; 36] = [
    0.0000, 0.0001, 0.0004, 0.0012, 0.0040, 0.0116, 0.0230, 0.0380, 0.0600, 0.0910, 0.1390, 0.2080,
    0.3230, 0.5030, 0.7100, 0.8620, 0.9540, 0.9950, 0.9950, 0.9520, 0.8700, 0.7570, 0.6310, 0.5030,
    0.3810, 0.2650, 0.1750, 0.1070, 0.0610, 0.0320, 0.0170, 0.0082, 0.0041, 0.0021, 0.0010, 0.0005,
];

pub const Z_BAR_2: [f32; 36] = [
    0.0065, 0.0201, 0.0679, 0.2074, 0.6456, 1.3856, 1.7471, 1.7721, 1.5794, 1.1143, 0.5701, 0.1970,
    0.0415, 0.0052, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000,
    0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000,
];

/// CIE 1964 10-degree Standard Observer CMFs (380-730nm, 10nm steps)
#[allow(clippy::approx_constant)]
pub const X_BAR_10: [f32; 36] = [
    0.0002, 0.0011, 0.0061, 0.0315, 0.1241, 0.3023, 0.5045, 0.6931, 0.8177, 0.7530, 0.5314, 0.3345,
    0.1570, 0.0538, 0.0331, 0.1117, 0.2230, 0.4243, 0.6627, 0.8690, 1.0107, 1.0743, 1.0257, 0.8724,
    0.6553, 0.4456, 0.2800, 0.1622, 0.0869, 0.0434, 0.0218, 0.0107, 0.0053, 0.0026, 0.0013, 0.0006,
];

pub const Y_BAR_10: [f32; 36] = [
    0.0000, 0.0000, 0.0002, 0.0010, 0.0041, 0.0105, 0.0207, 0.0407, 0.0702, 0.1120, 0.1852, 0.2904,
    0.4190, 0.5764, 0.7435, 0.8872, 0.9666, 0.9983, 0.9873, 0.9331, 0.8420, 0.7163, 0.5596, 0.4203,
    0.3021, 0.2003, 0.1245, 0.0713, 0.0380, 0.0189, 0.0094, 0.0046, 0.0023, 0.0011, 0.0006, 0.0003,
];

pub const Z_BAR_10: [f32; 36] = [
    0.0007, 0.0045, 0.0259, 0.1343, 0.5285, 1.3003, 2.1932, 3.0334, 3.5534, 3.2392, 2.2235, 1.3400,
    0.5752, 0.1866, 0.0427, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000,
    0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000, 0.0000,
];

/// CIE Standard Illuminants.
pub mod illuminant {
    use super::XYZ;
    /// D50 White Point (2-degree, normalized Y=1.0)
    pub const D50_2: XYZ = XYZ {
        x: 0.9642,
        y: 1.0,
        z: 0.8251,
    };
    /// D65 White Point (2-degree, normalized Y=1.0)
    pub const D65_2: XYZ = XYZ {
        x: 0.9504,
        y: 1.0,
        z: 1.0888,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct XYZ {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl XYZ {
    pub fn to_lab(&self, wp: XYZ) -> Lab {
        fn f(t: f32) -> f32 {
            if t > 0.008856 {
                t.powf(1.0 / 3.0)
            } else {
                7.787 * t + 16.0 / 116.0
            }
        }

        let fx = f(self.x / wp.x);
        let fy = f(self.y / wp.y);
        let fz = f(self.z / wp.z);

        Lab {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }
}

impl Lab {
    /// Calculates Delta E*ab (CIE 1976).
    pub fn delta_e_76(&self, other: &Lab) -> f32 {
        ((self.l - other.l).powi(2) + (self.a - other.a).powi(2) + (self.b - other.b).powi(2))
            .sqrt()
    }

    /// Calculates Delta E*00 (CIE 2000).
    pub fn delta_e_2000(&self, other: &Lab) -> f32 {
        let l1 = self.l;
        let l2 = other.l;
        let a1 = self.a;
        let a2 = other.a;
        let b1 = self.b;
        let b2 = other.b;

        let avg_l = (l1 + l2) / 2.0;
        let c1 = (a1.powi(2) + b1.powi(2)).sqrt();
        let c2 = (a2.powi(2) + b2.powi(2)).sqrt();
        let avg_c = (c1 + c2) / 2.0;

        let g = 0.5 * (1.0 - (avg_c.powi(7) / (avg_c.powi(7) + 25.0f32.powi(7))).sqrt());
        let a1p = (1.0 + g) * a1;
        let a2p = (1.0 + g) * a2;
        let c1p = (a1p.powi(2) + b1.powi(2)).sqrt();
        let c2p = (a2p.powi(2) + b2.powi(2)).sqrt();
        let avg_cp = (c1p + c2p) / 2.0;

        let h1p = a1p.atan2(b1).to_degrees();
        let h1p = if h1p < 0.0 { h1p + 360.0 } else { h1p };
        let h2p = a2p.atan2(b2).to_degrees();
        let h2p = if h2p < 0.0 { h2p + 360.0 } else { h2p };

        let mut d_hp = h2p - h1p;
        if d_hp.abs() > 180.0 {
            if h2p <= h1p {
                d_hp += 360.0;
            } else {
                d_hp -= 360.0;
            }
        }

        let d_lp = l2 - l1;
        let d_cp = c2p - c1p;
        let d_hp = 2.0 * (c1p * c2p).sqrt() * (d_hp.to_radians() / 2.0).sin();

        let mut avg_hp = h1p + h2p;
        if (h1p - h2p).abs() > 180.0 {
            if avg_hp < 360.0 {
                avg_hp += 360.0;
            } else {
                avg_hp -= 360.0;
            }
        }
        avg_hp /= 2.0;

        let t = 1.0 - 0.17 * (avg_hp - 30.0).to_radians().cos()
            + 0.24 * (2.0 * avg_hp).to_radians().cos()
            + 0.32 * (3.0 * avg_hp + 6.0).to_radians().cos()
            - 0.20 * (4.0 * avg_hp - 63.0).to_radians().cos();

        let sl = 1.0 + (0.015 * (avg_l - 50.0).powi(2)) / (20.0 + (avg_l - 50.0).powi(2)).sqrt();
        let sc = 1.0 + 0.045 * avg_cp;
        let sh = 1.0 + 0.015 * avg_cp * t;

        let d_theta = 30.0 * (-((avg_hp - 275.0) / 25.0).powi(2)).exp();
        let rc = 2.0 * (avg_cp.powi(7) / (avg_cp.powi(7) + 25.0f32.powi(7))).sqrt();
        let rt = -rc * (2.0 * d_theta.to_radians()).sin();

        ((d_lp / sl).powi(2)
            + (d_cp / sc).powi(2)
            + (d_hp / sh).powi(2)
            + rt * (d_cp / sc) * (d_hp / sh))
            .sqrt()
    }
}
