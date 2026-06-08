//! Ecosystem readiness audit.

use anyhow::Result;
use colored::Colorize;
use std::path::Path;
/// Audit result for a single repo.
#[derive(Debug, Clone)]
pub struct AuditResult {
    pub path: String,
    pub score: u8,
    pub checks: Vec<Check>,
}

/// Individual check result.
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub weight: u8,
}

/// Run an audit on a single repo directory.
pub fn audit_repo(path: &Path) -> Result<AuditResult> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    let mut checks = Vec::new();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    // 1. CAPABILITY.toml exists and is valid (25 points)
    let cap_path = path.join("CAPABILITY.toml");
    if cap_path.exists() {
        match crate::toml::parse_capability_file(&cap_path) {
            Ok(cap) => {
                checks.push(Check {
                    name: "CAPABILITY.toml".to_string(),
                    passed: true,
                    message: format!(
                        "Valid — provides {} capabilities, requires {}",
                        cap.provides.len(),
                        cap.requires.len()
                    ),
                    weight: 25,
                });
            }
            Err(e) => {
                checks.push(Check {
                    name: "CAPABILITY.toml".to_string(),
                    passed: false,
                    message: format!("Exists but invalid: {e}"),
                    weight: 25,
                });
            }
        }
    } else {
        checks.push(Check {
            name: "CAPABILITY.toml".to_string(),
            passed: false,
            message: "Not found".to_string(),
            weight: 25,
        });
    }

    // 2. INTEGRATION.md exists and is > 50 lines (15 points)
    let integ_path = path.join("INTEGRATION.md");
    if integ_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&integ_path) {
            let line_count = content.lines().count();
            if line_count > 50 {
                checks.push(Check {
                    name: "INTEGRATION.md".to_string(),
                    passed: true,
                    message: format!("{line_count} lines"),
                    weight: 15,
                });
            } else {
                checks.push(Check {
                    name: "INTEGRATION.md".to_string(),
                    passed: false,
                    message: format!("Only {line_count} lines (need > 50)"),
                    weight: 15,
                });
            }
        }
    } else {
        checks.push(Check {
            name: "INTEGRATION.md".to_string(),
            passed: false,
            message: "Not found".to_string(),
            weight: 15,
        });
    }

    // 3. README.md exists and is > 100 lines (15 points)
    let readme_path = path.join("README.md");
    if readme_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            let line_count = content.lines().count();
            if line_count > 100 {
                checks.push(Check {
                    name: "README.md".to_string(),
                    passed: true,
                    message: format!("{line_count} lines"),
                    weight: 15,
                });
            } else {
                checks.push(Check {
                    name: "README.md".to_string(),
                    passed: false,
                    message: format!("Only {line_count} lines (need > 100)"),
                    weight: 15,
                });
            }
        }
    } else {
        checks.push(Check {
            name: "README.md".to_string(),
            passed: false,
            message: "Not found".to_string(),
            weight: 15,
        });
    }

    // 4. Tests exist (20 points)
    let test_count = count_tests(path);
    if test_count > 0 {
        checks.push(Check {
            name: "Tests".to_string(),
            passed: true,
            message: format!("{test_count} test(s) found"),
            weight: 20,
        });
    } else {
        checks.push(Check {
            name: "Tests".to_string(),
            passed: false,
            message: "No tests found".to_string(),
            weight: 20,
        });
    }

    // 5. CI exists (15 points)
    let ci_path = path.join(".github/workflows");
    let has_ci = ci_path.exists() && ci_path.is_dir();
    if has_ci {
        let workflow_count = std::fs::read_dir(&ci_path)
            .map(|r| r.count())
            .unwrap_or(0);
        checks.push(Check {
            name: "CI/CD".to_string(),
            passed: true,
            message: format!("{workflow_count} workflow(s)"),
            weight: 15,
        });
    } else {
        checks.push(Check {
            name: "CI/CD".to_string(),
            passed: false,
            message: "No .github/workflows found".to_string(),
            weight: 15,
        });
    }

    // 6. Bonus: License file (10 points)
    let has_license = path.join("LICENSE").exists()
        || path.join("LICENSE.md").exists()
        || path.join("LICENSE.txt").exists();
    if has_license {
        checks.push(Check {
            name: "License".to_string(),
            passed: true,
            message: "License file present".to_string(),
            weight: 10,
        });
    } else {
        checks.push(Check {
            name: "License".to_string(),
            passed: false,
            message: "No license file".to_string(),
            weight: 10,
        });
    }

    // Compute score
    let score: u8 = checks.iter().map(|c| if c.passed { c.weight } else { 0 }).sum();
    let score = score.min(100);

    Ok(AuditResult {
        path: name,
        score,
        checks,
    })
}

