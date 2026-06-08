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



use crate::supabase::SupabaseClient;

/// Sync discovered capabilities to the Supabase `repos` table.
/// Returns the number of repos successfully upserted.
pub fn sync_to_supabase(
    client: &SupabaseClient,
    discovered: &[DiscoveredCapability],
) -> anyhow::Result<usize> {
    let mut count = 0;
    for dc in discovered {
        let parent = dc.path.parent().unwrap_or(std::path::Path::new("."));
        let name = dc.capability.name.clone();
        let description = dc.capability.description.clone();

        // Attempt to detect language from file extensions in the repo
        let language = detect_language(parent);

        // Guess a GitHub URL from repo name (best effort)
        let url = Some(format!(
            "https://github.com/SuperInstance/{}",
            parent.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&name)
        ));

        client.upsert_repo(&name, description.as_deref(), language.as_deref(), url.as_deref())?;
        count += 1;
    }
    Ok(count)
}

fn detect_language(dir: &std::path::Path) -> Option<String> {
    let mut has_rs = false;
    let mut has_py = false;
    let mut has_go = false;
    let mut has_js = false;
    let mut has_zig = false;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    match ext {
                        "rs" => has_rs = true,
                        "py" => has_py = true,
                        "go" => has_go = true,
                        "js" | "ts" => has_js = true,
                        "zig" => has_zig = true,
                        _ => {}
                    }
                }
            }
        }
    }

    // Return primary language based on what we found
    if has_rs { Some("rust".to_string()) }
    else if has_py { Some("python".to_string()) }
    else if has_go { Some("go".to_string()) }
    else if has_js { Some("javascript".to_string()) }
    else if has_zig { Some("zig".to_string()) }
    else { None }
}
