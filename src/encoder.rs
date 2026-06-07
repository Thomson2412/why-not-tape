use std::f32::consts::PI;

use crate::{Config, Image};


/// Encode an image into a single audio clip.
///
/// Every pixel in the image (scanned row-major, left-to-right, top-to-bottom) is
/// assigned a unique carrier frequency via `config.freq_for_pixel`. The output is
/// the sum of all carrier sine waves, each scaled by the pixel's intensity:
///
/// ```text
/// sample[t] = Σ_i  (pixel[i] / 255.0) * sin(2π * freq(i) * t / sample_rate)
/// ```
///
/// The returned `Vec<f32>` has length `config.total_samples()`.
pub fn encode(image: &Image, config: &Config) -> Vec<f32> {
    let mut samples: Vec<f32> = vec![0.0; config.total_samples()];
    for t in 0..config.total_samples() {
        for i in 0..image.pixel_amount() {
            samples[t] += image.pixel_by_index(i) as f32 / 255.0
                * (PI * 2.0 * config.freq_for_pixel(i, image.pixel_amount()) * t as f32
                    / config.sample_rate as f32)
                    .sin();
        }
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn mono_config(freq: f32) -> Config {
        // 1-second clip so t = i/44100 is exact.
        Config {
            sample_rate: 44100,
            min_freq: freq,
            max_freq: freq,
            window: 1.0,
        }
    }

    #[test]
    fn black_image_encodes_to_silence() {
        let config = Config {
            sample_rate: 100,
            min_freq: 440.0,
            max_freq: 880.0,
            window: 0.1, // 10 samples
        };
        let samples = encode(&Image::new(4, 3), &config);
        assert!(
            samples.iter().all(|&s| s == 0.0),
            "all samples must be zero for a black image"
        );
    }

    #[test]
    fn output_length_equals_total_samples() {
        let config = Config {
            sample_rate: 100,
            min_freq: 440.0,
            max_freq: 880.0,
            window: 0.1, // 10 samples
        };
        let samples = encode(&Image::new(4, 3), &config);
        assert_eq!(samples.len(), config.total_samples());
    }

    #[test]
    fn white_single_pixel_produces_full_amplitude_sine() {
        let config = mono_config(440.0);
        let samples = encode(&Image::from_pixels(1, 1, vec![255]), &config);

        for i in 0..20 {
            let t = i as f32 / 44100.0;
            let expected = (2.0 * PI * 440.0 * t).sin();
            assert_abs_diff_eq!(samples[i], expected, epsilon = 1e-5);
        }
    }

    #[test]
    fn amplitude_is_proportional_to_pixel_intensity() {
        let config = mono_config(440.0);
        let full = encode(&Image::from_pixels(1, 1, vec![255]), &config);
        let half = encode(&Image::from_pixels(1, 1, vec![128]), &config);

        // Use a sample where the sine is non-zero: t = 0.125 s
        let i = 44100 / 8;
        let ratio = half[i] / full[i];
        assert_abs_diff_eq!(ratio, 128.0 / 255.0, epsilon = 1e-5);
    }

    #[test]
    fn two_pixels_produce_sum_of_their_sine_waves() {
        let config = Config {
            sample_rate: 44100,
            min_freq: 440.0,
            max_freq: 880.0,
            window: 1.0,
        };
        let samples = encode(&Image::from_pixels(2, 1, vec![255, 255]), &config);

        for i in 0..20 {
            let t = i as f32 / 44100.0;
            let expected = (2.0 * PI * 440.0 * t).sin() + (2.0 * PI * 880.0 * t).sin();
            assert_abs_diff_eq!(samples[i], expected, epsilon = 1e-5);
        }
    }

    #[test]
    fn pixel_order_is_row_major() {
        // A 2×2 image:
        //  [255,   0]   pixel indices 0, 1
        //  [  0, 255]   pixel indices 2, 3
        //
        // Pixel 0 → min_freq, pixel 3 → max_freq.
        // Only pixel 0 and pixel 3 are lit, so only min_freq and max_freq should appear.
        let config = Config {
            sample_rate: 44100,
            min_freq: 440.0,
            max_freq: 1320.0,
            window: 1.0,
        };
        let image = Image::from_pixels(2, 2, vec![255, 0, 0, 255]);
        let samples = encode(&image, &config);

        for i in 0..20 {
            let t = i as f32 / 44100.0;
            // Only 440 Hz (pixel 0) and 1320 Hz (pixel 3) are present
            let expected = (2.0 * PI * 440.0 * t).sin() + (2.0 * PI * 1320.0 * t).sin();
            assert_abs_diff_eq!(samples[i], expected, epsilon = 1e-5);
        }
    }
}
