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

/// CIE Standard Illuminants (2-degree observer, normalized Y=1.0).
pub mod illuminant {
    use super::XYZ;

    /// D50 (Horizon Light, Print Industry Standard)
    pub const D50: XYZ = XYZ {
        x: 0.96422,
        y: 1.0,
        z: 0.82521,
    };
    /// D55 (Mid-Morning / Mid-Afternoon Daylight)
    pub const D55: XYZ = XYZ {
        x: 0.95682,
        y: 1.0,
        z: 0.92149,
    };
    /// D65 (Noon Daylight, sRGB/Display Standard)
    pub const D65: XYZ = XYZ {
        x: 0.95047,
        y: 1.0,
        z: 1.08883,
    };
    /// D75 (North Sky Daylight)
    pub const D75: XYZ = XYZ {
        x: 0.94972,
        y: 1.0,
        z: 1.22638,
    };
    /// Illuminant A (Incandescent / Tungsten, 2856K)
    pub const A: XYZ = XYZ {
        x: 1.09850,
        y: 1.0,
        z: 0.35585,
    };
    /// F2 (Cool White Fluorescent)
    pub const F2: XYZ = XYZ {
        x: 0.99186,
        y: 1.0,
        z: 0.67393,
    };
    /// F7 (Daylight Fluorescent, D65 Simulator)
    pub const F7: XYZ = XYZ {
        x: 0.95041,
        y: 1.0,
        z: 1.08747,
    };
    /// F11 (Philips TL84, Narrow Band Cool White)
    pub const F11: XYZ = XYZ {
        x: 1.00962,
        y: 1.0,
        z: 0.64350,
    };

    // Legacy aliases for backward compatibility
    pub const D50_2: XYZ = D50;
    pub const D65_2: XYZ = D65;
}

/// Bradford chromatic adaptation transform.
/// Converts XYZ from one illuminant to another.
pub mod chromatic_adaptation {
    use super::XYZ;

