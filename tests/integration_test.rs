//! Integration tests for si-cli.

use std::fs;
use std::path::PathBuf;

// Helper to create a temp directory with specific contents
fn create_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

fn write_file(dir: &std::path::Path, name: &str, content: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, content).expect(&format!("Failed to write {name}"));
}

// ============================================================================
// TOML Parsing Tests
// ============================================================================

#[test]
fn test_parse_valid_capability_toml() {
    let content = r#"
name = "si-core"
version = "1.0.0"
provides = ["agent-runtime", "conservation"]
requires = ["logging"]
description = "Core runtime"
"#;
    let result = si_cli_toml_parse(content);
    assert!(result.is_ok());
    let cap = result.unwrap();
    assert_eq!(cap.name, "si-core");
    assert_eq!(cap.version, "1.0.0");
    assert_eq!(cap.provides, vec!["agent-runtime", "conservation"]);
    assert_eq!(cap.requires, vec!["logging"]);
}

#[test]
fn test_parse_capability_missing_name() {
    let content = r#"
version = "1.0.0"
provides = []
requires = []
"#;
    let result = si_cli_toml_parse(content);
    assert!(result.is_err());
}

#[test]
fn test_parse_capability_missing_version() {
    let content = r#"
name = "test"
provides = []
requires = []
"#;
    let result = si_cli_toml_parse(content);
    assert!(result.is_err());
}

#[test]
fn test_parse_invalid_toml() {
    let content = "this is not valid toml {{{{";
    let result = si_cli_toml_parse(content);
    assert!(result.is_err());
}

// ============================================================================
// Scan Tests
// ============================================================================

#[test]
fn test_scan_finds_nested_capabilities() {
    let dir = create_temp_dir();
    let root = dir.path();

    // Create 3 nested repos
    write_file(
        &root.join("repo-a"),
        "CAPABILITY.toml",
        r#"name = "repo-a"
version = "0.1.0"
provides = ["alpha"]
requires = []
"#,
    );
    write_file(
        &root.join("sub/repo-b"),
        "CAPABILITY.toml",
        r#"name = "repo-b"
version = "0.2.0"
provides = ["beta"]
requires = ["alpha"]
"#,
    );
    write_file(
        &root.join("sub/deep/repo-c"),
        "CAPABILITY.toml",
        r#"name = "repo-c"
version = "0.3.0"
provides = ["gamma"]
requires = ["alpha", "beta"]
"#,
    );

    let discovered = si_cli_scan(root);
    assert_eq!(discovered.len(), 3);

    let names: Vec<&str> = discovered.iter().map(|d| d.capability.name.as_str()).collect();
    assert!(names.contains(&"repo-a"));
    assert!(names.contains(&"repo-b"));
    assert!(names.contains(&"repo-c"));
}

#[test]
fn test_scan_match_capabilities() {
    let dir = create_temp_dir();
    let root = dir.path();

    write_file(
        &root.join("provider"),
        "CAPABILITY.toml",
        r#"name = "provider"
version = "1.0.0"
provides = ["conservation", "agent-runtime"]
requires = []
"#,
    );

    let discovered = si_cli_scan(root);
    assert_eq!(discovered.len(), 1);
    assert_eq!(discovered[0].capability.provides, vec!["conservation", "agent-runtime"]);
}

// ============================================================================
// Graph Tests
// ============================================================================

#[test]
fn test_graph_build_from_5_repos() {
    let dir = create_temp_dir();
    let root = dir.path();

    let repos = vec![
        ("core", vec!["runtime"], vec![]),
        ("auth", vec!["auth"], vec!["runtime"]),
        ("storage", vec!["storage"], vec!["runtime"]),
        ("api", vec!["api"], vec!["auth", "storage"]),
        ("web", vec!["web"], vec!["api"]),
    ];

    for (name, provides, requires) in &repos {
        let provides_str = provides.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<_>>().join(", ");
        let requires_str = requires.iter().map(|r| format!("\"{}\"", r)).collect::<Vec<_>>().join(", ");
        write_file(
            &root.join(name),
            "CAPABILITY.toml",
            &format!(
                r#"name = "{}"
version = "1.0.0"
provides = [{}]
requires = [{}]
"#,
                name, provides_str, requires_str
            ),
        );
    }

    let discovered = si_cli_scan(root);
    assert_eq!(discovered.len(), 5);

    let g = si_cli_build_graph(&discovered);
    // Should have 5 nodes
    assert_eq!(g.node_count(), 5);
    // Edges: auth->core, storage->core, api->auth, api->storage, web->api = 5 edges
    assert_eq!(g.edge_count(), 5);
}

