use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

fn main() {
    println!("Starting debug build.rs");

    // Create vendors directory if not exists
    fs::create_dir_all("vendors/").unwrap_or_else(|e| {
        eprintln!("Failed to create vendors directory: {}", e);
        // Continue anyway
    });

    // Clone ggwave repository if not exists
    let ggwave_dir = PathBuf::from("vendors/ggwave");
    if !ggwave_dir.exists() {
        println!("Cloning ggwave repository...");
        let status = Command::new("git")
            .args(&[
                "clone",
                "https://github.com/ggerganov/ggwave.git",
                "--depth=1",
                "vendors/ggwave",
            ])
            .status();

        match status {
            Ok(exit_status) if exit_status.success() => {
                println!("Successfully cloned ggwave repository");
            }
            Ok(exit_status) => {
                eprintln!("Failed to clone ggwave repository: {}", exit_status);
            }
            Err(e) => {
                eprintln!("Failed to execute git clone: {}", e);
            }
        }
    } else {
        println!("ggwave directory already exists: {}", ggwave_dir.display());
    }

    // Check that the required files exist
    let header_path = ggwave_dir.join("include/ggwave/ggwave.h");
    let source_path = ggwave_dir.join("src/ggwave.cpp");

    if !header_path.exists() {
        eprintln!("ERROR: Header file not found: {}", header_path.display());
    } else {
        println!("Found header file: {}", header_path.display());
    }

    if !source_path.exists() {
        eprintln!("ERROR: Source file not found: {}", source_path.display());
    } else {
        println!("Found source file: {}", source_path.display());
    }

    // Get compiler flags
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("Target: {}", target);

    // Compile ggwave.cpp directly
    println!("Compiling ggwave.cpp...");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_else(|_| "unknown".to_string()));

    println!("OUT_DIR: {}", out_dir.display());

    let mut compiler = cc::Build::new();

    compiler
        .cpp(true)
        .file("vendors/ggwave/src/ggwave.cpp")
        .include("vendors/ggwave/include")
        .define("GGWAVE_SHARED", None) // Build with GGWAVE_SHARED defined
        .flag_if_supported("-std=c++11")
        .warnings(true) // Enable warnings to see potential issues
        .debug(true) // Include debug symbols
        .opt_level(0); // Disable optimizations for better debugging

    // Non-Windows targets need position independent code for linking into Rust
    if !target.contains("windows") {
        compiler.flag("-fPIC");
    }

    // Compile the library
    println!("Executing compiler...");
    compiler.compile("ggwave");
    println!("Compilation completed");

    // Tell cargo to statically link the library we just built
    println!("cargo:rustc-link-lib=static=ggwave");

    // Tell cargo where to find the library
    println!("cargo:rustc-link-search=native={}", out_dir.display());

    // Add C++ standard library on non-Windows platforms
    if !target.contains("windows") {
        println!("cargo:rustc-link-lib=stdc++");
    }

    // Generate bindings using bindgen
    println!("Generating bindings...");

    let bindings_builder = bindgen::Builder::default()
        .header(header_path.to_string_lossy())
        .allowlist_type("ggwave_.*")
        .allowlist_function("ggwave_.*")
        .allowlist_var("GGWAVE_.*")
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));

    let bindings = match bindings_builder.generate() {
        Ok(bindings) => {
            println!("Successfully generated bindings");
            bindings
        }
        Err(e) => {
            eprintln!("Failed to generate bindings: {}", e);
            panic!("Failed to generate bindings: {}", e);
        }
    };

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let bindings_path = out_path.join("bindings.rs");

    match bindings.write_to_file(&bindings_path) {
        Ok(_) => println!("Successfully wrote bindings to {}", bindings_path.display()),
        Err(e) => {
            eprintln!("Failed to write bindings: {}", e);
            panic!("Failed to write bindings: {}", e);
        }
    }

    // Make sure we rebuild if the header changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", header_path.to_string_lossy());
    println!("cargo:rerun-if-changed={}", source_path.to_string_lossy());

    println!("build.rs completed successfully");
}
