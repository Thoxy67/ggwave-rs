// examples/example_rx.rs
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ggwave_rs::{GGWave, operating_modes, sample_formats};
use hound::{WavSpec, WavWriter};
use std::io::Write;
use std::sync::Mutex;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::{thread, time::Duration};

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: usize = 1;
const BUFFER_SIZE: usize = SAMPLE_RATE as usize * 10; // 10 seconds of audio at 48kHz
const PROCESS_FRAMES: i32 = 1024; // Process 1024 samples at a time (matches C++ implementation)

fn main() -> Result<()> {
    println!("GGWave Optimized Receiver");
    println!("Initializing audio capture...");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    println!("Usage: {} [-cN] [-d]", args[0]);
    println!("    -cN - select capture device N");
    println!("    -d  - enable debug WAV file saving");
    println!();

    // Find capture device argument if present
    let mut capture_id = 0;
    let mut debug_mode = false;
    
    for arg in &args[1..] {
        if arg.starts_with("-c") {
            if let Ok(id) = arg[2..].parse::<usize>() {
                capture_id = id;
            }
        } else if arg == "-d" {
            debug_mode = true;
            println!("Debug mode enabled: WAV files will be saved");
        }
    }

    // Create a GGWave instance configured similarly to the C++ implementation
    let ggwave = match GGWave::builder()
        .sample_rate(SAMPLE_RATE as f32)
        .samples_per_frame(PROCESS_FRAMES)
        .sound_marker_threshold(0.5)
        .input_sample_format(sample_formats::F32)
        .output_sample_format(sample_formats::I16)
        .operating_mode(operating_modes::RX_AND_TX)
        .build()
    {
        Ok(instance) => {
            println!("GGWave instance created successfully");
            instance
        }
        Err(e) => {
            eprintln!("Failed to create GGWave instance: {:?}", e);
            return Err(anyhow::anyhow!("Failed to initialize GGWave"));
        }
    };

    // Enable all reception protocols (matching C++ implementation)
    ggwave.enable_all_rx_protocols();

    // Set up audio capture
    let host = cpal::default_host();

    // List available input devices
    println!("Found {} capture devices:", host.input_devices()?.count());
    let input_devices = host.input_devices()?;
    let mut devices: Vec<_> = input_devices.collect();
    for (i, device) in devices.iter().enumerate() {
        println!(
            "    - Capture device #{}: '{}'",
            i,
            device
                .name()
                .unwrap_or_else(|_| "Unknown device".to_string())
        );
    }

    // List available output devices
    println!("Found {} playback devices:", host.output_devices()?.count());
    let output_devices = host.output_devices()?;
    for (i, device) in output_devices.enumerate() {
        println!(
            "    - Playback device #{}: '{}'",
            i,
            device
                .name()
                .unwrap_or_else(|_| "Unknown device".to_string())
        );
    }

    // Select the capture device
    if capture_id >= devices.len() {
        println!("Capture device {} not found, using default", capture_id);
        capture_id = 0;
    }

    let device = if !devices.is_empty() {
        println!(
            "Attempt to open capture device {} : '{}' ...",
            capture_id,
            devices[capture_id]
                .name()
                .unwrap_or_else(|_| "Unknown device".to_string())
        );
        devices.remove(capture_id)
    } else {
        println!("Attempt to open default capture device ...");
        host.default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device available"))?
    };

    println!("Using input device: {}", device.name()?);

    // Configure audio to match C++ implementation
    let config = cpal::StreamConfig {
        channels: CHANNELS as u16,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    // Get supported configurations
    println!("Supported input configurations:");
    match device.supported_input_configs() {
        Ok(configs) => {
            for config in configs {
                println!("  - {:?}", config);
            }
        }
        Err(e) => println!("Error getting supported configurations: {}", e),
    }

    println!("Obtained spec for input device:");
    println!("    - Sample rate:       {}", SAMPLE_RATE);
    println!("    - Format:            {}", "f32");
    println!("    - Channels:          {}", CHANNELS);
    println!("    - Samples per frame: {}", PROCESS_FRAMES);
    println!("    - Debug mode:        {}", if debug_mode { "enabled" } else { "disabled" });

    // Create audio processing buffer (circular buffer like the C++ impl)
    let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(BUFFER_SIZE)));
    let audio_buffer_clone = audio_buffer.clone();

    // Create a buffer for recording samples to WAV file
    let recording_buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(
        SAMPLE_RATE as usize * 30,
    ))); // 30 seconds max
    let recording_buffer_clone = recording_buffer.clone();

    // Add some test data to verify the recording buffer works
    if debug_mode {
        println!("Adding test tone to recording buffer...");
        let mut rec_buf = recording_buffer.lock().unwrap();
        // Generate a 1-second 440 Hz test tone
        for i in 0..SAMPLE_RATE as usize {
            let t = i as f32 / SAMPLE_RATE as f32;
            let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
            rec_buf.push(sample);
        }
        println!("Added {} test samples to recording buffer", rec_buf.len());
    }

    // Flag to signal when we want to exit
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    // Save test recording before starting capture
    if debug_mode {
        println!("Saving test recording...");
        save_wav_snapshot(&recording_buffer, "test_recording.wav", true)?;
    }

    // Start audio capture
    println!("Building input stream with config: {:?}", config);
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &_| {
            // Check if we're getting data
            if data.is_empty() {
                return; // No data received
            }

            // Print diagnostic info occasionally
            static mut CALLBACK_COUNT: usize = 0;

            unsafe {
                CALLBACK_COUNT += 1;
                if CALLBACK_COUNT % 100 == 0 {
                    // Only log every ~5 seconds
                    if CALLBACK_COUNT % 500 == 0 {
                        println!("Audio callback received {} samples", data.len());
                        if !data.is_empty() {
                            let min = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                            let max = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                            println!("Audio data range: {:.6} to {:.6}", min, max);
                        }
                    }
                }
            }

            // Store in our circular buffer for processing
            let mut audio_buf = match audio_buffer_clone.lock() {
                Ok(buf) => buf,
                Err(e) => {
                    eprintln!("Failed to lock audio buffer: {:?}", e);
                    return;
                }
            };

            audio_buf.extend_from_slice(data);

            // Keep buffer at a reasonable size
            if audio_buf.len() > BUFFER_SIZE {
                let excess = audio_buf.len() - BUFFER_SIZE;
                audio_buf.drain(0..excess);
            }

            // Also store in our recording buffer for WAV export if debug mode is enabled
            if debug_mode {
                match recording_buffer_clone.lock() {
                    Ok(mut rec_buf) => {
                        rec_buf.extend_from_slice(data);

                        // Limit recording to 30 seconds max
                        if rec_buf.len() > SAMPLE_RATE as usize * 30 {
                            let excess = rec_buf.len() - (SAMPLE_RATE as usize * 30);
                            rec_buf.drain(0..excess);
                        }

                        // Report recording buffer size occasionally
                        unsafe {
                            if CALLBACK_COUNT % 500 == 0 {
                                println!(
                                    "Recording buffer size: {} samples ({:.1} seconds)",
                                    rec_buf.len(),
                                    rec_buf.len() as f32 / SAMPLE_RATE as f32
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to lock recording buffer: {:?}", e);
                    }
                }
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    stream.play()?;
    println!("Audio capture started");

    // Handle Ctrl+C to exit gracefully
    let recording_buffer_for_handler = recording_buffer.clone();
    let debug_mode_for_handler = debug_mode;
    ctrlc::set_handler(move || {
        running_clone.store(false, Ordering::SeqCst);
        println!("\nExiting...");

        // Save the recorded audio to a WAV file if debug mode is enabled
        if debug_mode_for_handler {
            println!("Saving recorded audio to 'debug_recording.wav'...");
            if let Ok(rec_buf) = recording_buffer_for_handler.lock() {
                if !rec_buf.is_empty() {
                    // Create WAV file with appropriate parameters
                    let spec = WavSpec {
                        channels: CHANNELS as u16,
                        sample_rate: SAMPLE_RATE,
                        bits_per_sample: 32,
                        sample_format: hound::SampleFormat::Float,
                    };

                    if let Ok(mut writer) = WavWriter::create("debug_recording.wav", spec) {
                        for &sample in rec_buf.iter() {
                            if let Err(e) = writer.write_sample(sample) {
                                eprintln!("Error writing sample to WAV: {}", e);
                                break;
                            }
                        }

                        if let Err(e) = writer.finalize() {
                            eprintln!("Error finalizing WAV file: {}", e);
                        } else {
                            println!("Successfully saved recording to 'debug_recording.wav'");
                        }
                    } else {
                        eprintln!("Error creating WAV file");
                    }
                } else {
                    println!("No audio recorded to save.");
                }
            } else {
                eprintln!("Could not access recording buffer");
            }
        }
    })?;

    println!("Listening for GGWave messages...");
    println!("Press Ctrl+C to exit");

    // Decode buffer - matching kMaxDataSize in C++ implementation
    let mut decode_buffer = vec![0u8; 256];

    // Create buffer for raw audio data
    let mut raw_audio = vec![0u8; (PROCESS_FRAMES as usize) * 4]; // 4 bytes per f32 sample

    // Main processing loop - structured similar to the C++ GGWave_mainLoop function
    let mut dot_timer = std::time::Instant::now();

    // For periodic WAV saving during debug
    let mut save_timer = std::time::Instant::now();

    while running.load(Ordering::SeqCst) {
        // Get audio data from our buffer, similar to SDL_DequeueAudio in C++
        let audio_data = {
            let mut audio_buf = match audio_buffer.lock() {
                Ok(buf) => buf,
                Err(_) => continue, // Mutex poisoned, can't continue
            };

            if audio_buf.len() < PROCESS_FRAMES as usize {
                // Not enough data yet
                drop(audio_buf);
                thread::sleep(Duration::from_millis(10));
                continue;
            }

            // Take samples for processing (similar to SDL_DequeueAudio)
            let mut data = Vec::with_capacity(PROCESS_FRAMES as usize);
            data.extend_from_slice(&audio_buf[0..PROCESS_FRAMES as usize]);

            // Remove processed samples from buffer
            audio_buf.drain(0..PROCESS_FRAMES as usize);

            data
        };

        // Convert f32 samples to bytes for processing
        // This mimics how C++ handles the audio buffer conversion
        for (i, sample) in audio_data.iter().enumerate() {
            let bytes = sample.to_le_bytes();
            raw_audio[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
        }

        // Process the audio with ggwave (similar to the C++ g_ggWave->decode call)
        match ggwave.decode(&raw_audio, &mut decode_buffer) {
            Ok(decoded_text) if !decoded_text.is_empty() => {
                println!("\n✅ Message received: \"{}\"", decoded_text);

                // Save WAV file when a message is detected to help debug, but only if debug mode is enabled
                if debug_mode {
                    save_wav_snapshot(&recording_buffer, "message_detected.wav", true)?;
                }
            }
            Err(e) => {
                eprintln!("Error decoding audio: {:?}", e);
            }
            _ => {}
        }

        // Show activity indicator (similar to simple progress output in other ggwave examples)
        if dot_timer.elapsed() > Duration::from_secs(1) {
            print!(".");
            std::io::stdout().flush().unwrap();
            dot_timer = std::time::Instant::now();
        }

        // Save debug recording periodically (every 10 seconds), but only if debug mode is enabled
        if debug_mode && save_timer.elapsed() > Duration::from_secs(10) {
            save_wav_snapshot(&recording_buffer, "debug_recording_periodic.wav", true)?;
            save_timer = std::time::Instant::now();
        }

        // Small pause to avoid high CPU usage
        thread::sleep(Duration::from_millis(5));
    }

    println!("\nShutting down...");

    // Final save of the recorded audio, but only if debug mode is enabled
    if debug_mode {
        save_wav_snapshot(&recording_buffer, "debug_recording_final.wav", true)?;
    }

    Ok(())
}

// Helper function to save the current audio buffer to a WAV file
fn save_wav_snapshot(recording_buffer: &Arc<Mutex<Vec<f32>>>, filename: &str, force: bool) -> Result<()> {
    // Skip if not forced (used for compatibility with existing code)
    if !force {
        return Ok(());
    }

    println!("Saving audio snapshot to '{}'...", filename);

    match recording_buffer.lock() {
        Ok(rec_buf) => {
            println!(
                "Recording buffer contains {} samples ({:.1} seconds of audio)",
                rec_buf.len(),
                rec_buf.len() as f32 / SAMPLE_RATE as f32
            );

            if !rec_buf.is_empty() {
                let spec = WavSpec {
                    channels: CHANNELS as u16,
                    sample_rate: SAMPLE_RATE,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                };

                let mut writer = WavWriter::create(filename, spec)?;

                // Print first few samples for debugging
                if rec_buf.len() > 10 {
                    println!("First 10 samples: {:?}", &rec_buf[0..10]);
                }

                for &sample in rec_buf.iter() {
                    writer.write_sample(sample)?;
                }

                writer.finalize()?;
                println!("Successfully saved audio to '{}'", filename);
            } else {
                println!("No audio recorded to save.");
            }
        }
        Err(e) => {
            eprintln!("Could not access recording buffer: {:?}", e);
        }
    }

    Ok(())
}