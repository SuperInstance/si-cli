//! Conservation law verification — check γ + H = total for fleet configs.

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use crate::toml::FleetToml;

/// A conservation check result for a single agent.
#[derive(Debug, Clone)]
pub struct ConservationCheck {
    pub agent_name: String,
    pub gamma: f64,
    pub h: f64,
    pub total: f64,
    pub computed_total: f64,
    pub violation: f64,
    pub passed: bool,
}

/// Check conservation laws across a fleet config.
pub fn check_conservation(fleet: &FleetToml) -> Vec<ConservationCheck> {
    let tolerance = 1e-10;

    fleet
        .agents
        .iter()
        .map(|agent| {
            let computed = agent.gamma + agent.h;
            let violation = (agent.total - computed).abs();
            let passed = violation < tolerance;

            ConservationCheck {
                agent_name: agent.name.clone(),
                gamma: agent.gamma,
                h: agent.h,
                total: agent.total,
                computed_total: computed,
                violation,
                passed,
            }
        })
        .collect()
}

/// Print conservation check results.
pub fn print_conservation(checks: &[ConservationCheck]) {
    println!("{}", "Conservation Law Verification (γ + H = total)".bold());
    println!("{}", "═".repeat(60).dimmed());

    if checks.is_empty() {
        println!("  {}", "(no agents in fleet)".yellow());
        return;
    }

    println!(
        "  {:<20} {:<10} {:<10} {:<10} {:<10} {}",
        "Agent".bold(),
        "γ".bold(),
        "H".bold(),
        "Total".bold(),
        "γ+H".bold(),
        "Status".bold(),
    );
    println!("  {}", "─".repeat(66).dimmed());

    for c in checks {
        let status = if c.passed {
            "✓ OK".green()
        } else {
            format!("✗ Δ={:.6}", c.violation).red()
        };

        println!(
            "  {:<20} {:<10.4} {:<10.4} {:<10.4} {:<10.4} {}",
            c.agent_name.cyan(),
            c.gamma,
            c.h,
            c.total,
            c.computed_total,
            status,
        );
    }

    println!("  {}", "─".repeat(66).dimmed());

    let passed_count = checks.iter().filter(|c| c.passed).count();
    let total_count = checks.len();

    if passed_count == total_count {
        println!(
            "  {} All {total_count} agents pass conservation checks.",
            "✓".green().bold()
        );
    } else {
        println!(
            "  {} {}/{} agents FAIL conservation checks.",
            "✗".red().bold(),
            total_count - passed_count,
            total_count
        );
    }
}

/// Run conservation check on a fleet.toml file.
pub fn check_fleet(path: &Path) -> Result<Vec<ConservationCheck>> {
    // Try fleet.toml first, then fall back to the given path
    let fleet_path = if path.is_dir() {
        let candidate = path.join("fleet.toml");
        if candidate.exists() {
            candidate
        } else {
            anyhow::bail!("No fleet.toml found in {}", path.display());
        }
    } else {
        path.to_path_buf()
    };

    let fleet = crate::toml::parse_fleet_file(&fleet_path)
        .with_context(|| format!("Failed to parse {}", fleet_path.display()))?;

    Ok(check_conservation(&fleet))
}
