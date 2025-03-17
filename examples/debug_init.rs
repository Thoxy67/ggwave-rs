// examples/debug_init.rs
use ggwave_rs::{GGWave, Parameters, operating_modes, sample_formats};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // First, check that the ggwave library is properly compiled
    println!("=== Checking build environment ===");
    check_build_environment();

    println!("\n=== Default Parameters ===");
    unsafe {
        let params = ggwave_rs::ffi::ggwave_getDefaultParameters();
        print_parameters(&params);
    }

    println!("\n=== Trying initialization with default parameters ===");
    match GGWave::new() {
        Ok(_) => println!("SUCCESS: Default initialization worked!"),
        Err(e) => println!("ERROR: Default initialization failed: {:?}", e),
    }

    println!("\n=== Trying with minimal parameters ===");
    let minimal_params = Parameters {
        payloadLength: 0,
        sampleRateInp: 16000.0,
        sampleRateOut: 16000.0,
        sampleRate: 16000.0,
        samplesPerFrame: 512,
        soundMarkerThreshold: 0.1,
        sampleFormatInp: sample_formats::F32,
        sampleFormatOut: sample_formats::F32,
        operatingMode: operating_modes::RX_AND_TX,
    };

    print_parameters(&minimal_params);

    match GGWave::new_with_params(minimal_params) {
        Ok(_) => println!("SUCCESS: Minimal parameter initialization worked!"),
        Err(e) => println!("ERROR: Minimal parameter initialization failed: {:?}", e),
    }

    println!("\n=== Trying with different operating modes ===");

    for mode in [
        operating_modes::RX,
        operating_modes::TX,
        operating_modes::RX_AND_TX,
        operating_modes::TX_ONLY_TONES,
    ] {
        let mode_str = match mode {
            m if m == operating_modes::RX => "RX",
            m if m == operating_modes::TX => "TX",
            m if m == operating_modes::RX_AND_TX => "RX_AND_TX",
            m if m == operating_modes::TX_ONLY_TONES => "TX_ONLY_TONES",
            _ => "UNKNOWN",
        };

        println!("Trying with mode: {}", mode_str);

        let mut params = GGWave::default_parameters();
        params.operatingMode = mode;

        match GGWave::new_with_params(params) {
            Ok(_) => println!("  SUCCESS: Mode {} worked!", mode_str),
            Err(e) => println!("  ERROR: Mode {} failed: {:?}", mode_str, e),
        }
    }

    println!("\n=== Trying with very simple parameters ===");

    for rate in [8000.0, 16000.0, 44100.0, 48000.0] {
        for samples_per_frame in [256, 512, 1024] {
            println!(
                "Trying: rate={}, samples_per_frame={}",
                rate, samples_per_frame
            );

            let mut params = GGWave::default_parameters();
            params.sampleRate = rate;
            params.sampleRateInp = rate;
            params.sampleRateOut = rate;
            params.samplesPerFrame = samples_per_frame;
            params.soundMarkerThreshold = 0.5;

            match GGWave::new_with_params(params) {
                Ok(_) => println!("  SUCCESS!"),
                Err(e) => println!("  ERROR: {:?}", e),
            }
        }
    }
}

fn print_parameters(params: &Parameters) {
    println!("  payloadLength:        {}", params.payloadLength);
    println!("  sampleRateInp:        {}", params.sampleRateInp);
    println!("  sampleRateOut:        {}", params.sampleRateOut);
    println!("  sampleRate:           {}", params.sampleRate);
    println!("  samplesPerFrame:      {}", params.samplesPerFrame);
    println!("  soundMarkerThreshold: {}", params.soundMarkerThreshold);
    println!("  sampleFormatInp:      {}", params.sampleFormatInp);
    println!("  sampleFormatOut:      {}", params.sampleFormatOut);
    println!("  operatingMode:        {}", params.operatingMode);
}

fn check_build_environment() {
    // Check if the ggwave library is properly compiled
    let out_dir = std::env::var("OUT_DIR").unwrap_or_else(|_| "unknown".to_string());
    println!("OUT_DIR: {}", out_dir);

    // Check for vendors directory
    let vendor_path = PathBuf::from("vendors/ggwave");
    if vendor_path.exists() {
        println!("ggwave vendor directory exists: {}", vendor_path.display());

        // Check for header and source
        let header_path = vendor_path.join("include/ggwave/ggwave.h");
        let source_path = vendor_path.join("src/ggwave.cpp");

        println!(
            "Header exists: {} ({})",
            header_path.exists(),
            header_path.display()
        );
        println!(
            "Source exists: {} ({})",
            source_path.exists(),
            source_path.display()
        );
    } else {
        println!("ggwave vendor directory is missing!");
    }

    // Try to list compiled objects in OUT_DIR
    if out_dir != "unknown" {
        println!("Checking compiled objects in OUT_DIR:");
        let output = Command::new("ls").arg("-la").arg(&out_dir).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("{}", stdout);
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("Error listing directory: {}", stderr);
                }
            }
            Err(e) => {
                println!("Failed to execute ls command: {}", e);
            }
        }
    }
}
