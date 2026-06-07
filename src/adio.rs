use std::path::Path;

/// A persistent mono output stream that plays queued [`Audio`] clips back-to-back
/// with no gap between them.
///
/// `rodio::OutputStream` is `!Send`, so keep this on the thread where it is created.
pub struct AudioSink {
    _stream: rodio::OutputStream,
    sink: rodio::Sink,
}

impl AudioSink {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (_stream, handle) = rodio::OutputStream::try_default()?;
        let sink = rodio::Sink::try_new(&handle)?;
        Ok(AudioSink { _stream, sink })
    }

    /// Append a clip to the playback queue.
    pub fn queue(&self, audio: &Audio) {
        let source =
            rodio::buffer::SamplesBuffer::new(1, audio.sample_rate, audio.normalized());
        self.sink.append(source);
    }

    /// Number of clips currently in the queue (including the one playing now).
    pub fn queued(&self) -> usize {
        self.sink.len()
    }
}

/// Mono audio clip. Samples are `f32` PCM in the range used by the encoder.
pub struct Audio {
    pub sample_rate: u32,
    samples: Vec<f32>,
}

impl Audio {
    pub fn new(sample_rate: u32, samples: Vec<f32>) -> Self {
        Audio {
            sample_rate,
            samples,
        }
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    fn normalized(&self) -> Vec<f32> {
        let peak = self
            .samples
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);
        let scale = if peak > 0.0 { 1.0 / peak } else { 1.0 };
        self.samples.iter().map(|&s| s * scale).collect()
    }

    /// Saves the audio to `path` as a 32-bit float mono WAV file, normalized to −1…1.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), hound::Error> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec)?;
        for s in self.normalized() {
            writer.write_sample(s)?;
        }
        writer.finalize()
    }

    /// Loads a 32-bit float mono WAV file from `path`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, hound::Error> {
        let mut reader = hound::WavReader::open(path)?;
        let sample_rate = reader.spec().sample_rate;
        let samples: Result<Vec<f32>, _> = reader.samples::<f32>().collect();
        Ok(Audio::new(sample_rate, samples?))
    }

    /// Plays the clip on the default output device, blocking until done.
    ///
    /// `rodio::OutputStream` is `!Send`, so this must run on whatever thread
    /// owns the audio context.  Use [`Audio::play_async`] to offload to a
    /// dedicated thread while keeping the calling thread free.
    pub fn play(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (_stream, handle) = rodio::OutputStream::try_default()?;
        let sink = rodio::Sink::try_new(&handle)?;
        let source =
            rodio::buffer::SamplesBuffer::new(1, self.sample_rate, self.normalized());
        sink.append(source);
        sink.sleep_until_end();
        Ok(())
    }

    /// Spawns a background thread that plays the clip and returns a handle.
    ///
    /// Call `handle.is_finished()` to poll for completion, or `handle.join()`
    /// to block until done.
    pub fn play_async(
        self,
    ) -> std::thread::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
        std::thread::spawn(|| self.play())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.wav");
        let audio = Audio::new(8000, vec![0.5, -0.5, 0.0]);
        audio.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn save_and_load_roundtrip_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.wav");
        let audio = Audio::new(8000, vec![0.0, 2.0, -2.0, 1.0]);
        audio.save(&path).unwrap();
        let loaded = Audio::load(&path).unwrap();
        assert_eq!(loaded.sample_rate, 8000);
        // Peak was 2.0, scale = 0.5 → [0.0, 1.0, -1.0, 0.5]
        let s = loaded.samples();
        assert!((s[0] - 0.0).abs() < 1e-5);
        assert!((s[1] - 1.0).abs() < 1e-5);
        assert!((s[2] - -1.0).abs() < 1e-5);
        assert!((s[3] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn silence_saves_without_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("silence.wav");
        let audio = Audio::new(44100, vec![0.0; 100]);
        audio.save(&path).unwrap();
    }
}
