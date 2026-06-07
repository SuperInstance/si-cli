//! Dependency graph building and output formats.

use anyhow::Result;
use colored::Colorize;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::dot::{Dot, Config};
use std::collections::HashMap;
use std::path::Path;

use crate::toml::DiscoveredCapability;

/// Output format for the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum GraphFormat {
    Ascii,
    Dot,
    Json,
}

/// A dependency graph built from discovered capabilities.
pub struct CapabilityGraph {
    graph: DiGraph<String, String>,
    _name_to_idx: HashMap<String, NodeIndex>,
}

impl CapabilityGraph {
    /// Build a dependency graph from discovered capabilities.
    pub fn build(discovered: &[DiscoveredCapability]) -> Self {
        let mut graph = DiGraph::new();
        let mut name_to_idx = HashMap::new();

        // Add all nodes first
        for dc in discovered {
            let idx = graph.add_node(dc.capability.name.clone());
            name_to_idx.insert(dc.capability.name.clone(), idx);
        }

        // Build capability -> provider map
        let cap_to_provider: HashMap<&str, &str> = discovered
            .iter()
            .flat_map(|dc| {
                dc.capability
                    .provides
                    .iter()
                    .map(move |p| (p.as_str(), dc.capability.name.as_str()))
            })
            .collect();

        // Add dependency edges
        for dc in discovered {
            let consumer_idx = name_to_idx[&dc.capability.name];
            for req in &dc.capability.requires {
                if let Some(&provider_name) = cap_to_provider.get(req.as_str()) {
                    if let Some(&provider_idx) = name_to_idx.get(provider_name) {
                        graph.add_edge(consumer_idx, provider_idx, req.clone());
                    }
                }
            }
        }

        CapabilityGraph {
            graph,
            _name_to_idx: name_to_idx,
        }
    }

    /// Output as ASCII art.
    pub fn to_ascii(&self) -> String {
        let mut lines = Vec::new();
        lines.push("Dependency Graph".bold().to_string());
        lines.push("═".repeat(60).dimmed().to_string());

        if self.graph.node_count() == 0 {
            lines.push("  (empty graph)".yellow().to_string());
            return lines.join("\n");
        }

        // Show adjacency: for each node, show what it depends on
        for node_idx in self.graph.node_indices() {
            let node_name = &self.graph[node_idx];
            let deps: Vec<String> = self
                .graph
                .neighbors_directed(node_idx, petgraph::Direction::Outgoing)
                .map(|neighbor| self.graph[neighbor].clone())
                .collect();

            if deps.is_empty() {
                lines.push(format!(
                    "  {} {}",
                    node_name.cyan(),
                    "(no dependencies)".dimmed()
                ));
            } else {
                lines.push(format!("  {} {}", node_name.cyan(), "depends on:".dimmed()));
                for dep in &deps {
                    lines.push(format!("    {} {}", "└─►".green(), dep.yellow()));
                }
            }
        }

        // Reverse: who depends on each node
        lines.push(String::new());
        lines.push("Reverse Dependencies".bold().to_string());
        lines.push("═".repeat(60).dimmed().to_string());

        for node_idx in self.graph.node_indices() {
            let node_name = &self.graph[node_idx];
            let dependents: Vec<String> = self
                .graph
                .neighbors_directed(node_idx, petgraph::Direction::Incoming)
                .map(|neighbor| self.graph[neighbor].clone())
                .collect();

            if dependents.is_empty() {
                lines.push(format!(
                    "  {} {}",
                    node_name.cyan(),
                    "(nothing depends on it)".dimmed()
                ));
            } else {
                lines.push(format!("  {} {} {}", node_name.cyan(), "← used by:".dimmed(), dependents.join(", ").yellow()));
            }
        }

        lines.join("\n")
    }

    /// Output as Graphviz DOT format.
    pub fn to_dot(&self) -> String {
        format!(
            "{:?}",
            Dot::with_config(&self.graph, &[Config::EdgeNoLabel])
        )
    }

    /// Output as JSON adjacency list.
    pub fn to_json(&self) -> Result<String> {
        let mut adjacency = serde_json::Map::new();

        for node_idx in self.graph.node_indices() {
            let node_name = &self.graph[node_idx];
            let deps: Vec<serde_json::Value> = self
                .graph
                .neighbors_directed(node_idx, petgraph::Direction::Outgoing)
                .map(|neighbor| serde_json::Value::String(self.graph[neighbor].clone()))
                .collect();
            adjacency.insert(
                node_name.clone(),
                serde_json::Value::Array(deps),
            );
        }

        let obj = serde_json::Value::Object(adjacency);
        Ok(serde_json::to_string_pretty(&obj)?)
    }
}

/// Build graph from a path (convenience function).
pub fn build_graph(path: &Path) -> Result<CapabilityGraph> {
    let discovered = crate::scan::scan_directory(path)?;
    Ok(CapabilityGraph::build(&discovered))
}
