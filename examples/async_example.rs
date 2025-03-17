use ggwave_rs::async_impl::{AsyncGGWave, streams};
use ggwave_rs::protocols;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufReader};

/// Simple example demonstrating how to use AsyncGGWave
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing AsyncGGWave...");
    
    // Create a new AsyncGGWave instance
    let ggwave = AsyncGGWave::new().await?;
    
    // Message to encode
    let message = "Hello, async world! This is a test message sent over audio.";
    println!("Encoding message: {}", message);
    
    // Encode the message using audible protocol
    let encoded = ggwave.encode(message, protocols::AUDIBLE_NORMAL, 50).await?;
    println!("Successfully encoded {} bytes", encoded.len());
    
    // Save to WAV file
    println!("Saving encoded audio to test.wav...");
    ggwave.encode_to_wav_file(message, protocols::AUDIBLE_NORMAL, 50, "test.wav").await?;
    println!("Saved to test.wav");
    
    // Demonstrate decoding
    println!("Decoding the encoded data...");
    let decoded = ggwave.decode_to_string(&encoded, 1024).await?;
    println!("Decoded message: {}", decoded);
    
    // Demonstrate streaming to a file
    println!("Demonstrating async streaming...");
    let mut file = File::create("streamed.raw").await?;
    ggwave.stream_encoded(message, protocols::AUDIBLE_NORMAL, 50, &mut file).await?;
    file.flush().await?;
    println!("Streamed encoded data to streamed.raw");
    
    // Only run this part if we want to demonstrate audio stream processing
    if std::env::args().any(|arg| arg == "--stream-demo") {
        println!("Starting stream processing demo (reading from test.wav)...");
        
        // Open the WAV file we just created
        let file = File::open("test.wav").await?;
        let reader = BufReader::new(file);
        
        // Start background processing
        let mut receiver = streams::start_background_processing(
            ggwave.clone(),
            reader,
            4096,  // chunk size
            1024,  // max payload size
            10,    // buffer size
        ).await?;
        
        println!("Listening for messages (timeout: 5 seconds)...");
        
        // Wait for a message with timeout
        match receiver.recv_timeout(Duration::from_secs(5)).await {
            Some(msg) => println!("Received message: {}", msg),
            None => println!("No message received within timeout"),
        }
    }
    
    println!("Done!");
    Ok(())
}