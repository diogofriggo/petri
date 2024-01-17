mod engine;
mod error;
mod json;
mod model;

use std::path::PathBuf;

use error::Result;

use crate::engine::Engine;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    /// Last simulation clock    
    #[arg(long)]
    terminal_clock: usize,

    // Executing node ip:port address
    #[arg(long)]
    node: String,

    // List of all ip:port addresses that will take part in the simulation
    #[arg(long, num_args = 1..)]
    nodes: Vec<String>,

    /// Folder with .json Petri nets    
    #[arg(long)]
    nets_folder: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut engine = Engine::new(
        args.terminal_clock,
        args.node,
        &args.nodes,
        &args.nets_folder,
    )?;
    engine.run()
}