/// Count test files/functions in a directory.
fn count_tests(path: &Path) -> usize {
    let mut count = 0;

    // Check tests/ directory
    let tests_dir = path.join("tests");
    if tests_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&tests_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "rs" || e == "go" || e == "js" || e == "ts")
                {
                    count += 1;
                }
            }
        }
    }

    // Check for #[test] in src files
    if let Ok(entries) = walk_rs_files(path) {
        for rs_path in entries {
            if let Ok(content) = std::fs::read_to_string(&rs_path) {
                count += content.matches("#[test]").count();
                count += content.matches("#[tokio::test]").count();
            }
        }
    }

    count
}

fn walk_rs_files(path: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    let src_dir = path.join("src");
    if src_dir.exists() {
        for entry in walkdir::WalkDir::new(&src_dir).into_iter().filter_map(|e| e.ok()) {
            let p = entry.into_path();
            if p.extension().is_some_and(|e| e == "rs") {
                files.push(p);
            }
        }
    }
    Ok(files)
}

/// Print audit results.
pub fn print_audit(result: &AuditResult) {
    let score_color = if result.score >= 80 {
        result.score.to_string().green().bold()
    } else if result.score >= 50 {
        result.score.to_string().yellow().bold()
    } else {
        result.score.to_string().red().bold()
    };

    println!("\n{} {}", "Audit:".bold(), result.path.cyan());
    println!("{}", "═".repeat(50).dimmed());

    for check in &result.checks {
        let icon = if check.passed {
            "✓".green()
        } else {
            "✗".red()
        };
        let weight = format!("[{}pts]", check.weight);
        println!(
            "  {} {:<20} {} {}",
            icon,
            check.name.bold(),
            check.message.dimmed(),
            weight.dimmed(),
        );
    }

    println!("{}", "─".repeat(50).dimmed());
    println!("  {} {}/100", "Score:".bold(), score_color);

    let grade = match result.score {
        90..=100 => "A".green().bold(),
        80..=89 => "B".green(),
        70..=79 => "C".yellow(),
        60..=69 => "D".yellow(),
        _ => "F".red().bold(),
    };
    println!("  {} {}", "Grade:".bold(), grade);
}

/// Audit all repos discovered from a path.
pub fn audit_all(path: &Path) -> Result<Vec<AuditResult>> {
    let discovered = crate::scan::scan_directory(path)?;

    let mut results = Vec::new();
    for dc in &discovered {
        if let Some(parent) = dc.path.parent() {
            match audit_repo(parent) {
                Ok(result) => results.push(result),
                Err(e) => eprintln!(
                    "{} Failed to audit {}: {e}",
                    "warning:".yellow().bold(),
                    dc.capability.name
                ),
            }
        }
    }

    // Sort by score descending
    results.sort_by_key(|b| std::cmp::Reverse(b.score));
    Ok(results)
}

use crate::supabase::SupabaseClient;

/// Log an audit result to the Supabase `fleet_events` table.
pub fn log_audit_to_supabase(
    client: &SupabaseClient,
    result: &AuditResult,
) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "path": result.path,
        "score": result.score,
        "checks": result.checks.iter().map(|c| {
            serde_json::json!({
                "name": c.name,
                "passed": c.passed,
                "message": c.message,
                "weight": c.weight,
            })
        }).collect::<Vec<_>>(),
    });

    client.insert_fleet_event(&result.path, "audit", Some(payload))?;
    Ok(())
}