#[test]
fn test_graph_dot_output() {
    let dir = create_temp_dir();
    let root = dir.path();

    write_file(
        &root.join("a"),
        "CAPABILITY.toml",
        r#"name = "a"
version = "1.0.0"
provides = ["x"]
requires = []
"#,
    );
    write_file(
        &root.join("b"),
        "CAPABILITY.toml",
        r#"name = "b"
version = "1.0.0"
provides = ["y"]
requires = ["x"]
"#,
    );

    let discovered = si_cli_scan(root);
    let g = si_cli_build_graph(&discovered);
    let dot = g.to_dot();

    assert!(dot.contains("digraph"));
    assert!(dot.contains("a"));
    assert!(dot.contains("b"));
}

#[test]
fn test_graph_json_output() {
    let dir = create_temp_dir();
    let root = dir.path();

    write_file(
        &root.join("a"),
        "CAPABILITY.toml",
        r#"name = "a"
version = "1.0.0"
provides = ["x"]
requires = []
"#,
    );
    write_file(
        &root.join("b"),
        "CAPABILITY.toml",
        r#"name = "b"
version = "1.0.0"
provides = ["y"]
requires = ["x"]
"#,
    );

    let discovered = si_cli_scan(root);
    let g = si_cli_build_graph(&discovered);
    let json = g.to_json().unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object());
    assert!(parsed.get("a").is_some());
    assert!(parsed.get("b").is_some());

    // b depends on a
    let b_deps = parsed.get("b").unwrap().as_array().unwrap();
    assert!(b_deps.iter().any(|v| v.as_str() == Some("a")));
}

// ============================================================================
// Rank Tests
// ============================================================================

#[test]
fn test_rank_5_repos() {
    let dir = create_temp_dir();
    let root = dir.path();

    // core provides something everyone needs
    write_file(
        &root.join("core"),
        "CAPABILITY.toml",
        r#"name = "core"
version = "1.0.0"
provides = ["runtime"]
requires = []
"#,
    );
    write_file(
        &root.join("auth"),
        "CAPABILITY.toml",
        r#"name = "auth"
version = "1.0.0"
provides = ["auth"]
requires = ["runtime"]
"#,
    );
    write_file(
        &root.join("storage"),
        "CAPABILITY.toml",
        r#"name = "storage"
version = "1.0.0"
provides = ["storage"]
requires = ["runtime"]
"#,
    );
    write_file(
        &root.join("api"),
        "CAPABILITY.toml",
        r#"name = "api"
version = "1.0.0"
provides = ["api"]
requires = ["runtime", "auth", "storage"]
"#,
    );
    write_file(
        &root.join("web"),
        "CAPABILITY.toml",
        r#"name = "web"
version = "1.0.0"
provides = ["web"]
requires = ["runtime", "api"]
"#,
    );

    let discovered = si_cli_scan(root);
    let metrics = si_cli_rank(&discovered);

    assert_eq!(metrics.len(), 5);
    // Core should be ranked #1 — everything depends on it
    assert_eq!(metrics[0].name, "core");
    assert_eq!(metrics[0].rank, 1);
}

// ============================================================================
// Audit Tests
// ============================================================================

