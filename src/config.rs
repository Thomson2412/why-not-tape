/// Audio and encoding configuration.
pub struct Config {
    /// Samples per second (e.g. 44100).
    pub sample_rate: u32,
    /// Frequency assigned to pixel 0, in Hz.
    pub min_freq: f32,
    /// Frequency assigned to the last pixel, in Hz.
    pub max_freq: f32,
    /// Duration of the audio clip in seconds.
    /// Bigger windown → finer frequency resolution → more distinguishable pixels.
    pub window: f32,
}

impl Config {
    /// Maps a flat pixel index (row-major, 0..total_pixels) to its carrier frequency.
    ///
    /// Pixel 0 → `min_freq`, last pixel → `max_freq`, linearly interpolated.
    /// For a single-pixel image the frequency is always `min_freq`.
    pub fn freq_for_pixel(&self, pixel_index: usize, total_pixels: usize) -> f32 {
        if total_pixels == 1 {
            return self.min_freq;
        }
        let available_frequency_range = self.max_freq - self.min_freq;
        let pixel_fraction = pixel_index as f32 / (total_pixels - 1) as f32;
        available_frequency_range * pixel_fraction + self.min_freq
    }

    /// Total number of audio samples (`sample_rate * duration`, rounded).
    pub fn total_samples(&self) -> usize {
        (self.sample_rate as f32 * self.window).round() as usize
    }

    /// Smallest frequency difference that can be resolved (Hz per DFT bin).
    ///
    /// Equal to `sample_rate / total_samples`. Pixel frequencies must be spaced
    /// at least this far apart to avoid spectral leakage between channels.
    pub fn freq_resolution(&self) -> f32 {
        self.sample_rate as f32 / self.total_samples() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn cfg() -> Config {
        Config {
            sample_rate: 44100,
            min_freq: 1000.0,
            max_freq: 8000.0,
            window: 1.0,
        }
    }

    #[test]
    fn first_pixel_maps_to_min_freq() {
        assert_abs_diff_eq!(cfg().freq_for_pixel(0, 10), 1000.0);
    }

    #[test]
    fn last_pixel_maps_to_max_freq() {
        assert_abs_diff_eq!(cfg().freq_for_pixel(9, 10), 8000.0);
    }

    #[test]
    fn middle_pixel_interpolates_linearly() {
        // 3 pixels: indices 0, 1, 2 → freqs 1000, 4500, 8000
        assert_abs_diff_eq!(cfg().freq_for_pixel(1, 3), 4500.0, epsilon = 1.0);
    }

    #[test]
    fn single_pixel_image_always_returns_min_freq() {
        assert_abs_diff_eq!(cfg().freq_for_pixel(0, 1), 1000.0);
    }

    #[test]
    fn total_samples_rounds_to_nearest() {
        // 44100 Hz * 1.0 s = 44100 samples
        assert_eq!(cfg().total_samples(), 44100);
    }

    #[test]
    fn total_samples_rounds_non_integer() {
        let c = Config {
            sample_rate: 44100,
            window: 1.0 / 29.0,
            ..cfg()
        };
        let expected = (44100.0_f32 / 29.0).round() as usize;
        assert_eq!(c.total_samples(), expected);
    }

    #[test]
    fn freq_resolution_is_one_over_duration_for_integer_samples() {
        // sample_rate=8000, duration=0.1 → 800 samples → resolution = 8000/800 = 10 Hz
        let c = Config {
            sample_rate: 8000,
            window: 0.1,
            ..cfg()
        };
        assert_abs_diff_eq!(c.freq_resolution(), 10.0, epsilon = 0.01);
    }
}
