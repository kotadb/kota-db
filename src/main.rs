// KotaDB CLI - Command-line interface for the database

use anyhow::Result;
use clap::{Parser, Subcommand};
use kotadb::{init_logging, with_trace_id};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build or rebuild database indices
    Index {
        /// Path to the directory to index
        #[arg(default_value = ".")]
        path: String,

        /// Force rebuild even if indices exist
        #[arg(short, long)]
        force: bool,
    },

    /// Search the database
    Search {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show database statistics
    Stats,

    /// Verify database integrity
    Verify {
        /// Check indices
        #[arg(long)]
        check_indices: bool,

        /// Check storage
        #[arg(long)]
        check_storage: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first
    init_logging()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Index { path, force } => {
            with_trace_id("cli.index", async { index_command(path, force).await }).await?;
        }
        Commands::Search { query, limit } => {
            with_trace_id("cli.search", async { search_command(query, limit).await }).await?;
        }
        Commands::Stats => {
            with_trace_id("cli.stats", async { stats_command().await }).await?;
        }
        Commands::Verify {
            check_indices,
            check_storage,
        } => {
            with_trace_id("cli.verify", async {
                verify_command(check_indices, check_storage).await
            })
            .await?;
        }
    }

    Ok(())
}

async fn index_command(path: String, force: bool) -> Result<()> {
    println!("Indexing directory: {} (force={})", path, force);
    // TODO: Implement after storage engine is ready
    Ok(())
}

async fn search_command(query: String, limit: usize) -> Result<()> {
    println!("Searching for: '{}' (limit={})", query, limit);
    // TODO: Implement after query engine is ready
    Ok(())
}

async fn stats_command() -> Result<()> {
    println!("Database Statistics:");
    println!("-------------------");
    // TODO: Implement after storage engine is ready
    let metrics = kotadb::observability::get_metrics();
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    Ok(())
}

async fn verify_command(check_indices: bool, check_storage: bool) -> Result<()> {
    println!("Verifying database integrity...");
    if check_indices {
        println!("Checking indices...");
        // TODO: Implement
    }
    if check_storage {
        println!("Checking storage...");
        // TODO: Implement
    }
    Ok(())
}
