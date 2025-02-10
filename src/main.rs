use clap::Parser;
use minim::{Args, Player};

use anyhow::Result;

fn main() -> Result<()> {
    let args = Args::parse();
    let mut player = Player::new(args)?;

    player.run()?;
    Ok(())
}
