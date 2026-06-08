# si-cli

**Unified CLI for the SuperInstance ecosystem.** Scan capabilities, build dependency graphs, rank repos by spectral importance, audit ecosystem readiness, suggest integrations, verify conservation laws, and generate templates — all from one command.

---

## Installation

```bash
# From source
git clone https://github.com/SuperInstance/si-cli.git
cd si-cli
cargo install --path .

# Verify
si --version
```

### Prerequisites

- Rust 1.70+ (edition 2021)
- Optional: `SUPABASE_URL` and `SUPABASE_SERVICE_KEY` environment variables for fleet registry sync

---

## Commands

`si` provides 8 subcommands:

| Command | Description |
|---------|-------------|
| `si scan` | Recursively discover `CAPABILITY.toml` files and check dependencies |
| `si graph` | Build and output a dependency graph (ASCII, DOT, or JSON) |
| `si rank` | Rank repos by importance using spectral analysis |
| `si audit` | Audit a repo for ecosystem readiness (scored 0–100) |
| `si suggest` | Suggest integrations between repos based on capability matching |
| `si generate` | Generate template files (`CAPABILITY.toml`) |
| `si check` | Verify conservation laws (γ + H = total) in fleet configs |
| `si version` | Print version info |

---

## Command Reference

### `si scan` — Discover Capabilities

Recursively scans a directory for `CAPABILITY.toml` files, validates them, checks that all required capabilities are satisfied by discovered providers, and optionally syncs to Supabase.

```bash
# Scan the current directory
si scan .

# Scan a specific path
si scan ~/repos/superinstance
```

**Output example:**

```
Scanning: ~/repos/superinstance
NAME                           VERSION     PROVIDES                                 REQUIRES
──────────────────────────────────────────────────────────────────────────────────────────────────
si-cli                         0.1.0       cli, fleet-management                     toml-parsing, conservation
si-fleet-api                   1.0.0       rest-api, fleet-api                       supabase-client
conservation-law               2.0.0       conservation, budget-enforcement          —
──────────────────────────────────────────────────────────────────────────────────────────────────
3 capabilities discovered

✓ All dependencies satisfied.
```

**With Supabase sync:**

When `SUPABASE_URL` and `SUPABASE_SERVICE_KEY` are set, `si scan` automatically upserts discovered repos to the `repos` table:

```bash
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_KEY="your-service-role-key"
si scan ~/repos/superinstance

# Output includes:
# Syncing to Supabase fleet registry...
#   ✓ 12 repos synced
```

**CAPABILITY.toml format:**

```toml
# CAPABILITY.toml — SuperInstance Ecosystem
name = "si-cli"
version = "0.1.0"
description = "Unified CLI for the SuperInstance ecosystem"

provides = [
    "cli",
    "fleet-management",
]

requires = [
    "toml-parsing",
    "conservation",
]
```

---

### `si graph` — Dependency Graph

Build a dependency graph from discovered capabilities. Output in three formats:

```bash
# ASCII art (default)
si graph ~/repos/superinstance

# Graphviz DOT format
si graph ~/repos/superinstance --format dot

# JSON adjacency list
si graph ~/repos/superinstance --format json
```

**ASCII output:**

```
Dependency Graph
════════════════════════════════════════════════════════════
  si-cli depends on:
    └─► conservation-law
    └─► toml-parsing
  si-fleet-api depends on:
    └─► supabase-client
  conservation-law (no dependencies)

Reverse Dependencies
════════════════════════════════════════════════════════════
  conservation-law ← used by: si-cli, si-fleet-api
  si-cli (nothing depends on it)
  si-fleet-api (nothing depends on it)
```

**DOT output** (pipe to Graphviz):

```bash
si graph ~/repos/superinstance --format dot | dot -Tpng -o graph.png
```

**JSON output:**

```json
{
  "conservation-law": [],
  "si-cli": ["conservation-law"],
  "si-fleet-api": ["supabase-client"]
}
```

---

### `si rank` — Spectral Importance Ranking

Rank repos by importance using PageRank-style spectral analysis on the capability dependency graph. Considers:

