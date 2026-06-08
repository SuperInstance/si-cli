# INTEGRATION.md — si-cli

> Unified CLI for the SuperInstance ecosystem. Scans CAPABILITY.toml files,
> verifies conservation laws, ranks repos by spectral importance, audits
> ecosystem readiness, and syncs everything to Supabase.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Supabase Integration](#supabase-integration)
3. [Capability Discovery & Scanning](#capability-discovery--scanning)
4. [Connecting to si-fleet-api](#connecting-to-si-fleet-api)
5. [Conservation Law Verification](#conservation-law-verification)
6. [Spectral Ranking](#spectral-ranking)
7. [Dependency Graph](#dependency-graph)
8. [Ecosystem Audit](#ecosystem-audit)
9. [Integration Suggestions](#integration-suggestions)
10. [Template Generation](#template-generation)
11. [Environment Variables](#environment-variables)
12. [Cross-Repo Data Flow](#cross-repo-data-flow)
13. [Usage Examples](#usage-examples)
14. [CI/CD Integration](#cicd-integration)
15. [Troubleshooting](#troubleshooting)

---

## Architecture Overview

`si-cli` is a Rust binary (`si`) built with `clap` for argument parsing,
`colored` for terminal output, `reqwest` for HTTP calls, and `serde` for
serialization. The module layout:

```
src/
├── main.rs          — CLI entry point, command dispatch
├── scan.rs          — Recursive CAPABILITY.toml discovery
├── toml.rs          — TOML parsing (CapabilityToml, FleetToml)
├── check.rs         — Conservation law verification (γ + H = total)
├── graph.rs         — Dependency graph (petgraph-based)
├── rank.rs          — Spectral importance ranking (PageRank-style)
├── audit.rs         — Ecosystem readiness scoring
├── suggest.rs       — Integration suggestion engine
├── generate.rs      — CAPABILITY.toml template generator
└── supabase.rs      — Supabase REST client
```

The CLI reads local `CAPABILITY.toml` and `fleet.toml` files, but also
communicates with the Supabase fleet registry for cloud-backed queries.

---

## Supabase Integration

### Client Setup

The `SupabaseClient` is constructed from environment variables:

```rust
use crate::supabase::SupabaseClient;

// Reads SUPABASE_URL and SUPABASE_SERVICE_KEY from env
let client = SupabaseClient::from_env()?;
```

Returns `Ok(None)` if credentials are not configured — all Supabase
features degrade gracefully to local-only mode.

### Tables Used

| Table            | Operations       | Purpose                              |
|------------------|------------------|--------------------------------------|
| `repos`          | upsert           | Sync discovered repos from scan      |
| `fleet_budgets`  | read             | Query agent budgets for ranking/check|
| `fleet_events`   | insert           | Log audit results and events         |

### Upserting Repos

After a `scan`, si-cli syncs discovered repos to the `repos` table:

```rust
// Inside cmd_scan() — automatic sync after local scan
if let Some(client) = supabase::SupabaseClient::from_env()? {
    let count = scan::sync_to_supabase(&client, &discovered)?;
    println!("  ✓ {} repos synced", count);
}
```

The `sync_to_supabase` function calls `client.upsert_repo()` for each
discovered capability, auto-detecting language from file extensions and
guessing the GitHub URL from the repo directory name.

### Querying Fleet Budgets

For `--from-supabase` flags, si-cli queries the `fleet_budgets` table:

```rust
let budgets = client.get_fleet_budgets()?;
// Returns Vec<FleetBudget> with agent_id, total_budget, gamma, eta
```

### Logging Audit Results

After an audit, results are logged as fleet events:

```rust
audit::log_audit_to_supabase(&client, &result)?;
// Inserts into fleet_events with event_type="audit"
```

---

## Capability Discovery & Scanning

The `scan` command recursively walks a directory tree looking for
`CAPABILITY.toml` files:

```rust
let discovered = scan::scan_directory(path)?;
// Returns Vec<DiscoveredCapability> with path + parsed CapabilityToml
```

### Dependency Resolution

After scanning, si-cli checks that all `requires` are satisfied by some
repo's `provides`:

```rust
let missing = scan::check_dependencies(&discovered);
// Returns Vec<(repo_name, missing_capability)>
```

Example output:

```
✗ 2 unsatisfied dependencies:
  si-fleet-api requires supabase-client (not provided by any repo)
  ecosystem-dashboard requires rest-fetch (not provided by any repo)
```

---

## Connecting to si-fleet-api

While si-cli has its own direct Supabase client, it can also interact with
the fleet indirectly through the `si-fleet-api` REST service.

### How si-cli and si-fleet-api Share Data

Both services read/write the same Supabase tables:

```
┌─────────┐         ┌──────────────┐         ┌──────────────┐
│  si-cli │ ──────► │   Supabase   │ ◄────── │ si-fleet-api │
│ (Rust)  │  upsert │    (Postgres)│  query  │  (Express)   │
└─────────┘         └──────────────┘         └──────────────┘
     │                     │                        │
     │  scan → upsert repos│                        │
     │  audit → log events │                        │
     │  rank ← read budgets│                        │
     │                     │  REST endpoints        │
     │                     │  /api/fleet/budgets    │
     │                     │  /api/fleet/audit      │
     │                     │  /api/repos            │
```

### When si-cli Uses si-fleet-api

si-cli currently talks directly to Supabase. However, the `si-fleet-api`
provides a superset of query capabilities that si-cli can consume:

- **`/api/fleet/audit`** — full fleet conservation audit with violation details
- **`/api/fleet/budgets`** — enriched budget data with conservation status
- **`/api/capabilities/resolve`** — find repos by capability needs

A future version of si-cli may add an `--api-url` flag to route queries
through si-fleet-api instead of direct Supabase access:

```bash
# Hypothetical future usage:
si rank --api-url http://localhost:3001
si check --api-url http://localhost:3001
```

---

## Conservation Law Verification

The `check` command verifies that agent budgets satisfy γ + H = total:

### Local Check (fleet.toml)

```rust
let checks = check::check_fleet(path)?;
// Parses fleet.toml, computes γ + H for each agent, compares to total
```

### Supabase Check

```bash
si check . --from-supabase
```

This calls `client.check_conservation()` which queries all `fleet_budgets`
rows and verifies `(gamma + eta - total_budget).abs() < 1e-9`.

### Check Result Structure

```rust
pub struct ConservationCheck {
    pub agent_name: String,
    pub gamma: f64,
    pub h: f64,
    pub total: f64,
    pub computed_total: f64,  // gamma + h
    pub violation: f64,        // |total - computed_total|
    pub passed: bool,          // violation < 1e-10
}
```

### Example Output

```
Conservation Law Verification (γ + H = total)
══════════════════════════════════════════════════════════════
  Agent                γ          H          Total      γ+H        Status
  ──────────────────────────────────────────────────────────────────────
  wasserstein-0        0.3500     0.6500     1.0000     1.0000     ✓ OK
  categorical-0        0.5000     0.5000     1.0000     1.0000     ✓ OK
  sunset-0             0.2000     0.8000     1.0000     1.0000     ✓ OK
  ──────────────────────────────────────────────────────────────────────
  ✓ All 3 agents pass conservation checks.
```

---

## Spectral Ranking

The `rank` command computes importance scores using PageRank-style power
iteration on the capability dependency graph.

### Local Ranking

```rust
let metrics = rank::rank_path(path)?;
// Scans CAPABILITY.toml files, builds dependency adjacency matrix,
// runs 100 iterations of power iteration with damping=0.85
```

### Supabase Ranking

```rust
let metrics = rank::rank_from_supabase(&client)?;
// Queries fleet_budgets, builds similarity matrix from gamma/total ratios,
// runs spectral analysis
```

### Metrics Computed

```rust
pub struct RepoMetrics {
    pub name: String,
    pub dependents: usize,        // how many other repos depend on this
    pub provides_count: usize,    // number of capabilities provided
    pub has_tests: bool,          // #[test] or tests/ directory found
    pub has_ci: bool,             // .github/workflows exists
    pub has_readme_over_100: bool,// README.md > 100 lines
    pub raw_score: f64,           // weighted sum of heuristics
    pub spectral_score: f64,      // PageRank-style score
    pub rank: usize,              // final position
}
```

---

## Dependency Graph

The `graph` command visualizes the capability dependency structure:

```bash
# ASCII art (default)
si graph /path/to/ecosystem

# Graphviz DOT format
si graph /path/to/ecosystem --format dot | dot -Tpng > deps.png

# JSON adjacency list
si graph /path/to/ecosystem --format json
```

The graph uses `petgraph` internally:

```rust
pub struct CapabilityGraph {
    graph: DiGraph<String, String>,  // directed graph
    _name_to_idx: HashMap<String, NodeIndex>,
}
```

Edges point from consumer → provider (consumer *depends on* provider).

---

## Ecosystem Audit

The `audit` command scores repos on a 100-point scale:

| Check                | Weight | Criteria                          |
|----------------------|--------|-----------------------------------|
| CAPABILITY.toml      | 25 pts | Exists and valid TOML             |
| INTEGRATION.md       | 15 pts | Exists and > 50 lines             |
| README.md            | 15 pts | Exists and > 100 lines            |
| Tests                | 20 pts | `#[test]`, `#[tokio::test]`, or tests/ dir |
| CI/CD                | 15 pts | `.github/workflows/` exists       |
| License              | 10 pts | LICENSE file present              |

```bash
si audit /path/to/repo           # single repo
si audit /path/to/ecosystem      # all repos with CAPABILITY.toml
```

---

## Integration Suggestions

The `suggest` command finds repos that should integrate based on
complementary capabilities:

```rust
let suggestions = suggest::suggest_path(path)?;
// Returns Vec<Suggestion> with provider, consumer, capability, reason
```

### Suggestion Types

1. **Direct match**: Provider has X in `provides`, consumer has X in `requires`
2. **Potential match**: Provider has X in `provides`, no one requires it yet

---

## Template Generation

```bash
# Non-interactive (defaults)
si generate capability my-new-repo

# Interactive (prompts for version, description, provides, requires)
si generate capability my-new-repo --interactive

# Custom output path
si generate capability my-new-repo -o ./repos/my-repo/CAPABILITY.toml
```

---

## Environment Variables

| Variable               | Required | Description                        |
|------------------------|----------|------------------------------------|
| `SUPABASE_URL`         | Optional | Supabase project URL               |
| `SUPABASE_SERVICE_KEY` | Optional | Service role key for API access    |

When both are set, si-cli syncs scan results and can query fleet data.
When unset, all commands work in local-only mode.

---

## Cross-Repo Data Flow

### si-cli → conservation-law-rs

The `check` module implements the same γ + H = total verification that
`conservation-law-rs` formalizes. The `conservation-law-rs` crate provides
the mathematical framework; si-cli's `check.rs` applies it to fleet configs:

```rust
// From check.rs — same formula as conservation-law-rs
let computed = agent.gamma + agent.h;
let violation = (agent.total - computed).abs();
let passed = violation < tolerance; // 1e-10
```

### si-cli → si-fleet-api

Both read/write the same Supabase tables. si-cli runs batch operations
(scan sync, audit logging) while si-fleet-api serves real-time queries.

### si-cli → ecosystem-dashboard

The dashboard reads from the same `repos`, `fleet_budgets`, and
`fleet_events` tables that si-cli writes to. Data synced by `si scan`
appears on the dashboard automatically.

### si-cli → wasserstein-agents / categorical-agents

These agent repos include `fleet.toml` files with agent definitions.
`si check` verifies their conservation properties:

```toml
# fleet.toml example from an agent repo
[fleet]
name = "optimal-transport-fleet"

[[agents]]
name = "wasserstein-0"
gamma = 0.35
h = 0.65
total = 1.0
```

---

## Usage Examples

### Full Ecosystem Scan

```bash
# Clone all repos into one directory
mkdir -p ~/si-ecosystem
cd ~/si-ecosystem
gh repo clone SuperInstance/si-cli
gh repo clone SuperInstance/si-fleet-api
gh repo clone SuperInstance/ecosystem-dashboard
gh repo clone SuperInstance/optimal-transport-agents-rs
gh repo clone SuperInstance/conservation-law-rs

# Scan all
si scan .
# → Discovers all CAPABILITY.toml files, checks deps, syncs to Supabase

# Generate dependency graph
si graph . --format dot | dot -Tsvg > ecosystem-graph.svg

# Rank by importance
si rank .

# Audit all
si audit .

# Suggest integrations
si suggest .
```

### CI Pipeline Integration

```yaml
# .github/workflows/ecosystem-check.yml
name: Ecosystem Check
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install si-cli
        run: cargo install --git https://github.com/SuperInstance/si-cli
      - name: Audit
        run: si audit .
      - name: Check conservation
        run: si check .
      - name: Check dependencies
        run: si scan . 2>&1 | grep "unsatisfied" && exit 1 || true
```

---

## Troubleshooting

### "No fleet.toml found"

The `check` command looks for `fleet.toml` in the given directory. Create
one with agent definitions:

```toml
[fleet]
name = "my-fleet"

[[agents]]
name = "agent-0"
gamma = 0.4
h = 0.6
total = 1.0
```

### "Supabase credentials not found"

Set environment variables:

```bash
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_KEY="your-service-role-key"
```

All commands work without Supabase — they just skip cloud sync.

### "No CAPABILITY.toml files found"

Ensure repos have a `CAPABILITY.toml` in their root. Generate one:

```bash
cd my-repo
si generate capability my-repo --interactive
```

### Audit score below 50

Check the per-check output. The most common missing items:
- **CAPABILITY.toml** (25 pts) — generate with `si generate capability`
- **Tests** (20 pts) — add a `tests/` directory or `#[test]` functions
- **INTEGRATION.md** (15 pts) — create with > 50 lines of integration docs
