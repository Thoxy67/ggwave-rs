// examples/example_tx.rs
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ggwave_rs::{GGWave, protocols};
use ringbuf::{
    HeapRb,
    traits::{Consumer, Producer, Split, Observer},
};
use std::io::{self, Write};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

const SAMPLE_RATE: u32 = 48000;
const CHANNELS: usize = 1;
const BUFFER_SIZE: usize = SAMPLE_RATE as usize * 5; // 5 seconds of audio
const MAX_MESSAGE_SIZE: usize = 140; // Maximum message size for variable encoding

fn main() -> Result<()> {
    println!("GGWave Optimized Transmitter");

    // Create a GGWave instance optimized for transmission
    let ggwave = GGWave::new()?;

    println!("GGWave instance initialized");

    // Get the audio output device
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No output device available");

    println!("Using output device: {}", device.name()?);

    // Configure audio
    let config = cpal::StreamConfig {
        channels: CHANNELS as u16,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    // Create a ring buffer for audio processing
    let rb = HeapRb::<f32>::new(BUFFER_SIZE);
    let (producer, consumer) = rb.split();

    // Wrap in mutex for thread-safe access
    let producer = Arc::new(Mutex::new(producer));
    let consumer = Arc::new(Mutex::new(consumer));
    let consumer_clone = consumer.clone();

    // Flag to signal when audio playback should stop
    let playing = Arc::new(AtomicBool::new(false));
    let playing_clone = playing.clone();

    // Buffer for encoded waveform, shared between threads
    let waveform_buffer = Arc::new(Mutex::new(Vec::new()));
    let waveform_clone = waveform_buffer.clone();

    // Start audio playback stream
    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // If not playing, output silence
            if !playing_clone.load(Ordering::SeqCst) {
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }
                return;
            }

            // Fill the output buffer with samples from ring buffer
            let mut consumer_guard = consumer_clone.lock().unwrap();

            if consumer_guard.is_empty() {
                // No more samples to play
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }
                playing_clone.store(false, Ordering::SeqCst);
                return;
            }

            // Read available samples into output buffer
            let mut samples_read = 0;
            while samples_read < data.len() {
                if consumer_guard.is_empty() {
                    break;
                }

                // Read as many samples as possible
                let num_read = consumer_guard.pop_slice(&mut data[samples_read..]);
                samples_read += num_read;

                if num_read == 0 {
                    break;
                }
            }

            // Fill the rest with silence if needed
            for sample in data.iter_mut().skip(samples_read) {
                *sample = 0.0;
            }

            // If we couldn't fill the buffer, we're done playing
            if samples_read < data.len() {
                playing_clone.store(false, Ordering::SeqCst);
            }
        },
        err_fn,
        None,
    )?;

    stream.play()?;
    println!("Audio playback ready");

    // Main program loop
    loop {
        // Wait for any current playback to finish
        while playing.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(100));
        }

        println!("\nEnter a message to transmit (or 'q' to quit):");
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input == "q" || input == "quit" || input == "exit" {
            break;
        }

        if input.len() > MAX_MESSAGE_SIZE {
            println!(
                "Message too long! Maximum is {} characters.",
                MAX_MESSAGE_SIZE
            );
            continue;
        }

        // List available protocols
        println!("\nAvailable protocols:");
        println!("1. AUDIBLE_NORMAL - Standard audible transmission");
        println!("2. AUDIBLE_FAST - Faster audible transmission");
        println!("3. AUDIBLE_FASTEST - Fastest audible transmission");
        println!("4. ULTRASOUND_NORMAL - Standard ultrasound transmission");
        println!("5. ULTRASOUND_FAST - Faster ultrasound transmission");
        println!("6. ULTRASOUND_FASTEST - Fastest ultrasound transmission");
        // Choose protocol
        println!("Select protocol (1-6):");
        print!("> ");
        io::stdout().flush()?;

        let mut protocol_input = String::new();
        io::stdin().read_line(&mut protocol_input)?;

        let protocol_id = match protocol_input.trim().parse::<u8>() {
            Ok(1) => protocols::AUDIBLE_NORMAL,
            Ok(2) => protocols::AUDIBLE_FAST,
            Ok(3) => protocols::AUDIBLE_FASTEST,
            Ok(4) => protocols::ULTRASOUND_NORMAL,
            Ok(5) => protocols::ULTRASOUND_FAST,
            Ok(6) => protocols::ULTRASOUND_FASTEST,
            _ => {
                println!("Invalid selection, using AUDIBLE_NORMAL");
                protocols::AUDIBLE_NORMAL
            }
        };

        // Choose volume
        println!("Volume (1-100, default 50):");
        print!("> ");
        io::stdout().flush()?;

        let mut volume_input = String::new();
        io::stdin().read_line(&mut volume_input)?;

        let volume = match volume_input.trim().parse::<i32>() {
            Ok(v) if v > 0 && v <= 100 => v,
            _ => 50,
        };

        // Encode the message
        println!("Encoding message...");
        let estimated_duration = ggwave.estimate_duration(protocol_id, input.len());
        println!("Estimated duration: {:.2} seconds", estimated_duration);

        match ggwave.encode(input, protocol_id, volume) {
            Ok(waveform) => {
                // Store the encoded waveform
                let mut waveform_guard = waveform_clone.lock().unwrap();
                *waveform_guard = waveform;
                drop(waveform_guard);

                // Convert waveform to f32 samples and push to ring buffer
                let waveform_guard = waveform_clone.lock().unwrap();

                // Clear the consumer and producer buffer
                {
                    let mut consumer_guard = consumer.lock().unwrap();
                    while !consumer_guard.is_empty() {
                        let mut temp = [0.0f32; 128];
                        let _ = consumer_guard.pop_slice(&mut temp);
                    }
                }

                // Convert to f32 samples, handling alignment correctly
                if waveform_guard.len() % 4 != 0 {
                    println!("Warning: Waveform length is not a multiple of 4 bytes");
                }

                let bytes = waveform_guard.as_slice();
                let mut producer_guard = producer.lock().unwrap();

                // Process in batches to avoid holding the lock too long
                for chunk in bytes.chunks(4) {
                    if chunk.len() == 4 {
                        let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        if producer_guard.is_full() {
                            // If buffer is full, break - we can't store more
                            println!("Warning: Buffer full, some audio may be truncated");
                            break;
                        }
                        let _ = producer_guard.try_push(sample);
                    }
                }

                drop(producer_guard);

                // Start playback
                println!("Playing audio...");
                playing.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                println!("Error encoding message: {:?}", e);
            }
        }
    }

    println!("Shutting down...");
    Ok(())
}

fn err_fn(err: cpal::StreamError) {
    eprintln!("Audio stream error: {}", err);
}
