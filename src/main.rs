use cargo_toml::Manifest;
use cargo_wsdeps::{diff::generate_diff, partition_dependencies, show::print_changes};
use clap::{Parser, Subcommand};

#[cfg(all(feature = "jemalloc", target_env = "musl"))]
use jemallocator::Jemalloc;

#[cfg(all(feature = "jemalloc", target_env = "musl"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

/// Manage Cargo workspace dependencies.
#[derive(Debug, Parser)]
#[command(bin_name = "cargo")]
struct Cli {
    #[command(flatten)]
    manifest: clap_cargo::Manifest,
    #[command(flatten)]
    workspace: clap_cargo::Workspace,
    #[command(flatten)]
    features: clap_cargo::Features,
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Print changes to shared dependencies in the workspace.
    Show,
    /// Generate a diff for the workspace.
    Diff {
        /// Use dotted notation for simple dependencies.
        #[arg(long, default_value = "false")]
        dotted: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut metadata = cli.manifest.metadata();
    cli.features.forward_metadata(&mut metadata);
    let metadata = metadata.exec()?;
    let manifest = Manifest::from_path(metadata.workspace_root.join("Cargo.toml"))?;
    let Some(ref workspace) = manifest.workspace else {
        return Ok(());
    };

    let (selected, _) = cli.workspace.partition_packages(&metadata);

    let (add, remove) = partition_dependencies(workspace, &selected)?;

    match cli.cmd {
        Commands::Show => {
            print_changes(&add, &remove);
        }
        Commands::Diff { dotted } => {
            if !add.is_empty() || !remove.is_empty() {
                generate_diff(&add, &remove, &metadata, dotted)?;
            }
        }
    }

    Ok(())
}
