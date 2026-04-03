use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod discovery;
mod rcv;
mod snd;

#[derive(Parser)]
#[command(name = "lancp", about = "Copy files over LAN")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send files to a host on the LAN
    Snd {
        /// Files or directories to send
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Announcement port
        #[arg(long, default_value_t = 5300)]
        port: u16,
        /// Data transfer port
        #[arg(long, default_value_t = 5301)]
        data_port: u16,
    },
    /// Receive files from a host on the LAN
    Rcv {
        /// Announcement port
        #[arg(long, default_value_t = 5300)]
        port: u16,
        /// Data transfer port
        #[arg(long, default_value_t = 5301)]
        data_port: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Snd { paths, port, data_port } => snd::run(paths, port, data_port).await,
        Commands::Rcv { port, data_port } => rcv::run(port, data_port).await,
    }
}
