use minim::Player;

use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    // TODO: allow user to configure library location
    let library_root = dirs::audio_dir().ok_or(anyhow!("Couldn't find music folder"))?;

    let mut player = Player::new()?;
    player.set_library_root(&library_root);
    player.run()?;
    Ok(())
}
