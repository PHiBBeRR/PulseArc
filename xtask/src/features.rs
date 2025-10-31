use std::process::Command;

use anyhow::{Context, Result};

const FEATURE_COMBINATIONS: &[&[&str]] = &[
    &[], // default
    &["calendar"],
    &["sap"],
    &["calendar", "sap"],
    &["ml"],
    &["graphql"],
    &["tree-classifier"],
    &["sap", "ml", "graphql"],
    &["calendar", "sap", "ml"],
    &["calendar", "sap", "ml", "graphql"],
];

/// Check that all required feature combinations compile successfully.
pub fn test_feature_matrix() -> Result<()> {
    println!("Testing {} pulsearc-infra feature combinations...", FEATURE_COMBINATIONS.len());

    for (index, features) in FEATURE_COMBINATIONS.iter().enumerate() {
        let joined = features.join(",");
        let (display_label, feature_arg) = if features.is_empty() {
            ("default".to_string(), None)
        } else {
            (joined.clone(), Some(joined))
        };

        println!(
            "\n[{}/{}] cargo check -p pulsearc-infra{}",
            index + 1,
            FEATURE_COMBINATIONS.len(),
            if feature_arg.is_none() {
                "".to_string()
            } else {
                format!(" --features {}", display_label)
            }
        );

        let mut command = Command::new("cargo");
        command.arg("check").arg("-p").arg("pulsearc-infra");

        if let Some(feature_list) = feature_arg {
            command.arg("--features").arg(feature_list);
        }

        let status = command
            .status()
            .with_context(|| format!("Failed to run cargo check for '{display_label}'"))?;

        if !status.success() {
            anyhow::bail!("Feature combination '{display_label}' failed to compile");
        }

        println!("✅ Features '{}' compiled successfully", display_label);
    }

    println!("\n✅ All {} feature combinations compile successfully!", FEATURE_COMBINATIONS.len());

    Ok(())
}
