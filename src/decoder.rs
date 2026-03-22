use std::f32::consts::PI;

use crate::{Config, Image};

/// Decode an audio clip back into an image.
///
/// For each pixel index `i` (0..width*height), evaluates the DFT at the pixel's carrier
/// frequency to recover its amplitude, then maps that back to an intensity (0–255):
///
/// ```text
/// amplitude(i) = (2 / N) * |Σ_{n=0}^{N-1} sample[n] * e^{-i 2π f_i n / sample_rate}|
/// pixel(i)     = clamp(round(amplitude(i) * 255), 0, 255)
/// ```
///
/// The factor `2/N` normalises so that a full-amplitude sine recovers amplitude 1.0.
///
/// # Note on spectral leakage
/// Best accuracy is obtained when each carrier frequency is an integer multiple of
/// `config.freq_resolution()`. Non-aligned frequencies bleed into neighbouring channels.
///
/// # Panics
/// Panics if `samples.len() != config.total_samples()`.
pub fn decode(samples: &[f32], config: &Config, width: usize, height: usize) -> Image {
    if samples.len() != config.total_samples() {
        panic!("samples.len() != config.total_samples()");
    }

    let pixel_amount = width * height;
    let mut pixels: Vec<u8> = Vec::with_capacity(pixel_amount);

    for pixel_index in 0..pixel_amount {
        let frequency_for_pixel = config.freq_for_pixel(pixel_index, pixel_amount);
        let amplitude = amplitude_for_frequency(samples, frequency_for_pixel, config.sample_rate);
        pixels.insert(pixel_index, amplitude_to_intensity(amplitude));
    }

    Image::from_pixels(width, height, pixels)
}

fn amplitude_for_frequency(samples: &[f32], f_i: f32, sample_rate: u32) -> f32 {
    let n = samples.len() as f32;
    let sample_rate = sample_rate as f32;

    let (real, imag) =
        samples
            .iter()
            .enumerate()
            .fold((0.0_f32, 0.0_f32), |(re, im), (idx, &sample)| {
                let theta = 2.0 * PI * f_i * idx as f32 / sample_rate;
                // e^{-iθ} = cos(θ) - i * sin(θ)
                let cos = theta.cos();
                let sin = theta.sin();
                (re + sample * cos, im - sample * sin)
            });

    let magnitude = (real * real + imag * imag).sqrt();
    (2.0 / n) * magnitude
}