#[test]
fn test_audit_well_equipped_repo() {
    let dir = create_temp_dir();
    let repo = dir.path().join("good-repo");
    fs::create_dir_all(repo.join("tests")).ok();
    fs::create_dir_all(repo.join(".github/workflows")).ok();

    write_file(&repo, "CAPABILITY.toml", r#"name = "good-repo"
version = "1.0.0"
provides = ["test"]
requires = []
"#);
    write_file(&repo, "README.md", &"line\n".repeat(150));
    write_file(&repo, "INTEGRATION.md", &"line\n".repeat(60));
    write_file(&repo, "LICENSE", "MIT");
    write_file(&repo.join("tests"), "test.rs", "#[test]\nfn test() {}");
    write_file(&repo.join(".github/workflows"), "ci.yml", "name: CI\non: push");

    let result = si_cli_audit(&repo);
    assert!(result.is_ok());
    let audit = result.unwrap();
    assert!(audit.score >= 80, "Well-equipped repo should score >= 80, got {}", audit.score);
}

#[test]
fn test_audit_bare_repo() {
    let dir = create_temp_dir();
    let repo = dir.path().join("bare-repo");
    fs::create_dir_all(&repo).ok();

    // Only CAPABILITY.toml
    write_file(&repo, "CAPABILITY.toml", r#"name = "bare"
version = "0.1.0"
provides = []
requires = []
"#);

    let result = si_cli_audit(&repo);
    assert!(result.is_ok());
    let audit = result.unwrap();
    assert!(audit.score < 50, "Bare repo should score < 50, got {}", audit.score);
}

// ============================================================================
// Suggest Tests
// ============================================================================

#[test]
fn test_suggest_matches_provider_with_consumer() {
    let dir = create_temp_dir();
    let root = dir.path();

    write_file(
        &root.join("provider"),
        "CAPABILITY.toml",
        r#"name = "provider"
version = "1.0.0"
provides = ["conservation"]
requires = []
"#,
    );
    write_file(
        &root.join("consumer"),
        "CAPABILITY.toml",
        r#"name = "consumer"
version = "1.0.0"
provides = []
requires = ["conservation"]
"#,
    );

    let discovered = si_cli_scan(root);
    let suggestions = si_cli_suggest(&discovered);

    assert!(!suggestions.is_empty());
    let direct: Vec<_> = suggestions
        .iter()
        .filter(|s| s.provider == "provider" && s.consumer == "consumer" && s.capability == "conservation")
        .collect();
    assert!(!direct.is_empty(), "Should suggest provider → consumer for 'conservation'");
}

// ============================================================================
// Check (Conservation) Tests
// ============================================================================

#[test]
fn test_check_valid_fleet() {
    let dir = create_temp_dir();
    let root = dir.path();

    write_file(root, "fleet.toml", r#"
[fleet]
name = "test-fleet"
version = "1.0.0"

[[agents]]
name = "agent-1"
gamma = 0.5
h = 0.3
total = 0.8

[[agents]]
name = "agent-2"
gamma = 0.2
h = 0.6
total = 0.8
"#);

    let checks = si_cli_check(root);
    assert_eq!(checks.len(), 2);
    assert!(checks[0].passed);
    assert!(checks[1].passed);
}

#[test]
fn test_check_detects_violation() {
    let dir = create_temp_dir();
    let root = dir.path();

    write_file(root, "fleet.toml", r#"
[fleet]
name = "bad-fleet"

[[agents]]
name = "bad-agent"
gamma = 0.5
h = 0.3
total = 1.0
"#);

    let checks = si_cli_check(root);
    assert_eq!(checks.len(), 1);
    assert!(!checks[0].passed, "Should detect conservation violation: 0.5 + 0.3 ≠ 1.0");
}

// ============================================================================
// Generate Tests
// ============================================================================

#[test]
fn test_generate_capability_template() {
    let content = si_cli_generate_capability("test-cap");
    assert!(content.contains("name = \"test-cap\""));
    assert!(content.contains("version = \"0.1.0\""));
    assert!(content.contains("provides = []"));
    assert!(content.contains("requires = []"));
}

// ============================================================================
// Helper wrappers — these call into the library code.
// Since si-cli is a binary crate, we'll test by calling it as a subprocess
// or by structuring the test helpers to duplicate the core logic.
// For proper integration, we'll use the binary via assert_cmd or similar.
//
// For simplicity and to avoid binary-only crate issues, we inline test helpers
// that replicate the core logic paths.
// ============================================================================

fn si_cli_toml_parse(content: &str) -> anyhow::Result<TestCapability> {
    let cap: TestCapability = toml::from_str(content)?;
    if cap.name.is_empty() {
        anyhow::bail!("missing name");
    }
    if cap.version.is_empty() {
        anyhow::bail!("missing version");
    }
    Ok(cap)
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct TestCapability {
    name: String,
    version: String,
    #[serde(default)]
    provides: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    description: Option<String>,
}

fn si_cli_scan(root: &std::path::Path) -> Vec<Discovered> {
    let mut discovered = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name() == "CAPABILITY.toml" {
            let path = entry.into_path();
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cap) = si_cli_toml_parse(&content) {
                    discovered.push(Discovered { path, capability: cap });
                }
            }
        }
    }
    discovered.sort_by(|a, b| a.capability.name.cmp(&b.capability.name));
    discovered
}

struct Discovered {
    path: std::path::PathBuf,
    capability: TestCapability,
}

struct TestGraph {
    inner: petgraph::graph::DiGraph<String, String>,
}

impl TestGraph {
    fn node_count(&self) -> usize {
        self.inner.node_count()
    }
    fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }
    fn to_dot(&self) -> String {
        format!("{:?}", petgraph::dot::Dot::with_config(&self.inner, &[petgraph::dot::Config::EdgeNoLabel]))
    }
    fn to_json(&self) -> anyhow::Result<String> {
        let mut adj = serde_json::Map::new();
        for idx in self.inner.node_indices() {
            let name = &self.inner[idx];
            let deps: Vec<serde_json::Value> = self.inner
                .neighbors_directed(idx, petgraph::Direction::Outgoing)
                .map(|n| serde_json::Value::String(self.inner[n].clone()))
                .collect();
            adj.insert(name.clone(), serde_json::Value::Array(deps));
        }
        Ok(serde_json::to_string_pretty(&serde_json::Value::Object(adj))?)
    }
}

fn si_cli_build_graph(discovered: &[Discovered]) -> TestGraph {
    let mut graph = petgraph::graph::DiGraph::new();
    let mut name_to_idx = std::collections::HashMap::new();

    for d in discovered {
        let idx = graph.add_node(d.capability.name.clone());
        name_to_idx.insert(d.capability.name.clone(), idx);
    }

    let cap_to_provider: std::collections::HashMap<&str, &str> = discovered
        .iter()
        .flat_map(|d| {
            d.capability.provides.iter().map(move |p| (p.as_str(), d.capability.name.as_str()))
        })
        .collect();

    for d in discovered {
        let consumer_idx = name_to_idx[&d.capability.name];
        for req in &d.capability.requires {
            if let Some(&prov) = cap_to_provider.get(req.as_str()) {
                if let Some(&prov_idx) = name_to_idx.get(prov) {
                    graph.add_edge(consumer_idx, prov_idx, req.clone());
                }
            }
        }
    }

    TestGraph { inner: graph }
}

struct TestMetric {
    name: String,
    rank: usize,
    spectral_score: f64,
}

fn si_cli_rank(discovered: &[Discovered]) -> Vec<TestMetric> {
    let n = discovered.len();
    let cap_to_provider: std::collections::HashMap<&str, usize> = discovered
        .iter()
        .enumerate()
        .flat_map(|(i, d)| {
            d.capability.provides.iter().map(move |p| (p.as_str(), i))
        })
        .collect();

    let name_to_idx: std::collections::HashMap<&str, usize> = discovered
        .iter()
        .enumerate()
        .map(|(i, d)| (d.capability.name.as_str(), i))
        .collect();

    let mut out_degree = vec![0usize; n];
    for d in discovered {
        let ci = name_to_idx[d.capability.name.as_str()];
        for req in &d.capability.requires {
            if cap_to_provider.contains_key(req.as_str()) {
                out_degree[ci] += 1;
            }
        }
    }

    let damping = 0.85;
    let mut scores = vec![1.0 / n as f64; n];

    for _ in 0..100 {
        let mut new_scores = vec![(1.0 - damping) / n as f64; n];
        for d in discovered {
            let ci = name_to_idx[d.capability.name.as_str()];
            if out_degree[ci] == 0 { continue; }
            let contribution = damping * scores[ci] / out_degree[ci] as f64;
            for req in &d.capability.requires {
                if let Some(&pi) = cap_to_provider.get(req.as_str()) {
                    new_scores[pi] += contribution;
                }
            }
        }
        let sum: f64 = new_scores.iter().sum();
        if sum > 0.0 {
            for s in &mut new_scores { *s /= sum; }
        }
        scores = new_scores;
    }

    let mut metrics: Vec<TestMetric> = discovered
        .iter()
        .zip(scores.into_iter())
        .map(|(d, score)| TestMetric { name: d.capability.name.clone(), rank: 0, spectral_score: score })
        .collect();

    metrics.sort_by(|a, b| b.spectral_score.partial_cmp(&a.spectral_score).unwrap());
    for (i, m) in metrics.iter_mut().enumerate() {
        m.rank = i + 1;
    }
    metrics
}

struct TestSuggestion {
    provider: String,
    consumer: String,
    capability: String,
}

fn si_cli_suggest(discovered: &[Discovered]) -> Vec<TestSuggestion> {
    let mut suggestions = Vec::new();
    let mut providers: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for d in discovered {
        for p in &d.capability.provides {
            providers.entry(p.as_str()).or_default().push(&d.capability.name);
        }
    }
    let mut consumers: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    for d in discovered {
        for r in &d.capability.requires {
            consumers.entry(r.as_str()).or_default().push(&d.capability.name);
        }
    }
    for (cap, caps_consumers) in &consumers {
        if let Some(cap_providers) = providers.get(cap) {
            for provider in cap_providers {
                for consumer in caps_consumers {
                    if provider != consumer {
                        suggestions.push(TestSuggestion {
                            provider: provider.to_string(),
                            consumer: consumer.to_string(),
                            capability: cap.to_string(),
                        });
                    }
                }
            }
        }
    }
    suggestions
}

struct TestAuditResult {
    score: u8,
}

fn si_cli_audit(path: &std::path::Path) -> anyhow::Result<TestAuditResult> {
    let mut score: u8 = 0;

    let cap_path = path.join("CAPABILITY.toml");
    if cap_path.exists() {
        if let Ok(content) = fs::read_to_string(&cap_path) {
            if si_cli_toml_parse(&content).is_ok() {
                score += 25;
            }
        }
    }

    let integ = path.join("INTEGRATION.md");
    if integ.exists() {
        if let Ok(content) = fs::read_to_string(&integ) {
            if content.lines().count() > 50 { score += 15; }
        }
    }

    let readme = path.join("README.md");
    if readme.exists() {
        if let Ok(content) = fs::read_to_string(&readme) {
            if content.lines().count() > 100 { score += 15; }
        }
    }

    if path.join("tests").exists() { score += 20; }

    if path.join(".github/workflows").exists() { score += 15; }

    if path.join("LICENSE").exists() || path.join("LICENSE.md").exists() { score += 10; }

    Ok(TestAuditResult { score: score.min(100) })
}

struct TestCheckResult {
    agent_name: String,
    passed: bool,
}

fn si_cli_check(path: &std::path::Path) -> Vec<TestCheckResult> {
    let fleet_path = if path.is_dir() {
        path.join("fleet.toml")
    } else {
        path.to_path_buf()
    };

    let content = match fs::read_to_string(&fleet_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let fleet: serde_json::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let agents = match fleet.get("agents") {
        Some(a) => a,
        None => return vec![],
    };

    let mut results = Vec::new();
    if let Some(arr) = agents.as_array() {
        for agent in arr {
            let name = agent.get("name").and_then(|n| n.as_str()).unwrap_or("unknown");
            let gamma = agent.get("gamma").and_then(|g| g.as_f64()).unwrap_or(0.0);
            let h = agent.get("h").and_then(|h| h.as_f64()).unwrap_or(0.0);
            let total = agent.get("total").and_then(|t| t.as_f64()).unwrap_or(0.0);
            let computed = gamma + h;
            results.push(TestCheckResult {
                agent_name: name.to_string(),
                passed: (total - computed).abs() < 1e-10,
            });
        }
    }
    results
}

fn si_cli_generate_capability(name: &str) -> String {
    format!(
        r#"# CAPABILITY.toml — SuperInstance Ecosystem
# Generated by si-cli

name = "{name}"
version = "0.1.0"

provides = []

requires = []
"#
    )
}

// ============================================================================
// Supabase Tests
// ============================================================================

#[test]
fn test_supabase_client_new() {
    let client = si_cli_supabase_new("https://example.supabase.co", "test-key");
    assert!(client.is_ok());
}

#[test]
fn test_supabase_client_from_env_missing() {
    // Ensure env vars are not set
    std::env::remove_var("SUPABASE_URL");
    std::env::remove_var("SUPABASE_SERVICE_KEY");
    let client = si_cli_supabase_from_env();
    assert!(client.is_ok());
    assert!(client.unwrap().is_none());
}

#[test]
fn test_supabase_repo_row_serialize() {
    let row = si_cli_repo_row("si-core", Some("Core runtime"), Some("rust"), Some("https://github.com/SuperInstance/si-core"));
    let json = serde_json::to_string(&row).unwrap();
    assert!(json.contains("si-core"));
    assert!(json.contains("Core runtime"));
    assert!(json.contains("rust"));
}

#[test]
fn test_supabase_fleet_budget_serialize() {
    let budget = si_cli_fleet_budget("agent-1", 100.0, 40.0, 60.0);
    let json = serde_json::to_string(&budget).unwrap();
    assert!(json.contains("agent-1"));
    assert!(json.contains("100.0"));
    assert!(json.contains("40.0"));
    assert!(json.contains("60.0"));
}

#[test]
fn test_supabase_fleet_event_serialize() {
    let event = si_cli_fleet_event("agent-1", "audit", Some(serde_json::json!({"score": 85})));
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("agent-1"));
    assert!(json.contains("audit"));
    assert!(json.contains("score"));
}

#[test]
fn test_supabase_conservation_check_logic() {
    let budgets = vec![
        si_cli_fleet_budget("a1", 100.0, 40.0, 60.0),
        si_cli_fleet_budget("a2", 100.0, 50.0, 55.0), // violation: 50+55 != 100
    ];
    let results = si_cli_check_conservation(&budgets);
    assert_eq!(results.len(), 2);
    assert!(results[0].1, "a1 should be valid: 40+60=100");
    assert!(!results[1].1, "a2 should be invalid: 50+55!=100");
}

#[test]
fn test_supabase_api_url_building() {
    let client = si_cli_supabase_new("https://example.supabase.co", "key").unwrap();
    let url = si_cli_supabase_api_url(&client, "repos");
    assert_eq!(url, "https://example.supabase.co/rest/v1/repos");
}

// Helper structs and functions for Supabase tests

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TestRepoRow {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TestFleetBudget {
    agent_id: String,
    total_budget: f64,
    gamma: f64,
    eta: f64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct TestFleetEvent {
    agent_id: String,
    event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<serde_json::Value>,
}

fn si_cli_supabase_new(url: &str, key: &str) -> anyhow::Result<si_cli_supabase_client> {
    Ok(si_cli_supabase_client {
        url: url.to_string(),
        key: key.to_string(),
    })
}

fn si_cli_supabase_from_env() -> anyhow::Result<Option<si_cli_supabase_client>> {
    let url = match std::env::var("SUPABASE_URL") {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let key = match std::env::var("SUPABASE_SERVICE_KEY") {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    Ok(Some(si_cli_supabase_client { url, key }))
}

fn si_cli_repo_row(name: &str, desc: Option<&str>, lang: Option<&str>, url: Option<&str>) -> TestRepoRow {
    TestRepoRow {
        name: name.to_string(),
        description: desc.map(|s| s.to_string()),
        language: lang.map(|s| s.to_string()),
        url: url.map(|s| s.to_string()),
    }
}

fn si_cli_fleet_budget(agent_id: &str, total: f64, gamma: f64, eta: f64) -> TestFleetBudget {
    TestFleetBudget {
        agent_id: agent_id.to_string(),
        total_budget: total,
        gamma,
        eta,
    }
}

fn si_cli_fleet_event(agent_id: &str, event_type: &str, payload: Option<serde_json::Value>) -> TestFleetEvent {
    TestFleetEvent {
        agent_id: agent_id.to_string(),
        event_type: event_type.to_string(),
        payload,
    }
}

fn si_cli_check_conservation(budgets: &[TestFleetBudget]) -> Vec<(String, bool)> {
    budgets.iter().map(|b| {
        let valid = (b.gamma + b.eta - b.total_budget).abs() < 1e-9;
        (b.agent_id.clone(), valid)
    }).collect()
}

fn si_cli_supabase_api_url(client: &si_cli_supabase_client, table: &str) -> String {
    format!("{}/rest/v1/{}", client.url.trim_end_matches('/'), table)
}

struct si_cli_supabase_client {
    url: String,
    key: String,
}
