# ggwave-rs

Rust bindings for the [ggwave](https://github.com/ggerganov/ggwave) data-over-sound library.

## Overview

`ggwave-rs` provides safe Rust bindings for the ggwave library, which enables data transmission via sound waves. This library allows you to encode text or binary data into audio waveforms that can be transmitted through speakers and received by microphones.

## Features

- Encode text into audio waveforms
- Decode audio waveforms back into text
- Save encoded data as WAV files for playback in any audio player
- Multiple protocols with different speed/reliability tradeoffs
- Support for audible and ultrasound transmission modes
- Direct usage of the raw ggwave FFI bindings

## Installation

### Prerequisites

- Rust and Cargo
- C/C++ compiler (for building ggwave)
- CMake (version 3.10+)

### Adding to Your Project

Add the following to your `Cargo.toml`:

```toml
[dependencies]
ggwave-rs = { git = "https://github.com/Thoxy67/ggwave-rs.git" }
```

## Usage

### Basic Example

```rust
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
```

### Available Protocols

The library provides several protocols for different use cases:

```rust
// Audible protocols (can be heard by humans)
protocols::AUDIBLE_NORMAL    // Most reliable but slower
protocols::AUDIBLE_FAST      // Medium speed and reliability
protocols::AUDIBLE_FASTEST   // Fastest but less reliable

// Ultrasound protocols (potentially inaudible to humans)
protocols::ULTRASOUND_NORMAL
protocols::ULTRASOUND_FAST
protocols::ULTRASOUND_FASTEST

// Dual-tone protocols (DT)
protocols::DT_NORMAL
protocols::DT_FAST
protocols::DT_FASTEST

// Multi-tone protocols (MT)
protocols::MT_NORMAL
protocols::MT_FAST
protocols::MT_FASTEST
```

## Advanced Usage

### Custom Parameters

You can customize parameters for specific needs:

```rust
use ggwave_rs::{GGWave, Parameters, sample_formats, operating_modes};

// Get default parameters and customize
let mut params = GGWave::default_parameters();
params.sampleRateOut = 48000.0;
params.sampleFormatOut = sample_formats::F32;
params.operatingMode = operating_modes::RX_AND_TX;

// Create GGWave instance with custom parameters
let gg = GGWave::new_with_params(params);
```

### Protocol Management

You can enable or disable specific protocols:

```rust
// Enable only specific protocols for better decoding accuracy
gg.toggle_rx_protocol(protocols::AUDIBLE_NORMAL, true);
gg.toggle_rx_protocol(protocols::AUDIBLE_FAST, false);
```

## WAV File Handling

To create WAV files for playback in audio applications:

```rust
// Encode directly to WAV file
gg.encode_to_wav_file(text, protocols::AUDIBLE_NORMAL, 25, "message.wav")?;

// Or convert raw data to WAV
let wav_data = gg.raw_to_wav(&raw_data)?;
fs::write("message.wav", &wav_data)?;
```

## Notes on Decoding

For decoding, always use the raw audio data format rather than the WAV file format:

```rust
// This works
let raw_data = gg.encode(text, protocols::AUDIBLE_NORMAL, 25)?;
let decoded = gg.decode(&raw_data, 1024)?;

// WAV files are for playback only, not for decoding
```

## Building from Source

```bash
# Clone the repository
git clone https://github.com/Thoxy67/ggwave-rs.git
cd ggwave-rs

# Build the project
cargo build

# Run the examples
cargo run --example simple_example
cargo run --example advanced_example
cargo run --example unsafe_example
```

## How it Works

The library:
1. Uses bindgen to generate Rust bindings for the ggwave C/C++ library
2. Provides a safe Rust wrapper around the unsafe C API
3. Handles memory management and error handling
4. Adds utilities for WAV file generation

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgements

- [ggwave](https://github.com/ggerganov/ggwave) by Georgi Gerganov for the original library
- The Rust community for tools like bindgen