fn amplitude_to_intensity(amplitude: f32) -> u8 {
    let intensity = (amplitude * 255.0).round();
    intensity.clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    /// Generates a pure sine wave with no spectral leakage.
    ///
    /// Uses sample_rate=8000, n_samples=800 (0.1 s) → resolution=10 Hz.
    /// Frequencies must be multiples of 10 Hz (e.g. 200, 300, 400 Hz).
    fn sine_wave(freq: f32, amplitude: f32, sample_rate: u32, n_samples: usize) -> Vec<f32> {
        (0..n_samples)
            .map(|i| amplitude * (2.0 * PI * freq * i as f32 / sample_rate as f32).sin())
            .collect()
    }

    /// Config where N pixel frequencies (200, 200+spacing, …) are all exact multiples
    /// of the frequency resolution (10 Hz), giving zero spectral leakage in tests.
    fn clean_config(total_pixels: usize) -> Config {
        let max_freq = if total_pixels == 1 {
            200.0
        } else {
            200.0 + (total_pixels - 1) as f32 * 100.0
        };
        Config {
            sample_rate: 8000,
            min_freq: 200.0,
            max_freq,
            window: 0.1, // 800 samples; freq_resolution = 10 Hz
        }
    }

    #[test]
    fn silence_decodes_to_black_image() {
        let config = clean_config(4);
        let samples = vec![0.0f32; config.total_samples()];
        let image = decode(&samples, &config, 2, 2);
        for row in 0..2 {
            for col in 0..2 {
                assert_eq!(image.pixel(col, row), 0);
            }
        }
    }

    #[test]
    fn decoded_image_has_correct_dimensions() {
        let config = clean_config(6);
        let samples = vec![0.0f32; config.total_samples()];
        let image = decode(&samples, &config, 3, 2);
        assert_eq!(image.width, 3);
        assert_eq!(image.height, 2);
    }

    #[test]
    fn full_amplitude_sine_decodes_to_white_pixel() {
        // 1×1 image, only frequency is 200 Hz
        let config = clean_config(1);
        let samples = sine_wave(200.0, 1.0, 8000, config.total_samples());
        let image = decode(&samples, &config, 1, 1);
        assert_abs_diff_eq!(image.pixel(0, 0) as f32, 255.0, epsilon = 2.0);
    }

    #[test]
    fn half_amplitude_sine_decodes_to_half_intensity() {
        let config = clean_config(1);
        let samples = sine_wave(200.0, 0.5, 8000, config.total_samples());
        let image = decode(&samples, &config, 1, 1);
        let expected = (0.5_f32 * 255.0).round() as u8;
        assert_abs_diff_eq!(image.pixel(0, 0) as f32, expected as f32, epsilon = 3.0);
    }

    #[test]
    fn sine_at_wrong_frequency_does_not_activate_other_pixels() {
        // 2-pixel image: pixel 0 = 200 Hz, pixel 1 = 300 Hz
        // Play 300 Hz only — pixel 0 should stay dark, pixel 1 should light up
        let config = clean_config(2);
        let samples = sine_wave(300.0, 1.0, 8000, config.total_samples());
        let image = decode(&samples, &config, 2, 1);
        assert_abs_diff_eq!(image.pixel(0, 0) as f32, 0.0, epsilon = 5.0);
        assert_abs_diff_eq!(image.pixel(1, 0) as f32, 255.0, epsilon = 2.0);
    }

    #[test]
    fn amplitude_above_1_clamps_to_255() {
        let config = clean_config(1);
        let samples = sine_wave(200.0, 2.0, 8000, config.total_samples());
        let image = decode(&samples, &config, 1, 1);
        assert_eq!(image.pixel(0, 0), 255);
    }

    #[test]
    fn multiple_simultaneous_frequencies_decode_independently() {
        // All 4 pixels lit at full intensity
        let config = clean_config(4); // 200, 300, 400, 500 Hz
        let n = config.total_samples();
        let samples: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / 8000.0;
                (2.0 * PI * 200.0 * t).sin()
                    + (2.0 * PI * 300.0 * t).sin()
                    + (2.0 * PI * 400.0 * t).sin()
                    + (2.0 * PI * 500.0 * t).sin()
            })
            .collect();
        let image = decode(&samples, &config, 2, 2);
        for row in 0..2 {
            for col in 0..2 {
                assert_abs_diff_eq!(image.pixel(col, row) as f32, 255.0, epsilon = 3.0);
            }
        }
    }

    #[test]
    fn pixel_order_is_row_major() {
        // 2×2 image decoded from a single 300 Hz tone should light up pixel index 1,
        // which is row 0, col 1 (second pixel in row-major order).
        // Pixel freqs: 0→200 Hz, 1→300 Hz, 2→400 Hz, 3→500 Hz
        let config = clean_config(4);
        let samples = sine_wave(300.0, 1.0, 8000, config.total_samples());
        let image = decode(&samples, &config, 2, 2);
        // pixel index 1 = row 0, col 1
        assert_abs_diff_eq!(image.pixel(1, 0) as f32, 255.0, epsilon = 2.0);
        // all others should be dark
        assert_abs_diff_eq!(image.pixel(0, 0) as f32, 0.0, epsilon = 5.0);
        assert_abs_diff_eq!(image.pixel(0, 1) as f32, 0.0, epsilon = 5.0);
        assert_abs_diff_eq!(image.pixel(1, 1) as f32, 0.0, epsilon = 5.0);
    }

    #[test]
    #[should_panic]
    fn panics_if_sample_count_does_not_match_config() {
        let config = clean_config(4);
        let samples = vec![0.0f32; config.total_samples() + 1];
        decode(&samples, &config, 2, 2);
    }
}
