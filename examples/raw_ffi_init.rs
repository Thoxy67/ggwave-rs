// examples/raw_ffi_init.rs
use ggwave_rs::ffi;
use std::mem;

fn main() {
    println!("Testing direct FFI calls to ggwave");
    println!("==================================\n");

    unsafe {
        // Get default parameters
        let params = ffi::ggwave_getDefaultParameters();

        println!("Default parameters obtained:");
        println!("  sampleRate:      {}", params.sampleRate);
        println!("  samplesPerFrame: {}", params.samplesPerFrame);
        println!("  operatingMode:   {}", params.operatingMode);

        // Attempt to initialize with default parameters
        println!("\nAttempting initialization with default parameters...");
        let instance = ffi::ggwave_init(params);

        if instance > 0 {
            println!("ERROR: ggwave_init failed, returned {}", instance);
            println!("\nTrying with minimal parameters...");

            // Try with minimal parameters
            let mut min_params = params;
            min_params.sampleRate = 16000.0;
            min_params.sampleRateInp = 16000.0;
            min_params.sampleRateOut = 16000.0;
            min_params.samplesPerFrame = 512;
            min_params.soundMarkerThreshold = 0.1;
            min_params.operatingMode = ffi::GGWAVE_OPERATING_MODE_TX as i32;

            println!("  sampleRate:      {}", min_params.sampleRate);
            println!("  samplesPerFrame: {}", min_params.samplesPerFrame);
            println!("  operatingMode:   {}", min_params.operatingMode);

            let min_instance = ffi::ggwave_init(min_params);
            if min_instance <= 0 {
                println!(
                    "ERROR: Minimal parameters init also failed, returned {}",
                    min_instance
                );
            } else {
                println!(
                    "SUCCESS: Minimal parameters worked, instance = {}",
                    min_instance
                );

                // Try a simple encoding
                let text = "test";
                let size = ffi::ggwave_encode(
                    min_instance,
                    text.as_ptr() as *const std::ffi::c_void,
                    text.len() as i32,
                    ffi::ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_NORMAL,
                    25,
                    std::ptr::null_mut(),
                    1,
                );

                if size <= 0 {
                    println!("ERROR: ggwave_encode query failed, returned {}", size);
                } else {
                    println!("SUCCESS: ggwave_encode query returned size {}", size);

                    // Free the instance
                    ffi::ggwave_free(min_instance);
                }
            }
        } else {
            println!("SUCCESS: ggwave_init worked, instance = {}", instance);

            // Try a simple encoding
            let text = "test";
            let size = ffi::ggwave_encode(
                instance,
                text.as_ptr() as *const std::ffi::c_void,
                text.len() as i32,
                ffi::ggwave_ProtocolId_GGWAVE_PROTOCOL_AUDIBLE_NORMAL,
                25,
                std::ptr::null_mut(),
                1,
            );

            if size <= 0 {
                println!("ERROR: ggwave_encode query failed, returned {}", size);
            } else {
                println!("SUCCESS: ggwave_encode query returned size {}", size);

                // Free the instance
                ffi::ggwave_free(instance);
            }
        }
    }

    // Also print size information
    println!("\nStruct size information:");
    println!(
        "  ggwave_Parameters size: {} bytes",
        mem::size_of::<ffi::ggwave_Parameters>()
    );
    println!(
        "  ggwave_SampleFormat size: {} bytes",
        mem::size_of::<ffi::ggwave_SampleFormat>()
    );
    println!(
        "  ggwave_ProtocolId size: {} bytes",
        mem::size_of::<ffi::ggwave_ProtocolId>()
    );
}
