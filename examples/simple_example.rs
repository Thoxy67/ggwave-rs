// examples/simple_example.rs
use ggwave_rs::{GGWave, Result, protocols};
use std::fs;

fn main() -> Result<()> {
    // Create a new GGWave instance with better error handling
    println!("Creating GGWave instance...");

    let gg = GGWave::new()?;
    // Text to encode
    let text = "Hello from Rust GGWave!";
    println!("Encoding text: {}", text);

    // Encode the text to raw audio data
    let raw_data = gg.encode(text, protocols::AUDIBLE_FASTEST, 25)?;
    println!("Encoded raw waveform size: {} bytes", raw_data.len());

    // Save raw data to a binary file (for direct decoding)
    fs::write("message.bin", &raw_data)?;
    println!("Raw data saved to message.bin");

    // Also save as WAV for playback in audio players
    gg.encode_to_wav_file(text, protocols::AUDIBLE_FASTEST, 25, "message.wav")?;
    println!("WAV file saved to message.wav");

    // Test decoding from the raw data using buffer
    let mut decode_buffer = vec![0u8; 1024];
    let decoded = gg.decode(&raw_data, &mut decode_buffer)?;
    println!("Decoded text: {}", decoded);
    assert_eq!(text, decoded);

    // Read back the raw data file and decode
    let file_data = fs::read("message.bin")?;
    let decoded_from_file = gg.decode(&file_data, &mut decode_buffer)?;
    println!("Decoded from file: {}", decoded_from_file);

    println!("Example completed successfully!");
    Ok(())
}