- Number of dependents (repos that depend on this one)
- Number of capabilities provided
- Whether the repo has tests, CI, and documentation
- Eigenvector centrality on the dependency graph

```bash
# Rank local repos
si rank ~/repos/superinstance

# Rank agents from Supabase fleet_budgets
si rank --from-supabase
```

**Output:**

```
Ecosystem Importance Ranking
═══════════════════════════════════════════════════════════════════════
  Rank Name                      Score    Dep      Prov   Tests  CI     Spectral
  ──────────────────────────────────────────────────────────────────────────────────
  ★ #1  conservation-law         55.0     8        3      ✓      ✓      0.284731
  ● #2  si-fleet-api             40.0     3        2      ✓      ✓      0.213054
  ● #3  si-cli                   25.0     0        2      ✓      ✓      0.178543
     #4  si-runtime-go           15.0     0        1      ✓      ✗      0.109872
     #5  ecosystem-dashboard     5.0      0        1      ✗      ✗      0.067421
  ──────────────────────────────────────────────────────────────────────────────────
```

**Scoring formula:**

| Factor | Weight |
|--------|--------|
| Dependents | 10 pts each |
| Capabilities provided | 5 pts each |
| Has tests | 15 pts |
| Has CI | 10 pts |
| README > 100 lines | 5 pts |

The spectral score uses power iteration with damping factor 0.85, plus bonus weights for provides/tests/CI/README.

---

### `si audit` — Ecosystem Readiness

Audit a repo or directory of repos for ecosystem readiness. Checks six criteria and produces a scored report (0–100).

```bash
# Audit a single repo
si audit ~/repos/superinstance/si-cli

# Audit all repos in a directory
si audit ~/repos/superinstance
```

**Scoring breakdown:**

| Check | Weight | Criteria |
|-------|--------|----------|
| CAPABILITY.toml | 25 pts | Exists and is valid TOML with `name` and `version` |
| INTEGRATION.md | 15 pts | Exists and is > 50 lines |
| README.md | 15 pts | Exists and is > 100 lines |
| Tests | 20 pts | Has `#[test]`, `tests/` dir, or other test indicators |
| CI/CD | 15 pts | Has `.github/workflows/` directory |
| License | 10 pts | Has LICENSE file |

**Output:**

```
Audit: si-cli
════════════════════════════════════════════════════════════════════
  ✓ CAPABILITY.toml         Valid — provides 2 capabilities, requires 2   [25pts]
  ✓ INTEGRATION.md          87 lines                                       [15pts]
  ✓ README.md               340 lines                                      [15pts]
  ✓ Tests                   12 test(s) found                               [20pts]
  ✓ CI/CD                   1 workflow(s)                                  [15pts]
  ✗ License                 No license file                                [10pts]
──────────────────────────────────────────────────────────────────────────
  Score: 90/100
  Grade: A
```

**Supabase logging:**

When Supabase credentials are configured, audit results are logged to the `fleet_events` table with event type `"audit"`.

```bash
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_KEY="your-service-role-key"
si audit ~/repos/superinstance
# Each audit result is logged to fleet_events
```

**Exit codes:**

- `0` — All repos score ≥ 50
- `1` — Any repo scores < 50

---

### `si suggest` — Integration Suggestions

Analyze capability graphs to suggest integrations between repos. Finds two types of suggestions:

1. **Direct matches**: Repo A provides capability X, Repo B requires X
2. **Potential integrations**: Repo A provides capability X that no one requires yet — suggest it to other repos

```bash
si suggest ~/repos/superinstance
```

**Output:**

```
Integration Suggestions
══════════════════════════════════════════════════════════════════════════

  Capability: conservation
    → conservation-law should integrate with si-cli
      conservation-law provides 'conservation', si-cli requires 'conservation'
    → conservation-law should integrate with si-fleet-api
      conservation-law provides 'conservation', si-fleet-api requires 'conservation'

  Capability: supabase-client
    → si-fleet-api should integrate with ecosystem-dashboard
      si-fleet-api provides 'supabase-client' — ecosystem-dashboard could benefit from integrating it

  ─ 5 suggestion(s) total
```

