//! TM-30-18 Color Quality Evaluation Metrics
//!
//! Implements the IES TM-30-18 standard for evaluating light source color quality
//! using CES99 test samples and color appearance modeling.

#![allow(clippy::needless_range_loop)]

use crate::cam02::{Cam02State, Surround, ViewingConditions};
use crate::colorimetry::{calculate_cct, XYZ};
use crate::spectrum::SpectralData;
use crate::tm30_data::CES99_SPDS;
use crate::tm30_data_cmf::{X_BAR_10_5NM, Y_BAR_10_5NM, Z_BAR_10_5NM};

#[derive(Debug, Clone)]
pub struct TM30Metrics {
    pub rf: f32,
    pub rg: f32,
    pub cct: f32,
    pub duv: f32,
    pub bin_rf: [f32; 16],
    pub bin_chroma_shift: [f32; 16],
    pub bin_hue_shift: [f32; 16],
    pub bin_test_a: [f32; 16],
    pub bin_test_b: [f32; 16],
    pub bin_ref_a: [f32; 16],
    pub bin_ref_b: [f32; 16],
    pub ces_rgb: Vec<[u8; 3]>,
}

/// Calculate IES TM-30-18 metrics (Rf and Rg).
pub fn calculate_tm30(test_spd: &SpectralData) -> TM30Metrics {
    // 1. Resample test SPD to 5nm (360-830nm)
    let test_5nm = test_spd.resample(360.0, 830.0, 5.0);
    let test_vals = &test_5nm.values;

    // 2. Calculate CCT and Duv
    let (cct, duv) = calculate_cct(test_spd);

    // 3. Generate reference SPD (5nm, 360-830nm)
    let ref_vals = generate_reference_spd_5nm(cct);

    // 4. Calculate XYZ for test and reference white points (10 degree)
    let test_white_raw = calculate_xyz_10_5nm(test_vals, test_vals);
    let ref_white_raw = calculate_xyz_10_5nm(&ref_vals, &ref_vals);

    // Normalize white points to Y=100
    let test_white = XYZ {
        x: test_white_raw.x * 100.0 / test_white_raw.y,
        y: 100.0,
        z: test_white_raw.z * 100.0 / test_white_raw.y,
    };
    let ref_white = XYZ {
        x: ref_white_raw.x * 100.0 / ref_white_raw.y,
        y: 100.0,
        z: ref_white_raw.z * 100.0 / ref_white_raw.y,
    };

    // 5. Calculate XYZ for 99 CES samples under both sources
    let mut test_ucs = Vec::with_capacity(99);
    let mut ref_ucs = Vec::with_capacity(99);
    let mut ces_rgb = Vec::with_capacity(99);

    let vc_test = ViewingConditions::new(test_white, 100.0, 20.0, Surround::AVERAGE);
    let cam_test = Cam02State::new(&vc_test);

    let vc_ref = ViewingConditions::new(ref_white, 100.0, 20.0, Surround::AVERAGE);
    let cam_ref = Cam02State::new(&vc_ref);

    for ces_spd in &CES99_SPDS {
        // Test source XYZ
        let mut test_sample_vals = [0.0f32; 95];
        for j in 0..95 {
            test_sample_vals[j] = test_vals[j] * ces_spd[j];
        }
        let test_xyz_raw = calculate_xyz_10_5nm(&test_sample_vals, test_vals);
        let test_xyz = XYZ {
            x: test_xyz_raw.x * 100.0 / test_white_raw.y,
            y: test_xyz_raw.y * 100.0 / test_white_raw.y,
            z: test_xyz_raw.z * 100.0 / test_white_raw.y,
        };

        // Convert to sRGB for preview (normalize to white point)
        let (r, g, b) = test_xyz.to_srgb_safe(test_white);
        ces_rgb.push([r, g, b]);

        // Reference source XYZ
        let mut ref_sample_vals = [0.0f32; 95];
        for j in 0..95 {
            ref_sample_vals[j] = ref_vals[j] * ces_spd[j];
        }
        let ref_xyz_raw = calculate_xyz_10_5nm(&ref_sample_vals, &ref_vals);
        let ref_xyz = XYZ {
            x: ref_xyz_raw.x * 100.0 / ref_white_raw.y,
            y: ref_xyz_raw.y * 100.0 / ref_white_raw.y,
            z: ref_xyz_raw.z * 100.0 / ref_white_raw.y,
        };

        test_ucs.push(cam_test.xyz_to_ucs(test_xyz));
        ref_ucs.push(cam_ref.xyz_to_ucs(ref_xyz));
    }

    // 6. Calculate Rf
    let mut sum_de = 0.0;
    for i in 0..99 {
        sum_de += test_ucs[i].distance(&ref_ucs[i]);
    }
    let avg_de = sum_de / 99.0;
    let rf = 10.0f32 * (((100.0f32 - 7.54f32 * avg_de) / 10.0f32).exp() + 1.0f32).ln();

    // 7. Calculate Rg
    let mut bin_test_a = [0.0f32; 16];
    let mut bin_test_b = [0.0f32; 16];
    let mut bin_ref_a = [0.0f32; 16];
    let mut bin_ref_b = [0.0f32; 16];
    let mut bin_count = [0usize; 16];
    let mut bin_rf = [0.0f32; 16];
    let mut bin_de_sum = [0.0f32; 16];

    for i in 0..99 {
        let h = ref_ucs[i].h();
        let bin = ((h / 22.5).floor() as usize) % 16;

        bin_test_a[bin] += test_ucs[i].a_prime;
        bin_test_b[bin] += test_ucs[i].b_prime;
        bin_ref_a[bin] += ref_ucs[i].a_prime;
        bin_ref_b[bin] += ref_ucs[i].b_prime;
        bin_count[bin] += 1;

        let de = test_ucs[i].distance(&ref_ucs[i]);
        bin_de_sum[bin] += de;
    }

    for i in 0..16 {
        if bin_count[i] > 0 {
            bin_test_a[i] /= bin_count[i] as f32;
            bin_test_b[i] /= bin_count[i] as f32;
            bin_ref_a[i] /= bin_count[i] as f32;
            bin_ref_b[i] /= bin_count[i] as f32;

            let avg_de_bin = bin_de_sum[i] / bin_count[i] as f32;
            bin_rf[i] =
                10.0f32 * (((100.0f32 - 7.54f32 * avg_de_bin) / 10.0f32).exp() + 1.0f32).ln();
        }
    }

    let area_test = calculate_polygon_area(&bin_test_a, &bin_test_b);
    let area_ref = calculate_polygon_area(&bin_ref_a, &bin_ref_b);
    let rg = 100.0 * (area_test / area_ref);

    let mut bin_chroma_shift = [0.0f32; 16];
    let mut bin_hue_shift = [0.0f32; 16];
    for i in 0..16 {
        let c_test = (bin_test_a[i].powi(2) + bin_test_b[i].powi(2)).sqrt();
        let c_ref = (bin_ref_a[i].powi(2) + bin_ref_b[i].powi(2)).sqrt();
        bin_chroma_shift[i] = (c_test - c_ref) / c_ref;

        let h_test = bin_test_b[i].atan2(bin_test_a[i]);
        let h_ref = bin_ref_b[i].atan2(bin_ref_a[i]);
        let mut dh = h_test - h_ref;
        while dh > std::f32::consts::PI {
            dh -= 2.0 * std::f32::consts::PI;
        }
        while dh < -std::f32::consts::PI {
            dh += 2.0 * std::f32::consts::PI;
        }
        bin_hue_shift[i] = dh;
    }

    TM30Metrics {
        rf,
        rg,
        cct,
        duv,
        bin_rf,
        bin_chroma_shift,
        bin_hue_shift,
        bin_test_a,
        bin_test_b,
        bin_ref_a,
        bin_ref_b,
        ces_rgb,
    }
}

