use std::{env, fs};
use std::path::PathBuf;
use std::process::Command;

fn main() {
    fs::create_dir_all("vendors/").unwrap();

    // Clone ggwave repository if not exists
    let ggwave_dir = PathBuf::from("vendors/ggwave");
    if !ggwave_dir.exists() {
        println!("Cloning ggwave repository...");
        Command::new("git")
            .args(&[
                "clone",
                "https://github.com/ggerganov/ggwave.git",
                "--depth=1",
                "vendors/ggwave",
            ])
            .status()
            .expect("Failed to clone ggwave repository");
    }

    // Get compiler flags
    let target = env::var("TARGET").unwrap();

    // Compile ggwave.cpp directly
    println!("Compiling ggwave.cpp...");
    let mut compiler = cc::Build::new();

    compiler
        .cpp(true)
        .file("vendors/ggwave/src/ggwave.cpp")
        .include("vendors/ggwave/include")
        .define("GGWAVE_SHARED", None) // Build with GGWAVE_SHARED defined
        .flag_if_supported("-std=c++11")
        .opt_level(2)
        .warnings(false);

    // Non-Windows targets need position independent code for linking into Rust
    if !target.contains("windows") {
        compiler.flag("-fPIC");
    }

    compiler.compile("ggwave");

    // Tell cargo to statically link the library we just built
    println!("cargo:rustc-link-lib=static=ggwave");

    // Add C++ standard library on non-Windows platforms
    if !target.contains("windows") {
        println!("cargo:rustc-link-lib=stdc++");
    }

    // Generate bindings using bindgen
    println!("Generating bindings...");
    let bindings = bindgen::Builder::default()
        .header("vendors/ggwave/include/ggwave/ggwave.h")
        .allowlist_type("ggwave_.*")
        .allowlist_function("ggwave_.*")
        .allowlist_var("GGWAVE_.*")
        .derive_default(true)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Make sure we rebuild if the header changes
    println!("cargo:rerun-if-changed=vendors/ggwave/include/ggwave/ggwave.h");
    println!("cargo:rerun-if-changed=vendors/ggwave/src/ggwave.cpp");
}
