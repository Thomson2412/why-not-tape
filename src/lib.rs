pub mod adio;
pub mod config;
pub mod decoder;
pub mod encoder;
pub mod image;

pub use adio::{Audio, AudioSink};
pub use config::Config;
pub use decoder::decode;
pub use encoder::encode;
pub use image::Image;

#[cfg(test)]
mod integration {
    use super::*;

    /// Config where pixel frequencies are exact multiples of freq_resolution.
    /// sample_rate=8000, duration=1.0 → 8000 samples → resolution=1 Hz.
    /// Frequencies 200, 300, 400, … Hz are all exact multiples of 1 Hz.
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
            window: 1.0,
        }
    }

    #[test]
    fn round_trip_black_image() {
        let config = clean_config(6);
        let image = Image::new(3, 2);
        let samples = encode(&image, &config);
        let decoded = decode(&samples, &config, 3, 2);
        assert_eq!(decoded, image);
    }

    #[test]
    fn round_trip_white_image() {
        let config = clean_config(6);
        let image = Image::from_pixels(3, 2, vec![255; 6]);
        let samples = encode(&image, &config);
        let decoded = decode(&samples, &config, 3, 2);
        for row in 0..2 {
            for col in 0..3 {
                let orig = image.pixel(col, row) as i32;
                let got = decoded.pixel(col, row) as i32;
                assert!(
                    (orig - got).abs() <= 5,
                    "pixel ({col},{row}): expected {orig}, got {got}"
                );
            }
        }
    }

    #[test]
    fn round_trip_checkerboard() {
        // Alternating black and white pixels
        let pixels: Vec<u8> = (0..9).map(|i| if i % 2 == 0 { 255 } else { 0 }).collect();
        let config = clean_config(9);
        let image = Image::from_pixels(3, 3, pixels);
        let samples = encode(&image, &config);
        let decoded = decode(&samples, &config, 3, 3);
        for row in 0..3 {
            for col in 0..3 {
                let orig = image.pixel(col, row) as i32;
                let got = decoded.pixel(col, row) as i32;
                assert!(
                    (orig - got).abs() <= 10,
                    "pixel ({col},{row}): expected {orig}, got {got}"
                );
            }
        }
    }
}
