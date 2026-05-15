use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "dualsense-mapper", version, about)]
struct Cli {
    #[arg(long)]
    validate: bool,
}

fn main() -> Result<()> {
    let _cli = Cli::parse();
    println!("dualsense-mapper {} — not yet implemented", env!("CARGO_PKG_VERSION"));
    Ok(())
}
