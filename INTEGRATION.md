# Integration Guide: si-cli

## What This Tool Provides

`si-cli` is the unified command-line interface for the SuperInstance ecosystem. It discovers, audits, ranks, and validates fleet repos through CAPABILITY.toml parsing, spectral analysis, and conservation-law verification.

- **`si scan <path>`** — Recursively scan for `CAPABILITY.toml` files, parse capabilities, check dependencies, and sync to Supabase fleet registry.
- **`si graph <path> --format ascii|dot|json`** — Build a dependency graph from discovered capabilities. Output formats: ASCII art, Graphviz DOT, JSON adjacency list.
- **`si rank <path> --from-supabase`** — Rank repos by importance using spectral PageRank-style analysis on the capability dependency graph. Supports local repos or Supabase `fleet_budgets`.
- **`si audit <path>`** — Audit ecosystem readiness: CAPABILITY.toml presence, INTEGRATION.md length, README.md length, tests, CI/CD, license. Scores 0–100 with grade A–F.
- **`si suggest <path>`** — Suggest integrations between repos based on capability matching (direct requires/provides and potential complementary fits).
- **`si generate capability <name>`** — Generate a `CAPABILITY.toml` template interactively or non-interactively.
- **`si check <path> --from-supabase`** — Verify conservation laws (`γ + η = total`) in `fleet.toml` configs or Supabase `fleet_budgets`.
- **`si version`** — Print version and ecosystem info.

### Core Modules

- **`scan`** — `scan_directory()`, `check_dependencies()`, `print_scan_table()`, `print_dependency_check()`, `sync_to_supabase()`, `detect_language()`
- **`graph`** — `build_graph()`, `CapabilityGraph::build()`, `GraphFormat` (Ascii/Dot/Json), `to_ascii()`, `to_dot()`, `to_json()`
- **`rank`** — `compute_metrics()`, `rank_path()`, `rank_from_supabase()`, `RepoMetrics`, `print_ranked()`, `has_tests_in_dir()`, `is_readme_over_lines()`
- **`audit`** — `audit_repo()`, `audit_all()`, `log_audit_to_supabase()`, `AuditResult`, `Check`, `count_tests()`, `print_audit()`
- **`suggest`** — `find_suggestions()`, `suggest_path()`, `Suggestion`, `print_suggestions()`
- **`generate`** — `generate_capability()`, `write_capability_template()`
- **`check`** — `check_fleet()`, `check_conservation()`, `ConservationCheck`, `print_conservation()`
- **`supabase`** — `SupabaseClient::from_env()`, `SupabaseClient::new()`, `upsert_repo()`, `get_fleet_budgets()`, `insert_fleet_event()`, `check_conservation()`, `RepoRow`, `FleetBudget`, `FleetEvent`
- **`toml`** — `parse_capability_file()`, `parse_fleet_file()`, `CapabilityToml`, `FleetToml`, `DiscoveredCapability`

## How to Install

```bash
cargo install --path .
# or
si --version
```

## Cross-Repo Connections

### With `conservation-law-rs`: Conservation Verification

Validate that every agent's budget satisfies `γ + η = total`, the core conservation law:

```rust
use si_cli::check::{check_fleet, ConservationCheck};

fn verify_fleet_conservation(fleet_path: &std::path::Path) {
    let checks = check_fleet(fleet_path).unwrap();
    for check in &checks {
        println!("{}: γ={:.4} η={:.4} total={:.4} → {}",
            check.agent_name,
            check.gamma,
            check.h,
            check.total,
            if check.passed { "PASS" } else { "FAIL" }
        );
    }
}
```

### With `si-fleet-api`: REST-Driven Ranking

Rank agents by querying the fleet API's budget data and running spectral analysis:

```rust
use si_cli::rank::rank_from_supabase;
use si_cli::supabase::SupabaseClient;

fn rank_via_api() {
    let client = SupabaseClient::from_env().unwrap().unwrap();
    let metrics = rank_from_supabase(&client).unwrap();
    for m in &metrics {
        println!("#{} {} — spectral score: {:.6}", m.rank, m.name, m.spectral_score);
    }
}
```

