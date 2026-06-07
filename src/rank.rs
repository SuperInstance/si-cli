//! Spectral ranking of repos by importance.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

use crate::toml::DiscoveredCapability;

/// A repo with computed importance metrics.
#[derive(Debug, Clone)]
pub struct RepoMetrics {
    pub name: String,
    pub dependents: usize,
    pub provides_count: usize,
    pub has_tests: bool,
    pub has_ci: bool,
    pub has_readme_over_100: bool,
    pub raw_score: f64,
    pub spectral_score: f64,
    pub rank: usize,
}

/// Compute importance metrics for each discovered repo.
pub fn compute_metrics(discovered: &[DiscoveredCapability]) -> Vec<RepoMetrics> {
    // Count dependents: how many other repos require capabilities this one provides
    let provided_by: HashMap<&str, &str> = discovered
        .iter()
        .flat_map(|dc| {
            dc.capability
                .provides
                .iter()
                .map(move |p| (p.as_str(), dc.capability.name.as_str()))
        })
        .collect();

    let mut dependent_count: HashMap<&str, usize> = HashMap::new();
    for dc in discovered {
        for req in &dc.capability.requires {
            if let Some(&provider) = provided_by.get(req.as_str()) {
                *dependent_count.entry(provider).or_insert(0) += 1;
            }
        }
    }

    let mut metrics: Vec<RepoMetrics> = discovered
        .iter()
        .map(|dc| {
            let name = &dc.capability.name;
            let parent_dir = dc.path.parent().unwrap_or(Path::new("."));

            let has_tests = has_tests_in_dir(parent_dir);
            let has_ci = parent_dir.join(".github/workflows").exists();
            let has_readme_over_100 = is_readme_over_lines(parent_dir, 100);

            let dependents = *dependent_count.get(name.as_str()).unwrap_or(&0);
            let provides_count = dc.capability.provides.len();

            RepoMetrics {
                name: name.clone(),
                dependents,
                provides_count,
                has_tests,
                has_ci,
                has_readme_over_100,
                raw_score: 0.0,
                spectral_score: 0.0,
                rank: 0,
            }
        })
        .collect();

    // Build adjacency matrix for spectral ranking
    let n = metrics.len();
    if n == 0 {
        return metrics;
    }

    // Adjacency: metrics[i] depends on metrics[j] if j provides something i requires
    let name_to_idx: HashMap<&str, usize> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (m.name.as_str(), i))
        .collect();

    let cap_to_provider: HashMap<&str, usize> = discovered
        .iter()
        .flat_map(|dc| {
            let idx = name_to_idx[dc.capability.name.as_str()];
            dc.capability
                .provides
                .iter()
                .map(move |p| (p.as_str(), idx))
        })
        .collect();

    // Build transition matrix (column-stochastic for PageRank-style)
    // M[i][j] = 1/out_degree(j) if j -> i (j depends on i)
    let mut out_degree = vec![0usize; n];
    for dc in discovered {
        let consumer_idx = name_to_idx[dc.capability.name.as_str()];
        for req in &dc.capability.requires {
            if cap_to_provider.contains_key(req.as_str()) {
                out_degree[consumer_idx] += 1;
            }
        }
    }

    // Power iteration for spectral ranking
    let damping = 0.85;
    let mut scores = vec![1.0 / n as f64; n];

    for _ in 0..100 {
        let mut new_scores = vec![(1.0 - damping) / n as f64; n];

        for dc in discovered {
            let consumer_idx = name_to_idx[dc.capability.name.as_str()];
            if out_degree[consumer_idx] == 0 {
                continue;
            }
            let contribution = damping * scores[consumer_idx] / out_degree[consumer_idx] as f64;
            for req in &dc.capability.requires {
                if let Some(&provider_idx) = cap_to_provider.get(req.as_str()) {
                    new_scores[provider_idx] += contribution;
                }
            }
        }

        // Also boost by raw metrics (provides count, has_tests, etc.)
        for (i, m) in metrics.iter().enumerate() {
            let bonus = m.provides_count as f64 * 0.01
                + if m.has_tests { 0.02 } else { 0.0 }
                + if m.has_ci { 0.02 } else { 0.0 }
                + if m.has_readme_over_100 { 0.01 } else { 0.0 };
            new_scores[i] += bonus;
        }

        // Normalize
        let sum: f64 = new_scores.iter().sum();
        if sum > 0.0 {
            for s in &mut new_scores {
                *s /= sum;
            }
        }

        scores = new_scores;
    }

    // Assign scores
    for (i, score) in scores.into_iter().enumerate() {
        metrics[i].spectral_score = score;
    }

    // Compute raw score for display
    for m in &mut metrics {
        m.raw_score = m.dependents as f64 * 10.0
            + m.provides_count as f64 * 5.0
            + if m.has_tests { 15.0 } else { 0.0 }
            + if m.has_ci { 10.0 } else { 0.0 }
            + if m.has_readme_over_100 { 5.0 } else { 0.0 };
    }

    // Sort by spectral score descending
    metrics.sort_by(|a, b| b.spectral_score.partial_cmp(&a.spectral_score).unwrap_or(std::cmp::Ordering::Equal));

    // Assign ranks
    for (i, m) in metrics.iter_mut().enumerate() {
        m.rank = i + 1;
    }

    metrics
}

