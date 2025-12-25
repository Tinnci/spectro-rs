//! CIECAM02 Color Appearance Model and CAM02-UCS Uniform Color Space.
//!
//! Used by TM-30-18 for perceptual color difference calculations.

use crate::colorimetry::XYZ;

/// Viewing conditions for CIECAM02.
#[derive(Debug, Clone, Copy)]
pub struct ViewingConditions {
    /// Adapting luminance in cd/m^2 (default: 100.0 / PI)
    pub la: f32,
    /// Relative luminance of background (default: 20.0)
    pub yb: f32,
    /// White point of the adapting source
    pub wp: XYZ,
    /// Surround parameters (default: Average)
    pub surround: Surround,
}

#[derive(Debug, Clone, Copy)]
pub struct Surround {
    pub f: f32,
    pub c: f32,
    pub nc: f32,
}

impl Surround {
    pub const AVERAGE: Self = Self {
        f: 1.0,
        c: 0.69,
        nc: 1.0,
    };
    pub const DIM: Self = Self {
        f: 0.9,
        c: 0.59,
        nc: 0.9,
    };
    pub const DARK: Self = Self {
        f: 0.8,
        c: 0.525,
        nc: 0.8,
    };
}

impl ViewingConditions {
    pub fn new(wp: XYZ, la: f32, yb: f32, surround: Surround) -> Self {
        Self {
            wp,
            la,
            yb,
            surround,
        }
    }
}

impl Default for ViewingConditions {
    fn default() -> Self {
        Self {
            la: 100.0 / std::f32::consts::PI,
            yb: 20.0,
            wp: XYZ {
                x: 0.95047,
                y: 1.0,
                z: 1.08883,
            }, // D65
            surround: Surround::AVERAGE,
        }
    }
}

/// CAM02-UCS (Uniform Color Space) coordinates.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cam02Ucs {
    pub j_prime: f32,
    pub a_prime: f32,
    pub b_prime: f32,
}

impl Cam02Ucs {
    pub fn distance(&self, other: &Self) -> f32 {
        ((self.j_prime - other.j_prime).powi(2)
            + (self.a_prime - other.a_prime).powi(2)
            + (self.b_prime - other.b_prime).powi(2))
        .sqrt()
    }

    pub fn h(&self) -> f32 {
        let h = self.b_prime.atan2(self.a_prime).to_degrees();
        if h < 0.0 {
            h + 360.0
        } else {
            h
        }
    }

    /// Simple gamut mapping: if a color is outside the target gamut,
    /// move it towards the neutral axis (a'=0, b'=0) until it's inside.
    /// This is a very basic "Chroma Clipping" strategy in CAM02-UCS.
    pub fn clip_to_gamut<F>(&self, mut is_in_gamut: F) -> Self
    where
        F: FnMut(f32, f32, f32) -> bool,
    {
        if is_in_gamut(self.j_prime, self.a_prime, self.b_prime) {
            return *self;
        }

        let mut low = 0.0;
        let mut high = 1.0;
        let mut best_a = 0.0;
        let mut best_b = 0.0;

        // Binary search for the maximum chroma that stays in gamut
        for _ in 0..10 {
            let mid = (low + high) / 2.0;
            let test_a = self.a_prime * mid;
            let test_b = self.b_prime * mid;
            if is_in_gamut(self.j_prime, test_a, test_b) {
                best_a = test_a;
                best_b = test_b;
                low = mid;
            } else {
                high = mid;
            }
        }

        Self {
            j_prime: self.j_prime,
            a_prime: best_a,
            b_prime: best_b,
        }
    }
}

/// Internal state for CIECAM02 calculations derived from viewing conditions.
pub struct Cam02State {
    c: f32,
    nc: f32,
    fl: f32,
    nbb: f32,
    ncb: f32,
    z: f32,
    rgb_w: [f32; 3],
    d: f32,
}

