#![allow(dead_code)]

pub fn wav_bytes(sample_rate: u32, seconds: f64) -> Vec<u8> {
    let samples = (sample_rate as f64 * seconds).max(1.0) as usize;
    let data_len = samples * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for i in 0..samples {
        let t = i as f64 / sample_rate as f64;
        let v = (t * 440.0 * std::f64::consts::TAU).sin() * 32_000.0;
        out.extend_from_slice(&(v as i16).to_le_bytes());
    }
    out
}

pub const MPV_TEST_SECONDS: f64 = 0.3;
