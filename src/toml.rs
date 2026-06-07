//! TOML parsing helpers for CAPABILITY.toml and fleet.toml files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parsed structure of a CAPABILITY.toml file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityToml {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Parsed structure of a fleet.toml file for conservation checking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetToml {
    pub fleet: FleetMeta,
    #[serde(default)]
    pub agents: Vec<AgentDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FleetMeta {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentDef {
    pub name: String,
    pub gamma: f64,
    pub h: f64,
    pub total: f64,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Parse a CAPABILITY.toml file from disk.
pub fn parse_capability_file(path: &Path) -> Result<CapabilityToml> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    parse_capability_str(&content)
}

/// Parse a CAPABILITY.toml from a string.
pub fn parse_capability_str(content: &str) -> Result<CapabilityToml> {
    let cap: CapabilityToml = toml::from_str(content).context("Failed to parse CAPABILITY.toml")?;
    if cap.name.is_empty() {
        anyhow::bail!("CAPABILITY.toml is missing required field: name");
    }
    if cap.version.is_empty() {
        anyhow::bail!("CAPABILITY.toml is missing required field: version");
    }
    Ok(cap)
}

/// Parse a fleet.toml file from disk.
pub fn parse_fleet_file(path: &Path) -> Result<FleetToml> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    parse_fleet_str(&content)
}

/// Parse a fleet.toml from a string.
pub fn parse_fleet_str(content: &str) -> Result<FleetToml> {
    let fleet: FleetToml = toml::from_str(content).context("Failed to parse fleet.toml")?;
    Ok(fleet)
}

/// A discovered capability with its filesystem path.
#[derive(Debug, Clone)]
pub struct DiscoveredCapability {
    pub path: std::path::PathBuf,
    pub capability: CapabilityToml,
}