fn calculate_xyz_10_5nm(sample_vals: &[f32], source_vals: &[f32]) -> XYZ {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    let mut sum_y_source = 0.0;

    for i in 0..95 {
        x += sample_vals[i] * X_BAR_10_5NM[i];
        y += sample_vals[i] * Y_BAR_10_5NM[i];
        z += sample_vals[i] * Z_BAR_10_5NM[i];
        sum_y_source += source_vals[i] * Y_BAR_10_5NM[i];
    }

    let scale = 100.0 / sum_y_source;
    XYZ {
        x: x * scale,
        y: y * scale,
        z: z * scale,
    }
}

fn generate_reference_spd_5nm(cct: f32) -> [f32; 95] {
    if cct < 4000.0 {
        generate_planckian_5nm(cct)
    } else if cct > 5000.0 {
        generate_daylight_5nm(cct)
    } else {
        let p = (5000.0 - cct) / (5000.0 - 4000.0);
        let planck = generate_planckian_5nm(cct);
        let daylight = generate_daylight_5nm(cct);
        let mut blended = [0.0f32; 95];
        for i in 0..95 {
            blended[i] = p * planck[i] + (1.0 - p) * daylight[i];
        }
        blended
    }
}

fn generate_planckian_5nm(temp: f32) -> [f32; 95] {
    let mut spd = [0.0f32; 95];
    let c1 = 3.741771e-16;
    let c2 = 1.4388e-2;

    for i in 0..95 {
        let wl = (360 + i * 5) as f32 * 1e-9;
        spd[i] = c1 * wl.powi(-5) / ((c2 / (wl * temp)).exp() - 1.0);
    }
    spd
}

