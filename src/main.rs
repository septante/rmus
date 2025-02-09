use minim::{Player, Track};

use anyhow::{anyhow, Result};
use walkdir::WalkDir;

fn main() -> Result<()> {
    // TODO: allow user to configure library location
    let library_root = dirs::audio_dir().ok_or(anyhow!("Couldn't find music folder"))?;

    let mut player = Player::new(library_root)?;
    player.start()?;

    Ok(())
}
