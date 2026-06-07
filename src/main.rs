//! si-cli — Unified CLI for the SuperInstance ecosystem.

mod audit;
mod check;
mod generate;
mod graph;
mod rank;
mod scan;
mod suggest;
mod toml;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "si")]
#[command(bin_name = "si")]
#[command(about = "Unified CLI for the SuperInstance ecosystem")]
#[command(version)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Recursively scan for CAPABILITY.toml files and check dependencies
    Scan {
        /// Directory to scan
        path: PathBuf,
    },
    /// Build and output a dependency graph
    Graph {
        /// Directory to scan
        path: PathBuf,
        /// Output format: ascii, dot, or json
        #[arg(long, default_value = "ascii")]
        format: graph::GraphFormat,
    },
    /// Rank repos by importance using spectral analysis
    Rank {
        /// Directory to scan
        path: PathBuf,
    },
    /// Audit a repo for ecosystem readiness
    Audit {
        /// Path to repo or directory containing repos
        path: PathBuf,
    },
    /// Suggest integrations between repos based on capability matching
    Suggest {
        /// Directory to scan
        path: PathBuf,
    },
    /// Generate template files
    Generate {
        #[command(subcommand)]
        what: GenerateCommands,
    },
    /// Verify conservation laws (γ + H = total) in fleet configs
    Check {
        /// Path to fleet.toml or directory containing one
        path: PathBuf,
    },
    /// Print version info
    Version,
}

#[derive(Subcommand)]
enum GenerateCommands {
    /// Generate a CAPABILITY.toml template
    Capability {
        /// Name of the capability
        name: String,
        /// Output file path (default: CAPABILITY.toml in current dir)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Interactive mode — prompt for fields
        #[arg(short, long)]
        interactive: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {e:#}", "error:".red().bold());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { path } => cmd_scan(&path),
        Commands::Graph { path, format } => cmd_graph(&path, format),
        Commands::Rank { path } => cmd_rank(&path),
        Commands::Audit { path } => cmd_audit(&path),
        Commands::Suggest { path } => cmd_suggest(&path),
        Commands::Generate { what } => cmd_generate(what),
        Commands::Check { path } => cmd_check(&path),
        Commands::Version => cmd_version(),
    }
}

fn cmd_scan(path: &std::path::Path) -> Result<()> {
    println!(
        "{} {}",
        "Scanning:".bold(),
        path.display().to_string().cyan()
    );

    let discovered = scan::scan_directory(path)?;
    scan::print_scan_table(&discovered);

    let missing = scan::check_dependencies(&discovered);
    let all_ok = scan::print_dependency_check(&missing);

    if !all_ok {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_graph(path: &std::path::Path, format: graph::GraphFormat) -> Result<()> {
    let g = graph::build_graph(path)?;

    match format {
        graph::GraphFormat::Ascii => println!("{}", g.to_ascii()),
        graph::GraphFormat::Dot => println!("{}", g.to_dot()),
        graph::GraphFormat::Json => println!("{}", g.to_json()?),
    }

    Ok(())
}

fn cmd_rank(path: &std::path::Path) -> Result<()> {
    let metrics = rank::rank_path(path)?;
    rank::print_ranked(&metrics);
    Ok(())
}

fn cmd_audit(path: &std::path::Path) -> Result<()> {
    if path.is_dir() && path.join("CAPABILITY.toml").exists() {
        // Single repo audit
        let result = audit::audit_repo(path)?;
        audit::print_audit(&result);

        if result.score < 50 {
            std::process::exit(1);
        }
    } else {
        // Audit all repos in directory
        let results = audit::audit_all(path)?;
        if results.is_empty() {
            println!("{}", "No repos found to audit.".yellow());
        } else {
            for result in &results {
                audit::print_audit(result);
            }

            // Summary
            println!("\n{}", "Summary".bold());
            println!("{}", "═".repeat(40).dimmed());
            let avg: f64 = results.iter().map(|r| r.score as f64).sum::<f64>() / results.len() as f64;
            println!(
                "  {} repos audited, average score: {:.0}/100",
                results.len().to_string().bold(),
                if avg >= 80.0 { avg.to_string().green() } else if avg >= 50.0 { avg.to_string().yellow() } else { avg.to_string().red() }
            );

            let low: Vec<_> = results.iter().filter(|r| r.score < 50).collect();
            if !low.is_empty() {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn cmd_suggest(path: &std::path::Path) -> Result<()> {
    let suggestions = suggest::suggest_path(path)?;
    suggest::print_suggestions(&suggestions);
    Ok(())
}

fn cmd_generate(what: GenerateCommands) -> Result<()> {
    match what {
        GenerateCommands::Capability {
            name,
            output,
            interactive,
        } => {
            generate::write_capability_template(&name, output.as_deref(), interactive)?;
        }
    }
    Ok(())
}

fn cmd_check(path: &std::path::Path) -> Result<()> {
    let checks = check::check_fleet(path)?;
    check::print_conservation(&checks);

    let has_violations = checks.iter().any(|c| !c.passed);
    if has_violations {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_version() -> Result<()> {
    println!(
        "{} {} {}",
        "si-cli".cyan().bold(),
        "v".dimmed(),
        VERSION.green()
    );
    println!(
        "  {} {}",
        "Part of the SuperInstance ecosystem".dimmed(),
        "⟁".green()
    );
    Ok(())
}