---

### `si generate` — Generate Templates

Generate `CAPABILITY.toml` templates:

```bash
# Generate a basic template
si generate capability my-new-repo

# Interactive mode — prompts for each field
si generate capability my-new-repo --interactive

# Write to a specific path
si generate capability my-new-repo -o /path/to/repo/CAPABILITY.toml
```

**Non-interactive output** (written to `CAPABILITY.toml`):

```toml
# CAPABILITY.toml — SuperInstance Ecosystem
# Generated by si-cli

name = "my-new-repo"
version = "0.1.0"

provides = []

requires = []
```

**Interactive mode:**

```
Version [0.1.0]: 2.0.0
Description: My awesome repo
Provides (one per line, empty line to finish):
  → http-server
  → rest-api
  →
Requires (one per line, empty line to finish):
  → database
  →
✓ Generated CAPABILITY.toml
```

---

### `si check` — Conservation Law Verification

Verify the conservation law **γ + H = total** in fleet configurations.

```bash
# Check a local fleet.toml file
si check /path/to/fleet.toml

# Check a directory (looks for fleet.toml inside)
si check /path/to/fleet-directory/

# Check Supabase fleet_budgets
si check --from-supabase
```

**fleet.toml format:**

```toml
[fleet]
name = "production-fleet"
version = "1.0"

[[agents]]
name = "agent-alpha"
gamma = 143.0
h = 82.0
total = 225.0
capabilities = ["compute", "network"]

[[agents]]
name = "agent-beta"
gamma = 60.0
h = 40.0
total = 100.0
capabilities = ["storage"]
```

**Output:**

```
Conservation Law Verification (γ + H = total)
════════════════════════════════════════════════════════════════════════
  Agent                          γ           H           Total       γ+H         Status
  ──────────────────────────────────────────────────────────────────────────────────────
  agent-alpha                    143.0000    82.0000     225.0000    225.0000    ✓ OK
  agent-beta                     60.0000     40.0000     100.0000    100.0000    ✓ OK
  ──────────────────────────────────────────────────────────────────────────────────────
  ✓ All 2 agents pass conservation checks.
```

**With violations:**

```
  agent-gamma                    50.0000     30.0000     100.0000    80.0000     ✗ Δ=20.000000
  ...
  ✗ 1/3 agents FAIL conservation checks.
```

**Supabase check:**

```bash
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_KEY="your-service-role-key"
si check --from-supabase

Conservation Check (Supabase)
════════════════════════════════════════
  ✓ agent-alpha gamma + eta = total? YES
  ✓ agent-beta gamma + eta = total? YES
```

---

## Supabase Integration

`si-cli` integrates with Supabase for fleet registry management. Set these environment variables:

```bash
export SUPABASE_URL="https://your-project.supabase.co"
export SUPABASE_SERVICE_KEY="eyJhbGciOiJI..."
```

**Tables used:**

| Table | Operations |
|-------|------------|
| `repos` | Upsert from `si scan` |
| `fleet_budgets` | Read from `si check --from-supabase` and `si rank --from-supabase` |
| `fleet_events` | Insert from `si audit` (event_type `"audit"`) |

**When credentials are not set**, Supabase features are silently skipped — all commands work offline.

---

## Architecture

```
src/
├── main.rs          # CLI entry point, command dispatch
├── audit.rs         # Ecosystem readiness auditing
├── check.rs         # Conservation law verification
├── generate.rs      # Template generation
├── graph.rs         # Dependency graph (ASCII/DOT/JSON)
├── rank.rs          # Spectral importance ranking
├── scan.rs          # CAPABILITY.toml discovery and Supabase sync
├── suggest.rs       # Integration suggestion engine
├── supabase.rs      # Supabase REST client
└── toml.rs          # TOML parsing for CAPABILITY.toml and fleet.toml
```

**Key types:**

