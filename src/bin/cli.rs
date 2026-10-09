use clap::{Parser, Subcommand};
use eotexrip::{
    catalog::{Game, Overrides},
    pipeline, workspace,
};
use std::{path::PathBuf, sync::atomic::AtomicBool};

#[derive(Parser)]
#[command(
    version,
    about = "Extract, organize, and prepare Etrian Odyssey textures"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Create small synthetic resources to try the app without game data.
    Demo {
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Extract a decrypted game dump, RomFS, or resource directory.
    Extract {
        input: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, value_enum)]
        game: Game,
        #[arg(long)]
        title_id: Option<String>,
    },
    /// Analyze saved metadata without a ROM or pixels. Writes a proposed catalog.
    Replay {
        bundle: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        overrides: Option<PathBuf>,
    },
    /// Refresh emulator deployment from edited masters.
    Export { workspace: PathBuf },
    /// Apply confirmed names/categories and rebuild the pack while preserving pixels.
    Apply {
        workspace: PathBuf,
        overrides: PathBuf,
    },
    /// Show output counts and unresolved items from an existing workspace.
    Inspect { workspace: PathBuf },
}
fn main() -> anyhow::Result<()> {
    let cancel = AtomicBool::new(false);
    let catalog = match Args::parse().command {
        Command::Demo { output } => {
            eotexrip::demo::create(&output)?;
            println!("Synthetic demo created in {}", output.display());
            return Ok(());
        }
        Command::Extract {
            input,
            output,
            game,
            title_id,
        } => pipeline::extract(
            &pipeline::Options {
                input,
                output,
                game,
                title_id,
            },
            &cancel,
            |p| {
                eprintln!(
                    "{} resources / {} textures: {}",
                    p.resources, p.textures, p.message
                )
            },
        )?,
        Command::Replay {
            bundle,
            output,
            overrides,
        } => {
            let catalog = pipeline::replay(&bundle, overrides.as_deref())?;
            workspace::save_plan(&output, &catalog)?;
            catalog
        }
        Command::Export { workspace } => pipeline::export(&workspace, &cancel)?,
        Command::Apply {
            workspace,
            overrides,
        } => {
            let overrides: Overrides = serde_json::from_reader(std::fs::File::open(overrides)?)?;
            workspace::reassign(&workspace, overrides)?
        }
        Command::Inspect { workspace } => workspace::load(&workspace)?,
    };
    println!("{}", serde_json::to_string_pretty(&catalog.summary)?);
    Ok(())
}