fn has_tests_in_dir(dir: &Path) -> bool {
    // Check common test locations
    dir.join("tests").exists()
        || dir.join("test").exists()
        || dir.join("src").join("test").exists()
        || has_rust_tests(dir)
        || has_go_tests(dir)
}

fn has_rust_tests(dir: &Path) -> bool {
    // Look for #[test] in .rs files
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.contains("#[test]") || content.contains("#[tokio::test]") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn has_go_tests(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "go") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if content.contains("func Test") {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_readme_over_lines(dir: &Path, min_lines: usize) -> bool {
    let readme = dir.join("README.md");
    if let Ok(content) = std::fs::read_to_string(&readme) {
        content.lines().count() > min_lines
    } else {
        false
    }
}

/// Print the ranked list.
pub fn print_ranked(metrics: &[RepoMetrics]) {
    println!("{}", "Ecosystem Importance Ranking".bold());
    println!("{}", "═".repeat(70).dimmed());

    if metrics.is_empty() {
        println!("  {}", "(no repos found)".yellow());
        return;
    }

    println!(
        "  {:<5} {:<25} {:<8} {:<8} {:<6} {:<6} {:<6} {:<10}",
        "Rank".bold(),
        "Name".bold(),
        "Score".bold(),
        "Dep".bold(),
        "Prov".bold(),
        "Tests".bold(),
        "CI".bold(),
        "Spectral".bold(),
    );
    println!("  {}", "─".repeat(68).dimmed());

    for m in metrics {
        let badge = if m.rank == 1 {
            "★".yellow().to_string()
        } else if m.rank <= 3 {
            "●".green().to_string()
        } else {
            " ".to_string()
        };

        println!(
            "  {:<2} {:<3} {:<25} {:<8.1} {:<8} {:<6} {:<6} {:<6} {:<10.6}",
            badge,
            format!("#{}", m.rank).dimmed(),
            m.name.cyan(),
            m.raw_score,
            m.dependents.to_string().yellow(),
            m.provides_count.to_string().green(),
            if m.has_tests { "✓".green() } else { "✗".red() }.to_string(),
            if m.has_ci { "✓".green() } else { "✗".red() }.to_string(),
            m.spectral_score.to_string().magenta(),
        );
    }

    println!("  {}", "─".repeat(68).dimmed());
}

/// Run ranking on a directory.
pub fn rank_path(path: &Path) -> Result<Vec<RepoMetrics>> {
    let discovered = crate::scan::scan_directory(path)?;
    let metrics = compute_metrics(&discovered);
    Ok(metrics)
}
