use std::path::PathBuf;

fn main() {
    // Ensure frontend dist directory exists for builds/tests
    // This prevents build failures when dist hasn't been created yet
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dist_path = manifest_dir.join("../../frontend/dist");

    if !dist_path.exists() {
        eprintln!("cargo:warning=Creating placeholder frontend/dist directory for build");
        std::fs::create_dir_all(&dist_path).expect("Failed to create dist directory");
        // Create a minimal index.html so Tauri doesn't complain
        std::fs::write(
            dist_path.join("index.html"),
            "<!DOCTYPE html><html><head><title>PulseArc</title></head><body></body></html>",
        )
        .expect("Failed to create placeholder index.html");
    }

    tauri_build::build()
}
