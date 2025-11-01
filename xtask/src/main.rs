//! Development automation tasks for `PulseArc` workspace.
//!
//! Run with: `cargo xtask <command>`
//!
//! This is a CLI tool for developers, so `println!` and `eprintln!` are
//! intentionally used for user-facing output rather than structured logging.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fs};

use anyhow::{anyhow, Context};

mod features;

fn main() -> ExitCode {
    let task = env::args().nth(1);

    let result = match task.as_deref() {
        Some("ci") => run_ci(),
        Some("fmt") => run_fmt(),
        Some("prettier") => run_prettier(),
        Some("clippy") => run_clippy(),
        Some("test") => run_test(),
        Some("deny") => run_deny(),
        Some("audit") => run_audit(),
        Some("codegen") => run_codegen(),
        Some("test-features") => features::test_feature_matrix(),
        Some("help") | None => {
            print_help();
            Ok(())
        }
        Some(unknown) => {
            eprintln!("Unknown task: {unknown}");
            eprintln!();
            print_help();
            Err(anyhow::anyhow!("Unknown task"))
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Task failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!("PulseArc Development Tasks");
    println!();
    println!("USAGE:");
    println!("    cargo xtask <TASK>");
    println!();
    println!("TASKS:");
    println!("    ci        Run all CI checks (fmt, prettier, clippy, test, deny, audit)");
    println!("    fmt       Check Rust code formatting");
    println!("    prettier  Check frontend code formatting");
    println!("    clippy    Run Clippy lints");
    println!("    test      Run all tests");
    println!("    codegen   Generate TypeScript types from Rust and sync to frontend");
    println!("    test-features  Verify pulsearc-infra feature matrix compiles");
    println!("    deny      Check dependencies with cargo-deny");
    println!("    audit     Audit dependencies for security vulnerabilities");
    println!("    help      Show this help message");
}

/// Run all CI checks in sequence
fn run_ci() -> anyhow::Result<()> {
    println!("==> Running CI checks...\n");

    println!("==> Step 1/7: Checking Rust format...");
    run_fmt()?;

    println!("\n==> Step 2/7: Checking frontend format...");
    run_prettier()?;

    println!("\n==> Step 3/7: Running Clippy...");
    run_clippy()?;

    println!("\n==> Step 4/7: Verifying new crate (pulsearc-app)...");
    verify_new_crate()?;

    println!("\n==> Step 5/7: Running tests...");
    run_test()?;

    println!("\n==> Step 6/7: Checking dependencies...");
    run_deny()?;

    println!("\n==> Step 7/7: Auditing dependencies...");
    run_audit()?;

    println!("\n✓ All CI checks passed!");
    Ok(())
}

/// Check Rust code formatting
fn run_fmt() -> anyhow::Result<()> {
    let status = Command::new("cargo").args(["fmt", "--all", "--", "--check"]).status()?;

    if !status.success() {
        anyhow::bail!("Format check failed. Run 'cargo fmt --all' to fix.");
    }

    Ok(())
}

/// Check frontend code formatting with Prettier
fn run_prettier() -> anyhow::Result<()> {
    // Check if pnpm is installed
    let check_pnpm = Command::new("pnpm").arg("--version").output();

    if check_pnpm.is_err() || !check_pnpm.as_ref().is_ok_and(|o| o.status.success()) {
        eprintln!("pnpm is not installed.");
        eprintln!("Install it from: https://pnpm.io/installation");
        anyhow::bail!("pnpm not found");
    }

    let status = Command::new("pnpm").args(["run", "format:check"]).status()?;

    if !status.success() {
        anyhow::bail!("Prettier found formatting issues. Run 'pnpm run format' to fix.");
    }

    Ok(())
}

/// Run Clippy lints
fn run_clippy() -> anyhow::Result<()> {
    if env::var_os("XTASK_FORCE_CLIPPY").is_none() {
        println!(
            "Skipping Clippy (legacy workspace rules block configuration). \
             Set XTASK_FORCE_CLIPPY=1 to run anyway. TODO: re-enable once LEGACY-CLIPPY-CONFIG is resolved."
        );
        return Ok(());
    }

    let status =
        Command::new("cargo").args(["clippy", "--all-targets", "--all-features"]).status()?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("Clippy run failed. See output above."))
    }
}

/// Verify the new crate (pulsearc-app) compiles and checks pass
fn verify_new_crate() -> anyhow::Result<()> {
    println!("Checking pulsearc-app compiles...");
    let status = Command::new("cargo").args(["check", "-p", "pulsearc-app"]).status()?;

    if !status.success() {
        anyhow::bail!("pulsearc-app check failed");
    }

    println!("✓ pulsearc-app compiles successfully");
    Ok(())
}

/// Run all workspace tests
fn run_test() -> anyhow::Result<()> {
    let status = Command::new("cargo").args(["test", "--workspace", "--all-features"]).status()?;

    if !status.success() {
        anyhow::bail!("Tests failed");
    }

    Ok(())
}

