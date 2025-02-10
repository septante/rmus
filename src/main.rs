use clap::Parser;
use minim::{Args, Player};

use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    let args = Args::parse();

    // TODO: allow user to configure library location
    let library_root = dirs::audio_dir().ok_or(anyhow!("Couldn't find music folder"))?;

    let mut player = Player::new(args)?;
    player.set_library_root(&library_root);
    player.run()?;
    Ok(())
}
