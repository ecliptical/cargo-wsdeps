use cargo_toml::Manifest;
use cargo_wsdeps::{diff::generate_diff, partition_dependencies, show::print_changes};
use clap::{Parser, Subcommand};

#[cfg(all(feature = "jemalloc", target_env = "musl"))]
use jemallocator::Jemalloc;

#[cfg(all(feature = "jemalloc", target_env = "musl"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
#[command(styles = clap_cargo::style::CLAP_STYLING)]
enum CargoCli {
    Wsdeps(WsDepsArgs),
}

#[derive(clap::Args)]
#[command(version, about, long_about = None)]
struct WsDepsArgs {
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
    let CargoCli::Wsdeps(args) = CargoCli::parse();
    let mut metadata = args.manifest.metadata();
    args.features.forward_metadata(&mut metadata);
    let metadata = metadata.exec()?;
    let manifest = Manifest::from_path(metadata.workspace_root.join("Cargo.toml"))?;
    let Some(ref workspace) = manifest.workspace else {
        return Ok(());
    };

    let (selected, _) = args.workspace.partition_packages(&metadata);

    let (add, remove) = partition_dependencies(workspace, &selected)?;

    match args.cmd {
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