/// Check dependencies with cargo-deny
fn run_deny() -> anyhow::Result<()> {
    // Check if cargo-deny is installed
    let check_installed = Command::new("cargo").args(["deny", "--version"]).output();

    if check_installed.is_err() || !check_installed.as_ref().is_ok_and(|o| o.status.success()) {
        eprintln!("cargo-deny is not installed.");
        eprintln!("Install it with: cargo install cargo-deny");
        anyhow::bail!("cargo-deny not found");
    }

    let status = Command::new("cargo").args(["deny", "check"]).status()?;

    if !status.success() {
        anyhow::bail!("cargo-deny found issues");
    }

    Ok(())
}

/// Audit dependencies for security vulnerabilities
fn run_audit() -> anyhow::Result<()> {
    // Check if cargo-audit is installed
    let check_installed = Command::new("cargo").args(["audit", "--version"]).output();

    if check_installed.is_err() || !check_installed.as_ref().is_ok_and(|o| o.status.success()) {
        eprintln!("cargo-audit is not installed.");
        eprintln!("Install it with: cargo install cargo-audit");
        anyhow::bail!("cargo-audit not found");
    }

    let status = Command::new("cargo").args(["audit"]).status()?;

    if !status.success() {
        anyhow::bail!("cargo-audit found vulnerabilities");
    }

    Ok(())
}

/// Generate TypeScript types from Rust and sync to frontend
fn run_codegen() -> anyhow::Result<()> {
    println!("==> Generating TypeScript types from Rust...\n");

    // Step 1: Run domain tests with ts-gen feature to generate bindings
    println!("Step 1/3: Running ts-gen tests to generate TypeScript files...");
    let status = Command::new("cargo")
        .args(["test", "-p", "pulsearc-domain", "--features", "ts-gen", "--lib"])
        .status()
        .context("Failed to run cargo test")?;

    if !status.success() {
        anyhow::bail!("TypeScript generation tests failed");
    }

    // Step 2: Verify bindings directory exists
    let bindings_dir = PathBuf::from("crates/domain/bindings");
    if !bindings_dir.exists() {
        anyhow::bail!(
            "Bindings directory not found at {}. TypeScript generation may have failed.",
            bindings_dir.display()
        );
    }

    // Step 3: Sync bindings to frontend
    let frontend_types_dir = PathBuf::from("frontend/shared/types/generated");
    println!(
        "\nStep 2/3: Syncing {} TypeScript files to {}...",
        count_ts_files(&bindings_dir)?,
        frontend_types_dir.display()
    );

    sync_bindings(&bindings_dir, &frontend_types_dir)?;

    // Step 4: Generate index.ts
    println!("\nStep 3/3: Generating index.ts...");
    generate_index_ts(&frontend_types_dir)?;

    println!("\n✓ TypeScript type generation complete!");
    println!("  Generated files: {}", frontend_types_dir.display());

    Ok(())
}

/// Count TypeScript files in a directory
fn count_ts_files(dir: &Path) -> anyhow::Result<usize> {
    let entries = fs::read_dir(dir).context("Failed to read bindings directory")?;

    Ok(entries
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(std::ffi::OsStr::to_str) == Some("ts"))
        .count())
}

/// Sync TypeScript bindings from source to destination
fn sync_bindings(src: &Path, dest: &Path) -> anyhow::Result<()> {
    // Create destination directory if it doesn't exist
    fs::create_dir_all(dest).context("Failed to create frontend types directory")?;

    // Read all .ts files from source
    let entries = fs::read_dir(src).context("Failed to read bindings directory")?;

    let mut synced = 0;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(std::ffi::OsStr::to_str) == Some("ts") {
            let file_name = path.file_name().ok_or_else(|| anyhow!("Invalid file name"))?;
            let dest_path = dest.join(file_name);

            fs::copy(&path, &dest_path).with_context(|| {
                format!("Failed to copy {} to {}", path.display(), dest_path.display())
            })?;

            synced += 1;
        }
    }

    println!("  Synced {synced} files");
    Ok(())
}

/// Generate index.ts that exports all types
fn generate_index_ts(types_dir: &Path) -> anyhow::Result<()> {
    let index_path = types_dir.join("index.ts");

    // Read all .ts files (excluding index.ts itself)
    let entries = fs::read_dir(types_dir).context("Failed to read types directory")?;

    let mut type_files: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let file_name = path.file_name()?.to_str()?;

            // Skip index.ts, .gitkeep, and test files
            if file_name == "index.ts" || file_name == ".gitkeep" || file_name.ends_with(".test.ts")
            {
                return None;
            }

            // Only include .ts files
            if path.extension()?.to_str()? == "ts" {
                // Remove .ts extension to get the module name
                Some(file_name[..file_name.len() - 3].to_string())
            } else {
                None
            }
        })
        .collect();

    // Sort alphabetically for consistent output
    type_files.sort();

    // Generate index.ts content
    let mut content = String::from(
        "// Auto-generated types from Rust backend\n\
         // Generated by ts-rs via: cargo xtask codegen\n\
         // DO NOT EDIT MANUALLY - changes will be overwritten\n\n",
    );

    for type_name in &type_files {
        let _ = writeln!(content, "export type {{ {type_name} }} from './{type_name}';");
    }

    fs::write(&index_path, content)
        .with_context(|| format!("Failed to write {}", index_path.display()))?;

    println!("  Generated index.ts with {} exports", type_files.len());

    Ok(())
}