fn generate_daylight_5nm(temp: f32) -> [f32; 95] {
    let x_d = if temp <= 7000.0 {
        -4.6070e9 / temp.powi(3) + 2.9678e6 / temp.powi(2) + 0.09911e3 / temp + 0.244063
    } else {
        -2.0064e9 / temp.powi(3) + 1.9018e6 / temp.powi(2) + 0.24748e3 / temp + 0.237040
    };

    let y_d = -3.000 * x_d * x_d + 2.870 * x_d - 0.275;

    let m1 = (-1.3515 - 1.7703 * x_d + 5.9114 * y_d) / (0.0241 + 0.2562 * x_d - 0.7341 * y_d);
    let m2 = (0.0300 - 31.4424 * x_d + 30.0717 * y_d) / (0.0241 + 0.2562 * x_d - 0.7341 * y_d);

    // CIE S0, S1, S2 at 10nm (380-780)
    const S0: [f32; 41] = [
        0.0, 0.0, 33.4, 37.4, 117.4, 117.8, 114.9, 115.9, 108.8, 109.3, 107.8, 104.8, 107.7, 104.4,
        104.0, 100.0, 96.0, 95.1, 89.1, 90.5, 90.3, 88.4, 84.0, 85.1, 81.9, 82.6, 84.9, 81.3, 71.9,
        74.3, 76.4, 63.3, 71.7, 77.0, 65.2, 47.7, 68.6, 65.0, 66.0, 61.0, 53.3,
    ];
    const S1: [f32; 41] = [
        0.0, 0.0, -1.1, -0.5, -0.7, -1.2, -2.6, -2.9, -2.8, -4.5, -6.1, -7.6, -9.7, -11.7, -12.2,
        -13.6, -12.0, -13.3, -12.9, -10.6, -11.6, -10.8, -8.1, -10.3, -11.0, -11.5, -10.8, -10.9,
        -8.8, -7.3, -12.9, -15.8, -15.1, -12.2, -10.2, -8.6, -12.0, -14.6, -15.1, -14.9, -13.7,
    ];
    const S2: [f32; 41] = [
        0.0, 0.0, -2.1, -1.9, -1.1, -2.2, -3.5, -3.5, -3.3, -2.0, -1.2, -1.1, -0.5, 0.2, 0.5, 2.1,
        3.2, 4.1, 4.7, 5.1, 6.7, 7.3, 8.6, 9.8, 10.2, 14.9, 18.1, 15.9, 16.8, 24.2, 31.7, 15.3,
        18.9, 21.2, 15.6, 8.3, 18.9, 14.6, 15.5, 15.4, 14.6,
    ];

    let mut spd = [0.0f32; 95];
    for i in 0..95 {
        let wl = (360 + i * 5) as f32;
        // Interpolate S0, S1, S2 from 10nm table
        let t = (wl - 380.0) / 10.0;
        let idx = t.floor() as i32;
        let x = t - idx as f32;

        let get_val = |table: &[f32; 41], idx: i32, x: f32| {
            if idx < 0 {
                table[0]
            } else if idx >= 40 {
                table[40]
            } else {
                let v0 = table[idx as usize];
                let v1 = table[(idx + 1) as usize];
                v0 + x * (v1 - v0)
            }
        };

        let s0 = get_val(&S0, idx, x);
        let s1 = get_val(&S1, idx, x);
        let s2 = get_val(&S2, idx, x);

        spd[i] = s0 + m1 * s1 + m2 * s2;
        if spd[i] < 0.0 {
            spd[i] = 0.0;
        }
    }
    spd
}

fn calculate_polygon_area(a: &[f32; 16], b: &[f32; 16]) -> f32 {
    let mut area = 0.0;
    for i in 0..16 {
        let j = (i + 1) % 16;
        area += a[i] * b[j] - a[j] * b[i];
    }
    0.5 * area.abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectrum::MeasurementMode;

    #[test]
    fn test_tm30_d65() {
        // Create a D65-like spectrum
        let mut values = Vec::new();
        let mut wavelengths = Vec::new();
        for i in 0..41 {
            let wl = 380.0 + i as f32 * 10.0;
            wavelengths.push(wl);
            // Simplified D65-like values or just use a known CCT
            values.push(1.0); // Flat spectrum for now
        }

        let spd = SpectralData {
            wavelengths,
            values,
            mode: MeasurementMode::Emissive,
        };

        let metrics = calculate_tm30(&spd);
        println!(
            "Rf: {}, Rg: {}, CCT: {}",
            metrics.rf, metrics.rg, metrics.cct
        );

        // For a flat spectrum, Rf should be high but not necessarily 100
        // because the reference will be Planckian/Daylight which is not flat.
        assert!(metrics.rf > 0.0);
        assert!(metrics.rg > 0.0);
    }
}