```rust
// from toml.rs
pub struct CapabilityToml {
    pub name: String,
    pub version: String,
    pub provides: Vec<String>,
    pub requires: Vec<String>,
    pub description: Option<String>,
}

pub struct FleetToml {
    pub fleet: FleetMeta,
    pub agents: Vec<AgentDef>,
}

pub struct AgentDef {
    pub name: String,
    pub gamma: f64,
    pub h: f64,
    pub total: f64,
    pub capabilities: Vec<String>,
}

// from check.rs
pub struct ConservationCheck {
    pub agent_name: String,
    pub gamma: f64,
    pub h: f64,
    pub total: f64,
    pub computed_total: f64,
    pub violation: f64,
    pub passed: bool,
}

// from rank.rs
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

// from audit.rs
pub struct AuditResult {
    pub path: String,
    pub score: u8,
    pub checks: Vec<Check>,
}

// from supabase.rs
pub struct FleetBudget {
    pub agent_id: String,
    pub total_budget: f64,
    pub gamma: f64,
    pub eta: f64,
}
```

---

## Working Examples

### Full Ecosystem Scan Pipeline

```bash
#!/bin/bash
# Scan, audit, rank, and check an entire ecosystem
ECO_DIR=~/repos/superinstance

echo "=== Scanning ==="
si scan "$ECO_DIR"

echo ""
echo "=== Auditing ==="
si audit "$ECO_DIR"

echo ""
echo "=== Ranking ==="
si rank "$ECO_DIR"

echo ""
echo "=== Dependency Graph ==="
si graph "$ECO_DIR" --format json > deps.json
echo "Graph saved to deps.json"

echo ""
echo "=== Conservation Check ==="
si check "$ECO_DIR"
```

### Generate a CAPABILITY.toml for a New Repo

```bash
#!/bin/bash
# Create a new repo with proper ecosystem metadata
mkdir my-agent && cd my-agent
si generate capability my-agent --interactive
# Fill in: version, description, provides, requires
cat CAPABILITY.toml
```

### CI Integration: Fail on Low Audit Score

```yaml
# .github/workflows/ecosystem-audit.yml
name: Ecosystem Audit
on: [push]
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install si-cli
        run: cargo install --git https://github.com/SuperInstance/si-cli
      - name: Run audit
        run: si audit .  # exits 1 if score < 50
      - name: Check conservation
        run: si check .
```

### Custom Graph Visualization

```bash
# Generate DOT and render as SVG
si graph ~/repos/superinstance --format dot > ecosystem.dot
dot -Tsvg ecosystem.dot -o ecosystem.svg

# Generate JSON and filter with jq
si graph ~/repos/superinstance --format json | jq '.["conservation-law"]'
```

---

## Conservation Law

The core invariant enforced by `si check`:

```
γ + H = C

Where:
  γ (gamma) = productive energy (useful compute)
  H (eta)   = entropy / waste budget
  C         = total capacity (fixed)
```

This law is checked with tolerance `1e-10`. Any violation causes the command to exit with code 1.

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing with derive macros |
| `colored` | Terminal color output |
| `toml` | TOML parsing |
| `serde` | Serialization/deserialization |
| `serde_json` | JSON output for graphs |
| `anyhow` | Error handling |
| `reqwest` | HTTP client for Supabase |
| `petgraph` | Graph data structures and algorithms |
| `walkdir` | Recursive directory traversal |

---

## Related Repos

| Repo | Language | Description |
|------|----------|-------------|
| [`conservation-law`](https://github.com/SuperInstance/conservation-law) | Rust | Core conservation law crate |
| [`si-fleet-api`](https://github.com/SuperInstance/si-fleet-api) | TypeScript | REST API for fleet management |
| [`si-conservation-python`](https://github.com/SuperInstance/si-conservation-python) | Rust/Python | PyO3 Python bindings for conservation law |
| [`si-runtime-python`](https://github.com/SuperInstance/si-runtime-python) | Python | Pure Python runtime |
| [`si-runtime-go`](https://github.com/SuperInstance/si-runtime-go) | Go | Go runtime |
| [`ecosystem-dashboard`](https://github.com/SuperInstance/ecosystem-dashboard) | HTML/JS | Live ecosystem dashboard |
| [`agent-operations`](https://github.com/SuperInstance/agent-operations) | Docs | Strategic operations hub |

---

## License

MIT
