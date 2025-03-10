// examples/unsafe_example.rs
//
// This example demonstrates how to use the raw FFI bindings directly
// for advanced use cases that might not be covered by the safe API.
//
// WARNING: This is unsafe code that requires proper understanding of the
// underlying C API. Use the safe wrapper functions when possible.

use ggwave_rs::{GGWave, ffi, protocols, Result};
use std::ffi::c_void;
use std::ptr;

fn main() -> Result<()> {
    println!("GGWave Unsafe Example - Using direct FFI bindings");
    println!("--------------------------------------------------");
    
    // Create a GGWave instance using the safe API
    let gg = GGWave::new();
    
    // Test message
    let text = "Hello from unsafe FFI example!";
    println!("Original text: {}", text);
    
    // First, encode using the safe API for comparison
    let encoded_safe = gg.encode(text, protocols::AUDIBLE_NORMAL, 25)?;
    println!("Safe API encoded data size: {} bytes", encoded_safe.len());
    
    // Now, let's do the same but using the raw FFI bindings directly
    unsafe {
        // Get the raw instance
        let instance = gg.raw_instance();
        
        // Convert message to a C-compatible pointer
        let payload_buffer = text.as_ptr() as *const c_void;
        let payload_size = text.len() as i32;
        
        // Calculate required buffer size (similar to what the safe API does)
        let waveform_size = ffi::ggwave_encode(
            instance,
            payload_buffer,
            payload_size,
            protocols::AUDIBLE_NORMAL,
            25, // volume
            ptr::null_mut(),
            1, // query size in bytes
        );
        
        println!("Using FFI directly to encode...");
        println!("Required buffer size: {} bytes", waveform_size);
        
        if waveform_size > 0 {
            // Allocate buffer for the encoded waveform
            let mut waveform_buffer = vec![0u8; waveform_size as usize];
            
            // Encode the data
            let result = ffi::ggwave_encode(
                instance,
                payload_buffer,
                payload_size,
                protocols::AUDIBLE_NORMAL,
                25, // volume
                waveform_buffer.as_mut_ptr() as *mut c_void,
                0, // perform actual encoding
            );
            
            if result > 0 {
                println!("FFI encoding successful: {} bytes", result);
                
                // Verify the unsafe result matches the safe result
                if waveform_buffer == encoded_safe {
                    println!("✅ Safe and unsafe results match perfectly!");
                } else {
                    println!("❌ Safe and unsafe results differ!");
                }
                
                // Now let's try decoding using the FFI directly
                let mut payload_buffer = vec![0u8; 1024]; // Max payload size
                
                let decode_result = ffi::ggwave_decode(
                    instance,
                    waveform_buffer.as_ptr() as *const c_void,
                    waveform_buffer.len() as i32,
                    payload_buffer.as_mut_ptr() as *mut c_void
                );
                
                if decode_result > 0 {
                    // Truncate to actual size
                    payload_buffer.truncate(decode_result as usize);
                    
                    // Convert to String
                    match String::from_utf8(payload_buffer) {
                        Ok(decoded_text) => {
                            println!("FFI decoded text: {}", decoded_text);
                            println!("✅ Decoding successful!");
                        },
                        Err(e) => {
                            println!("❌ UTF-8 conversion error: {}", e);
                        }
                    }
                } else {
                    println!("❌ FFI decoding failed!");
                }
            } else {
                println!("❌ FFI encoding failed!");
            }
        } else {
            println!("❌ FFI size calculation failed!");
        }
        
        // Advanced raw FFI usage - Query various internal parameters
        
        // Get duration frames
        let duration_frames = ffi::ggwave_rxDurationFrames(instance);
        println!("\nAdvanced FFI parameter inspection:");
        println!("Rx Duration Frames: {}", duration_frames);
        
        // Get default parameters to examine
        let params = ffi::ggwave_getDefaultParameters();
        println!("Sample Rate Input: {} Hz", params.sampleRateInp);
        println!("Sample Rate Output: {} Hz", params.sampleRateOut);
        println!("Samples Per Frame: {}", params.samplesPerFrame);
        println!("Sound Marker Threshold: {}", params.soundMarkerThreshold);
        
        // Example of modifying reception protocol parameters
        println!("\nModifying protocol parameters via FFI...");
        // Toggle protocols
        ffi::ggwave_txToggleProtocol(protocols::AUDIBLE_FAST, 0); // Disable fast
        ffi::ggwave_rxToggleProtocol(protocols::AUDIBLE_NORMAL, 1); // Enable normal
        
        // Modify frequency start parameters
        ffi::ggwave_rxProtocolSetFreqStart(protocols::AUDIBLE_NORMAL, 48);
        
        println!("Protocol parameters modified successfully");
        
        // Create a custom GGWave instance with specific parameters
        println!("\nCreating custom GGWave instance with FFI...");
        let mut custom_params = ffi::ggwave_getDefaultParameters();
        custom_params.sampleRateInp = 44100.0;
        custom_params.sampleRateOut = 44100.0;
        custom_params.soundMarkerThreshold = 4.0; // Increased threshold
        
        let custom_instance = ffi::ggwave_init(custom_params);
        println!("Custom instance created: {}", custom_instance);
        
        // Do something with the custom instance
        let frames = ffi::ggwave_rxDurationFrames(custom_instance);
        println!("Custom instance Rx duration frames: {}", frames);
        
        // Cleanup the custom instance
        ffi::ggwave_free(custom_instance);
        println!("Custom instance freed");
    }
    
    // Use the with_raw_instance helper for a more controlled unsafe experience
    unsafe {
        println!("\nUsing with_raw_instance helper...");
        
        gg.with_raw_instance(|instance| {
            // Get rx duration frames
            let frames = ffi::ggwave_rxDurationFrames(instance);
            println!("Rx Duration Frames: {}", frames);
            
            // Toggle a protocol
            ffi::ggwave_rxToggleProtocol(protocols::ULTRASOUND_NORMAL, 1);
            println!("ULTRASOUND_NORMAL protocol enabled");
        });
    }
    
    println!("\nUnsafe example completed successfully!");
    
    Ok(())
}