### With `ecosystem-dashboard`: Supabase Sync

Push discovered repos to Supabase so the dashboard can render them:

```rust
use si_cli::scan::{scan_directory, sync_to_supabase};
use si_cli::supabase::SupabaseClient;

fn sync_for_dashboard(workspace: &std::path::Path) {
    let discovered = scan_directory(workspace).unwrap();
    let client = SupabaseClient::from_env().unwrap().unwrap();
    let count = sync_to_supabase(&client, &discovered).unwrap();
    println!("Synced {} repos to dashboard", count);
}
```

### With Supabase: Fleet Registry Operations

Use the built-in Supabase client for CRUD on fleet tables:

```rust
use si_cli::supabase::SupabaseClient;

fn fleet_registry_ops() {
    let client = SupabaseClient::from_env().unwrap().unwrap();
    
    // Upsert repo metadata
    client.upsert_repo(
        "my-agent",
        Some("An experimental agent"),
        Some("rust"),
        Some("https://github.com/SuperInstance/my-agent"),
    ).unwrap();
    
    // Query budgets for conservation checks
    let budgets = client.get_fleet_budgets().unwrap();
    for b in &budgets {
        let valid = (b.gamma + b.eta - b.total_budget).abs() < 1e-9;
        println!("{} conservation: {}", b.agent_id, valid);
    }
    
    // Log audit events
    client.insert_fleet_event(
        "my-agent",
        "audit",
        Some(serde_json::json!({"score": 92})),
    ).unwrap();
}
```

## Design Patterns

### Pattern: CI Gate with Audit

Block CI if any repo scores below 50 on the audit:

```bash
si audit ./repos
# Exits with code 1 if any repo score < 50
```

### Pattern: Pre-Commit Capability Generation

Generate CAPABILITY.toml before committing:

```bash
si generate capability my-new-agent --interactive
```

### Pattern: Dependency Health Check

Run scan + dependency check as a nightly cron:

```bash
si scan ./workspace
# Automatically syncs to Supabase if SUPABASE_URL is set
```

### Pattern: Spectral Ranking for Prioritization

Use PageRank-style spectral scores to decide which repos need attention first:

```bash
si rank ./workspace --from-supabase
```

### With `fleet-warden-rs`: Disk Health Integration

Trigger disk cleanup when the si-cli audit detects bloated repos:

```rust
use si_cli::audit::{audit_repo, AuditResult};
use fleet_warden::scanner::full_scan;

fn audit_with_disk_check(repo_path: &std::path::Path) {
    let audit = audit_repo(repo_path).unwrap();
    if audit.score < 70 {
        let scan = full_scan().unwrap();
        println!("Repo score low ({}/100). Cleanable disk space: {} bytes",
            audit.score, scan.total_cleanable());
    }
}
```

### With `agent-homeostasis-rs`: Regulation Metrics Audit

Export homeostatic sensor readings as fleet events via si-cli:

```rust
use si_cli::supabase::SupabaseClient;
use agent_homeostasis::SensorReading;

fn log_homeostasis_metrics(readings: &[SensorReading]) {
    let client = SupabaseClient::from_env().unwrap().unwrap();
    for r in readings {
        client.insert_fleet_event(
            &r.sensor_name,
            "homeostasis",
            Some(serde_json::json!({
                "value": r.value,
                "raw": r.raw_value,
            })),
        ).unwrap();
    }
}
```

## Design Patterns

### Pattern: Batch Audit with Supabase Logging

Audit all repos and stream results to Supabase:

```rust
use si_cli::audit::{audit_all, log_audit_to_supabase};
use si_cli::supabase::SupabaseClient;

fn batch_audit_with_logging(workspace: &std::path::Path) {
    let results = audit_all(workspace).unwrap();
    let client = SupabaseClient::from_env().unwrap().unwrap();
    for result in &results {
        log_audit_to_supabase(&client, result).ok();
    }
}
```

### Pattern: Capability-Driven Refactoring

Use `si suggest` output to plan cross-repo refactoring:

```bash
si suggest ./workspace > refactor-plan.json
# Read suggestions and create integration tickets
```
