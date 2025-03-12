// examples/advanced_example.rs
use ggwave_rs::{GGWave, Result, operating_modes, protocols, sample_formats};

fn main() -> Result<()> {
    // Create a GGWave instance with custom parameters
    let mut params = GGWave::default_parameters();

    // Customize parameters
    params.sampleRateInp = 48000.0;
    params.sampleRateOut = 48000.0;
    params.sampleRate = 48000.0;
    params.samplesPerFrame = 1024;
    params.sampleFormatInp = sample_formats::F32;
    params.sampleFormatOut = sample_formats::I16;
    params.operatingMode = operating_modes::RX_AND_TX;

    let gg = GGWave::new();
    let text = "Testing direct encode/decode";

    // Enable debugging to see library output
    gg.set_debug_mode(Some("ggwave_debug.log"));

    let protocols_to_test = [
        (protocols::AUDIBLE_NORMAL, "AUDIBLE_NORMAL"),
        (protocols::AUDIBLE_FAST, "AUDIBLE_FAST"),
        (protocols::AUDIBLE_FASTEST, "AUDIBLE_FASTEST"),
        (protocols::ULTRASOUND_NORMAL, "ULTRASOUND_NORMAL"),
        (protocols::ULTRASOUND_FAST, "ULTRASOUND_FAST"),
        (protocols::ULTRASOUND_FASTEST, "ULTRASOUND_FASTEST"),
    ];

    for (protocol, name) in &protocols_to_test {
        // Encode with this protocol
        match gg.encode(text, *protocol, 25) {
            Ok(raw_data) => {
                // Store raw size
                let raw_size = raw_data.len();

                println!("Testing decoding with only {} enabled", name);

                // Try decoding the raw data directly
                match gg.decode(&raw_data, 1024) {
                    Ok(decoded) => println!("Success! Decoded: {}", decoded),
                    Err(_) => println!("Failed to decode raw data of {} bytes", raw_size),
                }
            }
            Err(e) => println!("Encode error: {:?}", e),
        }
    }

    Ok(())
}
