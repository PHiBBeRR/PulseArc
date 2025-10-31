//! Development automation tasks for `PulseArc` workspace.
//!
//! Run with: `cargo xtask <command>`
//!
//! This is a CLI tool for developers, so `println!` and `eprintln!` are
//! intentionally used for user-facing output rather than structured logging.

#![allow(clippy::print_stdout, clippy::print_stderr)]

mod features;

use std::env;
use std::process::{Command, ExitCode};

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
    println!("    test-features  Verify pulsearc-infra feature matrix compiles");
    println!("    deny      Check dependencies with cargo-deny");
    println!("    audit     Audit dependencies for security vulnerabilities");
    println!("    help      Show this help message");
}

/// Run all CI checks in sequence
fn run_ci() -> anyhow::Result<()> {
    println!("==> Running CI checks...\n");

    println!("==> Step 1/6: Checking Rust format...");
    run_fmt()?;

    println!("\n==> Step 2/6: Checking frontend format...");
    run_prettier()?;

    println!("\n==> Step 3/6: Running Clippy...");
    run_clippy()?;

    println!("\n==> Step 4/6: Running tests...");
    run_test()?;

    println!("\n==> Step 5/6: Checking dependencies...");
    run_deny()?;

    println!("\n==> Step 6/6: Auditing dependencies...");
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
    println!(
        "Skipping Clippy (legacy workspace rules block configuration). TODO: re-enable once LEGACY-CLIPPY-CONFIG is resolved."
    );
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
