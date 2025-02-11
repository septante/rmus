#![forbid(unsafe_code)]

use anyhow::Result;
use clap::Parser;

use minim::{Args, Player};

fn main() -> Result<()> {
    let args = Args::parse();
    let mut player = Player::new(args)?;

    player.run()
}
