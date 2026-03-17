use std::path::PathBuf;

use clap::{Parser, Subcommand};
use memory_installer::converters::select_converter;
use memory_installer::parser::parse_sources;
use memory_installer::types::{InstallConfig, InstallScope, Runtime};
use memory_installer::writer::write_files;

#[derive(Parser, Debug)]
#[command(name = "memory-installer")]
#[command(about = "Install memory-agent plugins for various AI runtimes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install memory plugins for an AI runtime
    Install {
        /// Target runtime
        #[arg(long, value_enum)]
        agent: Runtime,

        /// Install to project directory (e.g., ./.claude/)
        #[arg(long, conflicts_with = "global")]
        project: bool,

        /// Install to global user directory (e.g., ~/.claude/)
        #[arg(long, conflicts_with = "project")]
        global: bool,

        /// Custom target directory (required with --agent skills)
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Preview what would be installed without writing files
        #[arg(long)]
        dry_run: bool,

        /// Path to canonical source root (defaults to auto-discovery)
        #[arg(long)]
        source: Option<PathBuf>,
    },
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Install {
            agent,
            project,
            global,
            dir,
            dry_run,
            source,
        } => {
            if !project && !global && dir.is_none() {
                eprintln!("error: one of --project, --global, or --dir is required");
                std::process::exit(1);
            }

            if agent == Runtime::Skills && dir.is_none() {
                eprintln!("error: --agent skills requires --dir <path>");
                std::process::exit(1);
            }

            // Determine source root
            let source_root = match source {
                Some(path) => path,
                None => match discover_source_root() {
                    Some(path) => path,
                    None => {
                        eprintln!(
                            "error: could not find plugins/ directory. Use --source to specify."
                        );
                        std::process::exit(1);
                    }
                },
            };

            // Parse canonical sources
            let bundle = match parse_sources(&source_root) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("error: failed to parse plugin sources: {e}");
                    std::process::exit(1);
                }
            };

            // Build install config
            let scope = if let Some(dir_path) = dir {
                InstallScope::Custom(dir_path)
            } else if project {
                let cwd = std::env::current_dir().unwrap_or_else(|e| {
                    eprintln!("error: cannot determine current directory: {e}");
                    std::process::exit(1);
                });
                InstallScope::Project(cwd)
            } else {
                InstallScope::Global
            };

            let cfg = InstallConfig {
                scope,
                dry_run,
                source_root,
            };

            // Select converter for the target runtime
            let converter = select_converter(agent);

            // Collect all converted files
            let mut all_files = Vec::new();

            for cmd in &bundle.commands {
                all_files.extend(converter.convert_command(cmd, &cfg));
            }
            for agent_def in &bundle.agents {
                all_files.extend(converter.convert_agent(agent_def, &cfg));
            }
            for skill in &bundle.skills {
                all_files.extend(converter.convert_skill(skill, &cfg));
            }
            for hook in &bundle.hooks {
                if let Some(file) = converter.convert_hook(hook, &cfg) {
                    all_files.push(file);
                }
            }
            all_files.extend(converter.generate_guidance(&bundle, &cfg));

            // Write or report
            match write_files(&all_files, dry_run) {
                Ok(report) => {
                    let mode = if dry_run { " (dry-run)" } else { "" };
                    println!("Install complete{mode} for {}: {report}", converter.name());
                }
                Err(e) => {
                    eprintln!("error: failed to write files: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}

/// Discover the `plugins/` source root by checking the current directory
/// and the binary's directory ancestors.
fn discover_source_root() -> Option<PathBuf> {
    // Check current directory
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("plugins");
        if candidate.join("installer-sources.json").exists() {
            return Some(candidate);
        }
    }

    // Check binary's directory ancestors
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            let candidate = d.join("plugins");
            if candidate.join("installer-sources.json").exists() {
                return Some(candidate);
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }

    None
}
