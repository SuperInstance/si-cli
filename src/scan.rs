//! CAPABILITY.toml scanning — recursively discover and validate capabilities.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

use crate::toml::DiscoveredCapability;

/// Scan a directory recursively for CAPABILITY.toml files.
pub fn scan_directory(path: &Path) -> Result<Vec<DiscoveredCapability>> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }
    if !path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", path.display());
    }

    let mut discovered = Vec::new();

    for entry in WalkDir::new(path).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        let file_name = entry.file_name().to_string_lossy();
        if file_name == "CAPABILITY.toml" {
            let cap_path = entry.into_path();
            match crate::toml::parse_capability_file(&cap_path) {
                Ok(cap) => {
                    discovered.push(DiscoveredCapability {
                        path: cap_path.clone(),
                        capability: cap,
                    });
                }
                Err(e) => {
                    eprintln!(
                        "{} {}",
                        "warning:".yellow().bold(),
                        format!("Failed to parse {}: {e}", cap_path.display()).yellow()
                    );
                }
            }
        }
    }

    // Sort by name for deterministic output
    discovered.sort_by(|a, b| a.capability.name.cmp(&b.capability.name));
    Ok(discovered)
}

/// Check that all required capabilities are satisfied by discovered provides.
/// Returns a list of unsatisfied requirements (repo_name, missing_capability).
pub fn check_dependencies(discovered: &[DiscoveredCapability]) -> Vec<(String, String)> {
    let provided: HashMap<&str, &str> = discovered
        .iter()
        .flat_map(|dc| {
            dc.capability
                .provides
                .iter()
                .map(move |p| (p.as_str(), dc.capability.name.as_str()))
        })
        .collect();

    let mut missing = Vec::new();
    for dc in discovered {
        for req in &dc.capability.requires {
            if !provided.contains_key(req.as_str()) {
                missing.push((dc.capability.name.clone(), req.clone()));
            }
        }
    }
    missing
}

/// Print a formatted table of discovered capabilities.
pub fn print_scan_table(discovered: &[DiscoveredCapability]) {
    if discovered.is_empty() {
        println!("{}", "No CAPABILITY.toml files found.".yellow());
        return;
    }

    // Header
    println!(
        "{:<30} {:<12} {:<40} {:<40}",
        "NAME".bold(),
        "VERSION".bold(),
        "PROVIDES".bold(),
        "REQUIRES".bold(),
    );
    println!("{}", "─".repeat(122).dimmed());

    for dc in discovered {
        let provides = if dc.capability.provides.is_empty() {
            "—".dimmed().to_string()
        } else {
            dc.capability.provides.join(", ")
        };
        let requires = if dc.capability.requires.is_empty() {
            "—".dimmed().to_string()
        } else {
            dc.capability.requires.join(", ")
        };

        println!(
            "{:<30} {:<12} {:<40} {:<40}",
            dc.capability.name.cyan(),
            dc.capability.version.green(),
            provides,
            requires,
        );
    }

    println!("{}", "─".repeat(122).dimmed());
    println!(
        "{} {}",
        format!("{}", discovered.len()).bold(),
        "capabilities discovered".dimmed()
    );
}

/// Print dependency check results. Returns true if all satisfied.
pub fn print_dependency_check(missing: &[(String, String)]) -> bool {
    if missing.is_empty() {
        println!("\n{} All dependencies satisfied.", "✓".green().bold());
        return true;
    }

    println!(
        "\n{} {} unsatisfied {}:",
        "✗".red().bold(),
        missing.len().to_string().red().bold(),
        if missing.len() == 1 { "dependency" } else { "dependencies" }
    );
    for (repo, cap) in missing {
        println!(
            "  {} requires {} {}",
            repo.yellow(),
            cap.red().bold(),
            "(not provided by any repo)".to_string().dimmed()
        );
    }
    false
}


