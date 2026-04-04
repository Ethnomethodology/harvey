use hound::WavReader;
use std::io::Cursor;

pub fn generate_audio_peaks(audio_data: &[u8], block_size: usize) -> Result<Vec<f32>, String> {
    let mut reader = WavReader::new(Cursor::new(audio_data)).map_err(|e| e.to_string())?;
    let samples: Vec<i16> = reader.samples().collect::<Result<_, _>>().unwrap();

    let num_blocks = (samples.len() as f32 / block_size as f32).ceil() as usize;
    let mut peaks = Vec::with_capacity(num_blocks * 2);

    for i in 0..num_blocks {
        let start = i * block_size;
        let end = ((i + 1) * block_size).min(samples.len());
        let block = &samples[start..end];

        if block.is_empty() {
            peaks.push(0.0);
            peaks.push(0.0);
            continue;
        }

        let mut min = i16::MAX;
        let mut max = i16::MIN;

        for &sample in block {
            if sample < min {
                min = sample;
            }
            if sample > max {
                max = sample;
            }
        }

        peaks.push(min as f32 / i16::MAX as f32);
        peaks.push(max as f32 / i16::MAX as f32);
    }

    Ok(peaks)
}