    /// Apply Bradford transform to adapt XYZ from source to destination white point.
    pub fn bradford_adapt(xyz: XYZ, src_wp: XYZ, dst_wp: XYZ) -> XYZ {
        // Bradford M matrix (XYZ to LMS cone response)
        let m = [
            [0.8951, 0.2664, -0.1614],
            [-0.7502, 1.7135, 0.0367],
            [0.0389, -0.0685, 1.0296],
        ];
        // Inverse Bradford M matrix
        let m_inv = [
            [0.9869929, -0.1470543, 0.1599627],
            [0.4323053, 0.5183603, 0.0492912],
            [-0.0085287, 0.0400428, 0.9684867],
        ];

        // Convert to LMS
        let src_lms = [
            m[0][0] * src_wp.x + m[0][1] * src_wp.y + m[0][2] * src_wp.z,
            m[1][0] * src_wp.x + m[1][1] * src_wp.y + m[1][2] * src_wp.z,
            m[2][0] * src_wp.x + m[2][1] * src_wp.y + m[2][2] * src_wp.z,
        ];
        let dst_lms = [
            m[0][0] * dst_wp.x + m[0][1] * dst_wp.y + m[0][2] * dst_wp.z,
            m[1][0] * dst_wp.x + m[1][1] * dst_wp.y + m[1][2] * dst_wp.z,
            m[2][0] * dst_wp.x + m[2][1] * dst_wp.y + m[2][2] * dst_wp.z,
        ];

        // Scaling factors
        let scale = [
            dst_lms[0] / src_lms[0],
            dst_lms[1] / src_lms[1],
            dst_lms[2] / src_lms[2],
        ];

        // Convert input XYZ to LMS
        let lms = [
            m[0][0] * xyz.x + m[0][1] * xyz.y + m[0][2] * xyz.z,
            m[1][0] * xyz.x + m[1][1] * xyz.y + m[1][2] * xyz.z,
            m[2][0] * xyz.x + m[2][1] * xyz.y + m[2][2] * xyz.z,
        ];

        // Scale LMS
        let lms_adapted = [lms[0] * scale[0], lms[1] * scale[1], lms[2] * scale[2]];

        // Convert back to XYZ
        XYZ {
            x: m_inv[0][0] * lms_adapted[0]
                + m_inv[0][1] * lms_adapted[1]
                + m_inv[0][2] * lms_adapted[2],
            y: m_inv[1][0] * lms_adapted[0]
                + m_inv[1][1] * lms_adapted[1]
                + m_inv[1][2] * lms_adapted[2],
            z: m_inv[2][0] * lms_adapted[0]
                + m_inv[2][1] * lms_adapted[1]
                + m_inv[2][2] * lms_adapted[2],
        }
    }
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
    /// Convert XYZ to CIE L*a*b* using the given white point.
    /// Uses precise CIE constants for continuity at the threshold.
    pub fn to_lab(&self, wp: XYZ) -> Lab {
        // CIE standard constants for continuity
        const EPSILON: f32 = 216.0 / 24389.0; // ≈ 0.008856
        const KAPPA: f32 = 24389.0 / 27.0; // ≈ 903.2963

        let f = |t: f32| -> f32 {
            if t > EPSILON {
                t.powf(1.0 / 3.0)
            } else {
                (KAPPA * t + 16.0) / 116.0
            }
        };

        let fx = f(self.x / wp.x);
        let fy = f(self.y / wp.y);
        let fz = f(self.z / wp.z);

        Lab {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }

    /// Convert XYZ (D65) to linear sRGB, then apply gamma correction.
    /// Returns clamped (r, g, b) values in [0, 255].
    #[allow(clippy::excessive_precision)]
    pub fn to_srgb(&self) -> (u8, u8, u8) {
        // XYZ to linear sRGB matrix (IEC 61966-2-1, D65 reference)
        let r_lin = 3.2404542 * self.x - 1.5371385 * self.y - 0.4985314 * self.z;
        let g_lin = -0.9692660 * self.x + 1.8760108 * self.y + 0.0415560 * self.z;
        let b_lin = 0.0556434 * self.x - 0.2040259 * self.y + 1.0572252 * self.z;

        // Gamma correction (sRGB companding)
        fn gamma(c: f32) -> f32 {
            if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        }

        let r = (gamma(r_lin).clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (gamma(g_lin).clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (gamma(b_lin).clamp(0.0, 1.0) * 255.0).round() as u8;

        (r, g, b)
    }
}

impl Lab {
    /// Convert Lab back to XYZ using the given white point.
    /// Uses precise CIE constants for continuity at the threshold.
    pub fn to_xyz(&self, wp: XYZ) -> XYZ {
        // CIE standard constants for continuity
        const EPSILON: f32 = 216.0 / 24389.0; // ≈ 0.008856
        const KAPPA: f32 = 24389.0 / 27.0; // ≈ 903.2963

        let fy = (self.l + 16.0) / 116.0;
        let fx = self.a / 500.0 + fy;
        let fz = fy - self.b / 200.0;

        let f_inv = |t: f32| -> f32 {
            let t3 = t.powi(3);
            if t3 > EPSILON {
                t3
            } else {
                (116.0 * t - 16.0) / KAPPA
            }
        };

        XYZ {
            x: wp.x * f_inv(fx),
            y: wp.y * f_inv(fy),
            z: wp.z * f_inv(fz),
        }
    }

    /// Convert Lab to sRGB via XYZ (using D65 white point).
    /// Returns clamped (r, g, b) values in [0, 255].
    pub fn to_srgb(&self) -> (u8, u8, u8) {
        self.to_xyz(illuminant::D65_2).to_srgb()
    }

    /// Calculates Delta E*ab (CIE 1976).
    pub fn delta_e_76(&self, other: &Lab) -> f32 {
        ((self.l - other.l).powi(2) + (self.a - other.a).powi(2) + (self.b - other.b).powi(2))
            .sqrt()
    }

    /// Calculates Delta E*00 (CIE 2000) using the Sharma (2005) reference implementation.
    /// This is the industry standard for perceptual color difference.
    pub fn delta_e_2000(&self, other: &Lab) -> f32 {
        // Weight factors (default = 1.0)
        let k_l = 1.0;
        let k_c = 1.0;
        let k_h = 1.0;

        let c1 = (self.a.powi(2) + self.b.powi(2)).sqrt();
        let c2 = (other.a.powi(2) + other.b.powi(2)).sqrt();
        let avg_c = (c1 + c2) / 2.0;

        // Calculate G and adjusted a'
        let g = 0.5 * (1.0 - (avg_c.powi(7) / (avg_c.powi(7) + 25.0f32.powi(7))).sqrt());
        let a1p = (1.0 + g) * self.a;
        let a2p = (1.0 + g) * other.a;

        let c1p = (a1p.powi(2) + self.b.powi(2)).sqrt();
        let c2p = (a2p.powi(2) + other.b.powi(2)).sqrt();

        // Key fix: atan2(b, a') - correct parameter order
        let get_hp = |b: f32, ap: f32| -> f32 {
            if b == 0.0 && ap == 0.0 {
                0.0
            } else {
                let h = b.atan2(ap).to_degrees();
                if h < 0.0 {
                    h + 360.0
                } else {
                    h
                }
            }
        };
        let h1p = get_hp(self.b, a1p);
        let h2p = get_hp(other.b, a2p);

        // Calculate delta values
        let d_lp = other.l - self.l;
        let d_cp = c2p - c1p;

        let mut d_hp_deg = h2p - h1p;
        if c1p * c2p != 0.0 {
            if d_hp_deg.abs() > 180.0 {
                if h2p <= h1p {
                    d_hp_deg += 360.0;
                } else {
                    d_hp_deg -= 360.0;
                }
            }
        } else {
            d_hp_deg = 0.0;
        }
        let d_hp = 2.0 * (c1p * c2p).sqrt() * (d_hp_deg / 2.0).to_radians().sin();

        // Calculate averages
        let avg_lp = (self.l + other.l) / 2.0;
        let avg_cp = (c1p + c2p) / 2.0;

        let mut avg_hp = h1p + h2p;
        if c1p * c2p != 0.0 {
            if (h1p - h2p).abs() > 180.0 {
                if h1p + h2p < 360.0 {
                    avg_hp += 360.0;
                } else {
                    avg_hp -= 360.0;
                }
            }
            avg_hp /= 2.0;
        } else {
            avg_hp = h1p + h2p;
        }

        // T term
        let t = 1.0 - 0.17 * (avg_hp - 30.0).to_radians().cos()
            + 0.24 * (2.0 * avg_hp).to_radians().cos()
            + 0.32 * (3.0 * avg_hp + 6.0).to_radians().cos()
            - 0.20 * (4.0 * avg_hp - 63.0).to_radians().cos();

        // Key fix: SL denominator structure
        let s_l = 1.0 + (0.015 * (avg_lp - 50.0).powi(2)) / (20.0 + (avg_lp - 50.0).powi(2)).sqrt();
        let s_c = 1.0 + 0.045 * avg_cp;
        let s_h = 1.0 + 0.015 * avg_cp * t;

        // Rotation term RT
        let d_theta = 30.0 * (-((avg_hp - 275.0) / 25.0).powi(2)).exp();
        let rc = 2.0 * (avg_cp.powi(7) / (avg_cp.powi(7) + 25.0f32.powi(7))).sqrt();
        let rt = -rc * (2.0 * d_theta.to_radians()).sin();

        // Final Delta E 00
        ((d_lp / (k_l * s_l)).powi(2)
            + (d_cp / (k_c * s_c)).powi(2)
            + (d_hp / (k_h * s_h)).powi(2)
            + rt * (d_cp / (k_c * s_c)) * (d_hp / (k_h * s_h)))
            .sqrt()
    }

    /// Mix two Lab colors by a given ratio (0.0 = self, 1.0 = other).
    pub fn mix(&self, other: &Lab, ratio: f32) -> Lab {
        let ratio = ratio.clamp(0.0, 1.0);
        Lab {
            l: self.l * (1.0 - ratio) + other.l * ratio,
            a: self.a * (1.0 - ratio) + other.a * ratio,
            b: self.b * (1.0 - ratio) + other.b * ratio,
        }
    }

    /// Calculate chroma (C*) from a* and b*.
    pub fn chroma(&self) -> f32 {
        (self.a.powi(2) + self.b.powi(2)).sqrt()
    }

    /// Calculate hue angle (h°) in degrees [0, 360).
    pub fn hue(&self) -> f32 {
        let h = self.b.atan2(self.a).to_degrees();
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    }
}

/// Color appearance and analysis utilities.
pub mod appearance {
    use super::{illuminant, Lab, XYZ};
    use crate::spectrum::SpectralData;

    /// Calculate Metamerism Index between two spectral samples.
    /// Compares how differently the samples appear under a test illuminant
    /// relative to a reference illuminant (typically D65).
    ///
    /// A high metamerism index means the samples look similar under one illuminant
    /// but different under another — a common issue in color matching.
    pub fn metamerism_index(
        sample1: &SpectralData,
        sample2: &SpectralData,
        ref_illuminant: XYZ,
        test_illuminant: XYZ,
    ) -> f32 {
        // Calculate Lab under reference illuminant
        let xyz1_ref = sample1.to_xyz();
        let xyz2_ref = sample2.to_xyz();
        let lab1_ref = XYZ {
            x: xyz1_ref.x / 100.0,
            y: xyz1_ref.y / 100.0,
            z: xyz1_ref.z / 100.0,
        }
        .to_lab(ref_illuminant);
        let lab2_ref = XYZ {
            x: xyz2_ref.x / 100.0,
            y: xyz2_ref.y / 100.0,
            z: xyz2_ref.z / 100.0,
        }
        .to_lab(ref_illuminant);

        // Adapt XYZ to test illuminant using Bradford
        let xyz1_test =
            super::chromatic_adaptation::bradford_adapt(xyz1_ref, illuminant::D65, test_illuminant);
        let xyz2_test =
            super::chromatic_adaptation::bradford_adapt(xyz2_ref, illuminant::D65, test_illuminant);

        let lab1_test = XYZ {
            x: xyz1_test.x / 100.0,
            y: xyz1_test.y / 100.0,
            z: xyz1_test.z / 100.0,
        }
        .to_lab(test_illuminant);
        let lab2_test = XYZ {
            x: xyz2_test.x / 100.0,
            y: xyz2_test.y / 100.0,
            z: xyz2_test.z / 100.0,
        }
        .to_lab(test_illuminant);

        // Delta E under reference
        let de_ref = lab1_ref.delta_e_2000(&lab2_ref);
        // Delta E under test
        let de_test = lab1_test.delta_e_2000(&lab2_test);

        // Metamerism index is the difference in color difference
        (de_test - de_ref).abs()
    }

    /// Simulate how a color appears under a different illuminant.
    /// Uses Bradford chromatic adaptation.
    pub fn simulate_illuminant(lab: &Lab, from: XYZ, to: XYZ) -> Lab {
        let xyz = lab.to_xyz(from);
        let adapted = super::chromatic_adaptation::bradford_adapt(xyz, from, to);
        adapted.to_lab(to)
    }
}
