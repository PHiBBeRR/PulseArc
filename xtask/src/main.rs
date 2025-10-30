//! Development automation tasks for `PulseArc` workspace.
//!
//! Run with: `cargo xtask <command>`
//!
//! This is a CLI tool for developers, so `println!` and `eprintln!` are
//! intentionally used for user-facing output rather than structured logging.

#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::env;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let task = env::args().nth(1);

    let result = match task.as_deref() {
        Some("ci") => run_ci(),
        Some("fmt") => run_fmt(),
        Some("clippy") => run_clippy(),
        Some("test") => run_test(),
        Some("deny") => run_deny(),
        Some("audit") => run_audit(),
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
    println!("    ci        Run all CI checks (fmt, clippy, test, deny, audit)");
    println!("    fmt       Check code formatting");
    println!("    clippy    Run Clippy lints");
    println!("    test      Run all tests");
    println!("    deny      Check dependencies with cargo-deny");
    println!("    audit     Audit dependencies for security vulnerabilities");
    println!("    help      Show this help message");
}

/// Run all CI checks in sequence
fn run_ci() -> anyhow::Result<()> {
    println!("==> Running CI checks...\n");

    println!("==> Step 1/5: Checking format...");
    run_fmt()?;

    println!("\n==> Step 2/5: Running Clippy...");
    run_clippy()?;

    println!("\n==> Step 3/5: Running tests...");
    run_test()?;

    println!("\n==> Step 4/5: Checking dependencies...");
    run_deny()?;

    println!("\n==> Step 5/5: Auditing dependencies...");
    run_audit()?;

    println!("\n✓ All CI checks passed!");
    Ok(())
}

/// Check code formatting
fn run_fmt() -> anyhow::Result<()> {
    let status = Command::new("cargo").args(["fmt", "--all", "--", "--check"]).status()?;

    if !status.success() {
        anyhow::bail!("Format check failed. Run 'cargo fmt --all' to fix.");
    }

    Ok(())
}

/// Run Clippy lints
fn run_clippy() -> anyhow::Result<()> {
    let status = Command::new("cargo")
        .args(["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"])
        .status()?;

    if !status.success() {
        anyhow::bail!("Clippy found issues");
    }

    Ok(())
}

/// Run all workspace tests
fn run_test() -> anyhow::Result<()> {
    let status = Command::new("cargo").args(["test", "--workspace"]).status()?;

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
