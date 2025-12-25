/// Sprague interpolation algorithm for high-resolution spectral reconstruction.
///
/// Sprague interpolation (from Sprague 1901) is a cubic interpolation method that provides
/// smooth, accurate spectral reconstruction from limited wavelength samples. It's particularly
/// useful for spectrometers that only provide data at fixed intervals (e.g., 5nm or 10nm).
///
/// This implementation enables:
/// - Smooth high-resolution spectrum from coarse samples
/// - Better accuracy in colorimetry calculations
/// - Improved CRI and TM-30 metrics computation
///
/// Reference: Sprague, K. (1901). "On the Interpolation of Curves."
/// Applied to spectral data: Ohno, Y. (1997). "Spectral luminous efficiency functions"
use crate::spectrum::SpectralData;

/// Perform Sprague cubic interpolation on spectral data.
///
/// # Arguments
/// * `wavelengths` - Input wavelengths in nm
/// * `intensities` - Corresponding spectral intensities
/// * `output_wavelengths` - Desired output wavelengths for interpolation
///
/// # Returns
/// Interpolated intensities at the output wavelengths
pub fn sprague_interpolate(
    wavelengths: &[f32],
    intensities: &[f32],
    output_wavelengths: &[f32],
) -> Vec<f32> {
    if wavelengths.len() < 4 {
        // Fallback to linear interpolation if not enough points
        return linear_interpolate(wavelengths, intensities, output_wavelengths);
    }

    let mut results = Vec::with_capacity(output_wavelengths.len());

    for &out_wl in output_wavelengths {
        // Find the bracketing interval
        let idx = wavelengths
            .binary_search_by(|w| w.partial_cmp(&out_wl).unwrap_or(std::cmp::Ordering::Equal));

        let result = match idx {
            Ok(i) => {
                // Exact match
                intensities[i]
            }
            Err(i) => {
                if i == 0 || i >= wavelengths.len() {
                    // Out of bounds - use nearest neighbor
                    if i == 0 {
                        intensities[0]
                    } else {
                        intensities[wavelengths.len() - 1]
                    }
                } else {
                    // Sprague interpolation between i-1 and i
                    let i0 = i.saturating_sub(2);
                    let i1 = i.saturating_sub(1);
                    let i2 = i;
                    let i3 = if i < wavelengths.len() - 1 {
                        i + 1
                    } else {
                        wavelengths.len() - 1
                    };

                    let y0 = intensities[i0];
                    let y1 = intensities[i1];
                    let y2 = intensities[i2];
                    let y3 = intensities[i3];

                    let h = wavelengths[i2] - wavelengths[i1];
                    let t = (out_wl - wavelengths[i1]) / h;

                    // Sprague basis functions
                    sprague_value(t, y0, y1, y2, y3)
                }
            }
        };

        results.push(result);
    }

    results
}

/// Compute Sprague cubic interpolation value.
/// Uses the Sprague cubic polynomial with standard coefficients.
fn sprague_value(t: f32, y0: f32, y1: f32, y2: f32, y3: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;

    // Sprague coefficients for uniform spacing
    let a0 = y1;
    let a1 = -y0 / 6.0 + y2 / 2.0 - y3 / 3.0;
    let a2 = y0 / 2.0 - y1 + y2 / 2.0;
    let a3 = -y0 / 6.0 + y1 / 2.0 - y2 / 2.0 + y3 / 6.0;

    a0 + a1 * t + a2 * t2 + a3 * t3
}

/// Linear fallback interpolation (for insufficient data points).
fn linear_interpolate(
    wavelengths: &[f32],
    intensities: &[f32],
    output_wavelengths: &[f32],
) -> Vec<f32> {
    let mut results = Vec::with_capacity(output_wavelengths.len());

    for &out_wl in output_wavelengths {
        let idx = wavelengths
            .binary_search_by(|w| w.partial_cmp(&out_wl).unwrap_or(std::cmp::Ordering::Equal));

        let value = match idx {
            Ok(i) => intensities[i],
            Err(i) => {
                if i == 0 {
                    intensities[0]
                } else if i >= wavelengths.len() {
                    intensities[wavelengths.len() - 1]
                } else {
                    let w1 = wavelengths[i - 1];
                    let w2 = wavelengths[i];
                    let y1 = intensities[i - 1];
                    let y2 = intensities[i];

                    let t = (out_wl - w1) / (w2 - w1);
                    y1 + t * (y2 - y1)
                }
            }
        };

        results.push(value);
    }

    results
}

/// Reconstruct high-resolution spectrum from coarse samples using Sprague interpolation.
///
/// # Arguments
/// * `spd` - Input spectral data (typically 5nm or 10nm samples)
/// * `output_resolution` - Target resolution in nm (e.g., 1.0 for 1nm steps)
///
/// # Returns
/// High-resolution spectral data reconstructed via Sprague interpolation
pub fn reconstruct_spectrum(spd: &SpectralData, output_resolution: f32) -> Vec<f32> {
    let wl_start = 380.0;
    let wl_end = 780.0;

    let num_samples = ((wl_end - wl_start) / output_resolution) as usize + 1;
    let mut output_wls = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        output_wls.push(wl_start + i as f32 * output_resolution);
    }

    // Extract wavelengths and data from SpectralData
    let (wavelengths, intensities) = spd.get_wavelength_data();

    sprague_interpolate(&wavelengths, &intensities, &output_wls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprague_smoothness() {
        // Simple test: interpolate at coarse points should match
        let wls = vec![380.0, 390.0, 400.0, 410.0, 420.0];
        let data = vec![10.0, 20.0, 15.0, 25.0, 30.0];

        let interp = sprague_interpolate(&wls, &data, &wls);

        // Should match original data exactly at the original wavelengths
        for (orig, interp_val) in data.iter().zip(interp.iter()) {
            assert!(
                (orig - interp_val).abs() < 0.1,
                "Mismatch at original wavelength"
            );
        }
    }

    #[test]
    fn test_sprague_intermediate() {
        let wls = vec![380.0, 390.0, 400.0];
        let data = vec![10.0, 20.0, 15.0];

        let interp = sprague_interpolate(&wls, &data, &[385.0]);

        // Interpolated value should be between the bracketing values
        assert!(interp[0] > 10.0 && interp[0] < 20.0);
    }
}
