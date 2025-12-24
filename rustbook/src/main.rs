mod parser;
mod builder;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rustbook")]
#[command(about = "HonKit/GitBook compatible static book generator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new book
    Init {
        /// Directory to initialize
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Build the book
    Build {
        /// Source directory
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Output directory
        #[arg(short, long, default_value = "_book")]
        output: PathBuf,
    },
    /// Start a local server for preview
    Serve {
        /// Source directory
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Port to listen on
        #[arg(short, long, default_value = "4000")]
        port: u16,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            println!("Initializing book in {:?}", path);
            // TODO: Implement init
            Ok(())
        }
        Commands::Build { path, output } => {
            println!("Building book from {:?} to {:?}", path, output);
            builder::build(&path, &output)
        }
        Commands::Serve { path, port } => {
            println!("Serving book from {:?} on port {}", path, port);
            // TODO: Implement serve
            Ok(())
        }
    }
}
