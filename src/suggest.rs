//! Integration suggestion engine — match providers with consumers.

use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

use crate::toml::DiscoveredCapability;

/// A suggested integration between two repos.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub provider: String,
    pub consumer: String,
    pub capability: String,
    pub reason: String,
}

/// Find integration suggestions based on capability matching.
pub fn find_suggestions(discovered: &[DiscoveredCapability]) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    // Map: capability name -> list of providers
    let mut providers: HashMap<&str, Vec<&str>> = HashMap::new();
    for dc in discovered {
        for cap in &dc.capability.provides {
            providers.entry(cap.as_str()).or_default().push(&dc.capability.name);
        }
    }

    // Map: capability name -> list of consumers
    let mut consumers: HashMap<&str, Vec<&str>> = HashMap::new();
    for dc in discovered {
        for cap in &dc.capability.requires {
            consumers.entry(cap.as_str()).or_default().push(&dc.capability.name);
        }
    }

    // Direct matches: provider provides X, consumer requires X
    for (cap, cap_consumers) in &consumers {
        if let Some(cap_providers) = providers.get(cap) {
            for provider in cap_providers {
                for consumer in cap_consumers {
                    if *provider != *consumer {
                        suggestions.push(Suggestion {
                            provider: provider.to_string(),
                            consumer: consumer.to_string(),
                            capability: cap.to_string(),
                            reason: format!(
                                "{provider} provides '{cap}', {consumer} requires '{cap}'"
                            ),
                        });
                    }
                }
            }
        }
    }

    // Suggest potential integrations: if provider provides something and
    // no consumer explicitly requires it, suggest it to all repos that
    // might benefit (complementary capabilities)
    let all_names: Vec<&str> = discovered.iter().map(|dc| dc.capability.name.as_str()).collect();
    for (cap, cap_providers) in &providers {
        if !consumers.contains_key(cap) {
            // No one requires this capability yet — suggest to repos that don't have it
            for provider in cap_providers {
                for other in &all_names {
                    if *other != *provider {
                        let other_dc = discovered
                            .iter()
                            .find(|dc| dc.capability.name == *other);
                        if let Some(dc) = other_dc {
                            // Only suggest if the capability name is related to what the other repo does
                            if !dc.capability.provides.contains(&cap.to_string())
                                && !dc.capability.requires.contains(&cap.to_string())
                            {
                                suggestions.push(Suggestion {
                                    provider: provider.to_string(),
                                    consumer: other.to_string(),
                                    capability: cap.to_string(),
                                    reason: format!(
                                        "{provider} provides '{cap}' — {other} could benefit from integrating it"
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    suggestions.sort_by(|a, b| a.capability.cmp(&b.capability));
    suggestions.dedup_by(|a, b| a.provider == b.provider && a.consumer == b.consumer && a.capability == b.capability);
    suggestions
}

/// Print suggestions in a formatted table.
pub fn print_suggestions(suggestions: &[Suggestion]) {
    if suggestions.is_empty() {
        println!("{}", "No integration suggestions found.".yellow());
        return;
    }

    println!("{}", "Integration Suggestions".bold());
    println!("{}", "═".repeat(70).dimmed());

    let mut current_cap = "";
    for s in suggestions {
        if s.capability != current_cap {
            current_cap = &s.capability;
            println!("\n  {} {}", "Capability:".bold(), s.capability.magenta());
        }
        println!(
            "    {} {} {} {}",
            "→".green(),
            s.provider.cyan(),
            "should integrate with".dimmed(),
            s.consumer.yellow(),
        );
        println!("      {}", s.reason.dimmed());
    }

    println!("\n  {} {} suggestion(s) total", "─".dimmed(), suggestions.len().to_string().bold());
}

/// Run suggestions on a directory.
pub fn suggest_path(path: &Path) -> Result<Vec<Suggestion>> {
    let discovered = crate::scan::scan_directory(path)?;
    Ok(find_suggestions(&discovered))
}
