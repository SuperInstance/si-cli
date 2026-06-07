# si-cli

> **Unified CLI for the SuperInstance ecosystem** — scan, audit, rank, graph, and generate your way through 200+ repos.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`si` is the single command-line tool for anyone working with the [SuperInstance](https://github.com/SuperInstance) ecosystem. Whether you're exploring 200 interdependent repos, auditing a new project for ecosystem readiness, or figuring out which projects should integrate — `si` does it all.

## Table of Contents

- [Install](#install)
- [Quick Start](#quick-start)
- [Commands](#commands)
  - [si scan](#si-scan)
  - [si graph](#si-graph)
  - [si rank](#si-rank)
  - [si audit](#si-audit)
  - [si suggest](#si-suggest)
  - [si generate](#si-generate)
  - [si check](#si-check)
  - [si version](#si-version)
- [The Full Workflow](#the-full-workflow)
- [CAPABILITY.toml Spec](#capabilitytoml-spec)
- [Fleet.toml Spec](#fleettoml-spec)
- [Architecture](#architecture)
- [Connecting to Runtimes](#connecting-to-runtimes)
- [Development](#development)
- [License](#license)

## Install

```bash
# From source
git clone https://github.com/SuperInstance/si-cli.git
cd si-cli
cargo install --path .

# Or directly from GitHub
cargo install --git https://github.com/SuperInstance/si-cli
```

Requires Rust 1.70+.

```bash
# Verify
si version
# si-cli v0.1.0
#   Part of the SuperInstance ecosystem ⟁
```

## Quick Start

```bash
# Scan your ecosystem for capabilities
si scan ./my-ecosystem

# Visualize dependencies
si graph ./my-ecosystem

# Find the most important repos
si rank ./my-ecosystem

# Audit a repo for readiness
si audit ./my-ecosystem/my-repo

# Get integration suggestions
si suggest ./my-ecosystem

# Generate a new CAPABILITY.toml
si generate capability my-new-project

# Verify conservation laws in a fleet config
si check ./fleet-config
```

## Commands

### `si scan`

Recursively scan a directory for `CAPABILITY.toml` files, parse them, and print a table of discovered capabilities. Exit code 0 if all dependencies are satisfied, 1 if any dependency is missing.

```bash
si scan ./ecosystem
```

**Output:**

```
Scanning: ./ecosystem
NAME                           VERSION      PROVIDES                                 REQUIRES
──────────────────────────────────────────────────────────────────────────────────────────────────
si-core                        1.0.0        agent-runtime, conservation              —
si-auth                        0.3.0        auth                                     agent-runtime
si-storage                     0.2.0        storage, persistence                     agent-runtime
si-api                         0.5.0        api, rest                                auth, storage
si-web                         0.1.0        web, frontend                            api
──────────────────────────────────────────────────────────────────────────────────────────────────
5 capabilities discovered

✓ All dependencies satisfied.
```

If a dependency is missing:

```
✗ 2 unsatisfied dependencies:
  si-web requires api (not provided by any repo)
  si-frontend requires auth (not provided by any repo)
```

**Exit codes:** 0 = all satisfied, 1 = missing dependencies.

### `si graph`

Read all `CAPABILITY.toml` files and build a dependency graph. Output in three formats:

```bash
# ASCII art (default)
si graph ./ecosystem

# Graphviz DOT format
si graph ./ecosystem --format dot

# JSON adjacency list
si graph ./ecosystem --format json
```

**ASCII output:**

```
Dependency Graph
════════════════════════════════════════════════════════════
  si-core (no dependencies)
  si-auth depends on:
    └─► si-core
  si-storage depends on:
    └─► si-core
  si-api depends on:
    └─► si-auth
    └─► si-storage
  si-web depends on:
    └─► si-api

Reverse Dependencies
════════════════════════════════════════════════════════════
  si-core ← used by: si-auth, si-storage
  si-auth ← used by: si-api
  si-storage ← used by: si-api
  si-api ← used by: si-web
  si-web (nothing depends on it)
```

**DOT output** (pipe to Graphviz):

```bash
si graph ./ecosystem --format dot | dot -Tpng > deps.png
```

**JSON output:**

```json
{
  "si-api": ["si-auth", "si-storage"],
  "si-auth": ["si-core"],
  "si-core": [],
  "si-storage": ["si-core"],
  "si-web": ["si-api"]
}
```

### `si rank`

Rank repos by importance using spectral analysis (power iteration / PageRank-style). The ranking considers:

- **How many other repos depend on this one** (in-degree centrality)
- **How many capabilities it provides** (breadth)
- **Maturity signals** (has tests, has CI, has README > 100 lines)

```bash
si rank ./ecosystem
```

**Output:**

```
Ecosystem Importance Ranking
══════════════════════════════════════════════════════════════════════
  Rank Name                       Score     Dep      Prov  Tests CI     Spectral
  ──────────────────────────────────────────────────────────────────────────────
  ★ #1  si-core                    45.0     2        2      ✓    ✗      0.321543
  ● #2  si-auth                    25.0     1        1      ✓    ✓      0.218732
  ● #3  si-storage                 25.0     1        2      ✓    ✗      0.198451
    #4  si-api                     20.0     0        2      ✗    ✗      0.145892
    #5  si-web                     15.0     0        2      ✗    ✗      0.115382
  ──────────────────────────────────────────────────────────────────────────────
```

The ★ marks the #1 ranked repo (most critical to the ecosystem). Repos in the top 3 get a ● badge.

### `si audit`

Check a repo (or all repos in a directory) for ecosystem readiness. Scores 0-100 based on:

| Check | Points |
|-------|--------|
| Has valid CAPABILITY.toml | 25 |
| Has INTEGRATION.md (> 50 lines) | 15 |
| Has README.md (> 100 lines) | 15 |
| Has tests | 20 |
| Has CI (.github/workflows) | 15 |
| Has license file | 10 |

```bash
# Audit a single repo
si audit ./ecosystem/si-core

# Audit all repos in a directory
si audit ./ecosystem
```

**Output:**

```
Audit: si-core
════════════════════════════════════════════════════════════
  ✓ CAPABILITY.toml          Valid — provides 2 capabilities, requires 0  [25pts]
  ✓ INTEGRATION.md           78 lines                                      [15pts]
  ✓ README.md                245 lines                                     [15pts]
  ✓ Tests                    12 test(s) found                              [20pts]
  ✓ CI/CD                    2 workflow(s)                                 [15pts]
  ✓ License                  License file present                          [10pts]
──────────────────────────────────────────────────────────
  Score: 100/100
  Grade: A
```

**Exit codes:** 0 = all repos ≥ 50, 1 = any repo < 50.

### `si suggest`

Based on capability matching, suggest which repos should integrate. If repo A provides "conservation" and repo B requires "conservation", the tool suggests the integration.

```bash
si suggest ./ecosystem
```

**Output:**

```
Integration Suggestions
══════════════════════════════════════════════════════════════════════

  Capability: agent-runtime
    → si-auth should integrate with si-core
      si-core provides 'agent-runtime', si-auth requires 'agent-runtime'
    → si-storage should integrate with si-core
      si-core provides 'agent-runtime', si-storage requires 'agent-runtime'

  Capability: api
    → si-web should integrate with si-api
      si-api provides 'api', si-web requires 'api'

  Capability: auth
    → si-api should integrate with si-auth
      si-auth provides 'auth', si-api requires 'auth'

  ─ 4 suggestion(s) total
```

### `si generate`

Generate template files for new projects.

```bash
# Generate a CAPABILITY.toml template (non-interactive)
si generate capability my-new-project

# Interactive mode — prompts for fields
si generate capability my-new-project --interactive

# Specify output path
si generate capability my-new-project --output ./my-repo/CAPABILITY.toml
```

**Non-interactive output:**

```toml
# CAPABILITY.toml — SuperInstance Ecosystem
# Generated by si-cli

name = "my-new-project"
version = "0.1.0"

provides = []

requires = []
```

**Interactive mode:**

```
Version [0.1.0]: 1.0.0
Description: My awesome project
Provides (one per line, empty line to finish):
  → cool-feature
  → another-feature
  →
Requires (one per line, empty line to finish):
  → agent-runtime
  →
✓ Generated CAPABILITY.toml
```

### `si check`

Verify conservation laws across a fleet configuration. Reads a `fleet.toml` file and checks that **γ + H = total** for each agent's budget.

```bash
# Check a specific fleet file
si check ./fleet.toml

# Check fleet.toml in a directory
si check ./config/
```

**Output (all passing):**

```
Conservation Law Verification (γ + H = total)
════════════════════════════════════════════════════════════
  Agent                          γ               H          Total          γ+H        Status
  ──────────────────────────────────────────────────────────────────────────────────────
  agent-1                     0.5000        0.3000        0.8000        0.8000 ✓ OK
  agent-2                     0.2000        0.6000        0.8000        0.8000 ✓ OK
  ──────────────────────────────────────────────────────────────────────────────────────
  ✓ All 2 agents pass conservation checks.
```

**Output (with violation):**

```
  ✗ agent-bad          0.5000        0.3000        1.0000        0.8000 ✗ Δ=0.200000
  ──────────────────────────────────────────────────────────────────────────────────────
  ✗ 1/3 agents FAIL conservation checks.
```

**Exit codes:** 0 = all pass, 1 = violations detected.

### `si version`

Print version info.

```bash
si version
# si-cli v0.1.0
#   Part of the SuperInstance ecosystem ⟁
```

## The Full Workflow

Here's a complete walkthrough of managing a SuperInstance ecosystem:

### 1. Discover What You Have

You've just cloned 50 repos into `~/superinstance`. Start by scanning:

```bash
si scan ~/superinstance
```

This shows every repo that has a `CAPABILITY.toml`, what it provides, and what it needs. If any dependency is missing, the exit code is 1 — useful in CI:

```bash
si scan ~/superinstance || echo "Missing dependencies!"
```

### 2. Map the Dependencies

Now visualize how repos connect:

```bash
# Quick ASCII view
si graph ~/superinstance

# Generate a PNG for documentation
si graph ~/superinstance --format dot | dot -Tpng > ecosystem-graph.png

# Export JSON for your own tooling
si graph ~/superinstance --format json > deps.json
```

### 3. Find the Most Important Repos

Not all repos are equal. Some are foundational — everything depends on them. Find out which:

```bash
si rank ~/superinstance
```

The top-ranked repo is your keystone. If it breaks, everything breaks. Invest in its tests, its CI, its documentation.

### 4. Audit for Readiness

Before onboarding a new repo or releasing a new version, run an audit:

```bash
si audit ~/superinstance/si-core
```

The 0-100 score tells you at a glance how "ecosystem-ready" a repo is. Anything below 50 fails CI:

```bash
si audit ~/superinstance/my-repo || exit 1
```

### 5. Discover Integration Opportunities

You've added a new repo. What should it integrate with?

```bash
si suggest ~/superinstance
```

The tool matches providers with consumers across the entire ecosystem, surfacing integration suggestions you might have missed.

### 6. Start a New Project

```bash
mkdir my-new-repo && cd my-new-repo
si generate capability my-new-repo --interactive
# Fill in the prompts...
```

### 7. Verify Fleet Configs

For agents running in production, verify that budgets are conserved:

```bash
si check ./fleet-config
```

Any violation of **γ + H = total** is flagged immediately.

### CI Integration

Put it all together in a GitHub Action:

```yaml
name: Ecosystem Health
on: [push, pull_request]
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install --git https://github.com/SuperInstance/si-cli
      - name: Scan dependencies
        run: si scan .
      - name: Audit readiness
        run: si audit .
      - name: Check conservation
        run: si check .
```

## CAPABILITY.toml Spec

Every repo in the SuperInstance ecosystem should have a `CAPABILITY.toml` at its root:

```toml
name = "si-core"
version = "1.0.0"
description = "Core agent runtime for the SuperInstance ecosystem"

provides = [
    "agent-runtime",
    "conservation",
    "budget-allocation",
]

requires = [
    "logging",
    "configuration",
]
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | ✓ | Unique identifier for this project |
| `version` | string | ✓ | Semantic version |
| `description` | string | | Human-readable description |
| `provides` | string[] | | Capabilities this project exports |
| `requires` | string[] | | Capabilities this project needs from others |

### Convention

- Capability names are lowercase, hyphen-separated: `agent-runtime`, `conservation`, `rest-api`
- A project can provide multiple capabilities
- A project can require multiple capabilities
- Circular dependencies are allowed but discouraged (the graph command will show them)

## Fleet.toml Spec

For conservation law verification, a `fleet.toml` describes agent budgets:

```toml
[fleet]
name = "production-fleet"
version = "1.0.0"

[[agents]]
name = "agent-alpha"
gamma = 0.5
h = 0.3
total = 0.8
capabilities = ["agent-runtime", "conservation"]

[[agents]]
name = "agent-beta"
gamma = 0.2
h = 0.6
total = 0.8
capabilities = ["storage", "persistence"]
```

### Conservation Law

For each agent, the following must hold:

```
γ (gamma) + H (entropy) = total (budget)
```

The `si check` command verifies this equality with a tolerance of 10⁻¹⁰.

## Architecture

```
si-cli/
├── Cargo.toml
├── src/
│   ├── main.rs          # CLI entry point, clap parser
│   ├── scan.rs          # CAPABILITY.tomL scanning & dependency checking
│   ├── graph.rs         # Dependency graph (petgraph) + DOT/ASCII/JSON output
│   ├── rank.rs          # Spectral ranking via power iteration
│   ├── audit.rs         # Ecosystem readiness audit (0-100 scoring)
│   ├── suggest.rs       # Provider-consumer matching for integration suggestions
│   ├── generate.rs      # CAPABILITY.toml template generation
│   ├── check.rs         # Fleet conservation law verification
│   └── toml.rs          # TOML parsing types & helpers
├── tests/
│   └── integration_test.rs  # 16 integration tests
└── README.md
```

### Key Design Decisions

1. **Real files, not mocks** — All commands work with actual files on disk. Tests create temp directories with real `CAPABILITY.toml` files.

2. **Colored output** — Green for good, red for errors, yellow for warnings. Respects `NO_COLOR` environment variable.

3. **Exit codes** — 0 = success, 1 = errors found (missing deps, audit failures, conservation violations), 2 = CLI misuse.

4. **Spectral ranking** — Uses power iteration (PageRank-style) on the dependency graph, augmented with maturity signals (tests, CI, docs).

5. **Petgraph** — The `petgraph` crate handles graph construction and DOT output. It's battle-tested and efficient.

## Connecting to Runtimes

The SuperInstance ecosystem spans multiple language runtimes:

- **[si-core-c](https://github.com/SuperInstance/si-core-c)** — C implementation of the core agent runtime
- **[si-runtime-js](https://github.com/SuperInstance/si-runtime-js)** — JavaScript/TypeScript runtime for Node.js and browsers
- **[si-runtime-zig](https://github.com/SuperInstance/si-runtime-zig)** — Zig runtime for high-performance systems

Each runtime has its own `CAPABILITY.toml`:

```toml
# si-core-c/CAPABILITY.toml
name = "si-core-c"
version = "0.1.0"
provides = ["agent-runtime"]
requires = []
```

When you run `si scan` across the entire ecosystem, you see how these runtimes relate:

```
NAME                           VERSION      PROVIDES                                 REQUIRES
──────────────────────────────────────────────────────────────────────────────────────────────────
si-core-c                      0.1.0        agent-runtime                            —
si-runtime-js                  0.2.0        agent-runtime, js-bindings               agent-runtime
si-runtime-zig                 0.3.0        agent-runtime, zig-native                agent-runtime
```

The `si graph` command shows which runtimes depend on the C core:

```
  si-core-c (no dependencies)
  si-runtime-js depends on:
    └─► si-core-c
  si-runtime-zig depends on:
    └─► si-core-c
```

And `si rank` confirms what you'd expect — the C core is the most important repo in the ecosystem:

```
  ★ #1  si-core-c               65.0     2        1      ✓    ✓      0.412089
  ● #2  si-runtime-js           40.0     0        2      ✓    ✓      0.298312
  ● #3  si-runtime-zig          35.0     0        2      ✓    ✗      0.289599
```

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Install locally
cargo install --path .
```

### Adding a New Command

1. Create `src/new_command.rs` with your implementation
2. Add `mod new_command;` to `src/main.rs`
3. Add a new variant to the `Commands` enum in `main.rs`
4. Add the handler in the `run()` function
5. Write tests in `tests/integration_test.rs`

## License

MIT — see [LICENSE](LICENSE) for details.