impl Cam02State {
    pub fn new(vc: &ViewingConditions) -> Self {
        let ViewingConditions {
            la,
            yb,
            wp,
            surround,
        } = vc;
        let Surround { f, c, nc } = surround;

        // Step 0: Pre-calculations
        let k = 1.0 / (5.0 * la + 1.0);
        let k4 = k * k * k * k;
        let fl = 0.2 * k4 * (5.0 * la) + 0.1 * (1.0 - k4) * (1.0 - k4) * (5.0 * la).powf(1.0 / 3.0);

        let n = yb / wp.y;
        let nbb = 0.725 * (1.0 / n).powf(0.2);
        let ncb = nbb;
        let z = 1.48 + n.sqrt();

        // XYZ to LMS (CIECAT02 matrix)
        let rgb_w = [
            0.7328 * wp.x + 0.4296 * wp.y - 0.1624 * wp.z,
            -0.7036 * wp.x + 1.6975 * wp.y + 0.0061 * wp.z,
            0.0030 * wp.x + 0.0136 * wp.y + 0.9834 * wp.z,
        ];

        let d = (f * (1.0 - (1.0 / 3.6) * ((-la - 42.0) / 92.0).exp())).clamp(0.0, 1.0);

        Self {
            c: *c,
            nc: *nc,
            fl,
            nbb,
            ncb,
            z,
            rgb_w,
            d,
        }
    }

    pub fn xyz_to_ucs(&self, xyz: XYZ) -> Cam02Ucs {
        // Step 1: Chromatic adaptation
        let rgb = [
            0.7328 * xyz.x + 0.4296 * xyz.y - 0.1624 * xyz.z,
            -0.7036 * xyz.x + 1.6975 * xyz.y + 0.0061 * xyz.z,
            0.0030 * xyz.x + 0.0136 * xyz.y + 0.9834 * xyz.z,
        ];

        let mut rgb_c = [0.0f32; 3];
        for i in 0..3 {
            let factor = self.d * (100.0 / self.rgb_w[i]) + 1.0 - self.d;
            rgb_c[i] = rgb[i] * factor;
        }

        // Step 2: To Hunt-Pointer-Estevez (HPE) space
        // Matrix M_CAT02^-1 * M_HPE
        let rgb_p = [
            0.7409792 * rgb_c[0] + 0.218025 * rgb_c[1] + 0.041 * rgb_c[2],
            0.2853532 * rgb_c[0] + 0.6242014 * rgb_c[1] + 0.0904454 * rgb_c[2],
            -0.009628 * rgb_c[0] - 0.005698 * rgb_c[1] + 1.015326 * rgb_c[2],
        ];

        // Step 3: Response compression
        let mut rgb_a = [0.0f32; 3];
        for i in 0..3 {
            let val = (self.fl * rgb_p[i].abs() / 100.0).powf(0.42);
            rgb_a[i] = (400.0 * val) / (val + 27.13) + 0.1;
            if rgb_p[i] < 0.0 {
                rgb_a[i] = -rgb_a[i];
            }
        }

        // Step 4: Appearance correlates
        let a = rgb_a[0] - 12.0 * rgb_a[1] / 11.0 + rgb_a[2] / 11.0;
        let b = (1.0 / 9.0) * (rgb_a[0] + rgb_a[1] - 2.0 * rgb_a[2]);
        let h_rad = b.atan2(a);

        let et = 0.25 * ((h_rad + 2.0).cos() + 3.8);
        let ac = (2.0 * rgb_a[0] + rgb_a[1] + 0.05 * rgb_a[2] - 0.305) * self.nbb;

        let j = 100.0
            * (ac
                / ((2.0 * self.rgb_w[0] + self.rgb_w[1] + 0.05 * self.rgb_w[2] - 0.305)
                    * self.nbb))
                .powf(self.c * self.z);

        let t = (50000.0 / 13.0) * self.nc * self.ncb * et * (a * a + b * b).sqrt()
            / (rgb_a[0] + rgb_a[1] + 1.05 * rgb_a[2]);
        let c = t.powf(0.9) * (j / 100.0).sqrt() * (1.64 - 0.29f32.powf(self.nbb)).powf(0.73);

        // Step 5: CAM02-UCS scaling (Luo et al. 2006)
        // Using the UCS (Uniform Color Space) coefficients
        let kl = 1.00;
        let c1 = 0.007;
        let c2 = 0.0228;

        let j_prime = ((1.0 + 100.0 * c1) * j) / (1.0 + c1 * j);
        let m = c * self.fl.powf(0.25); // Use colorfulness M or chroma C? UCS uses M usually, but often simplified.
                                        // Actually, CAM02-UCS uses J', a', b' derived from J, M, h
                                        // M = C * F_L^0.25
        let m_prime = (1.0 / c2) * (1.0 + c2 * m).ln();

        let a_prime = m_prime * h_rad.cos();
        let b_prime = m_prime * h_rad.sin();

        Cam02Ucs {
            j_prime: j_prime / kl,
            a_prime,
            b_prime,
        }
    }
}
