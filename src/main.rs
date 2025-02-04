use minim::files::Track;
use minim::player::Player;

use std::fs;

use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    // TODO: allow user to configure library location
    let library_root = dirs::audio_dir().ok_or(anyhow!("Couldn't find music folder"))?;
    let files = fs::read_dir(library_root)?.flatten().filter(|f| {
        if let Ok(file_type) = f.file_type() {
            file_type.is_file()
        } else {
            false
        }
    });

    let tracks: Vec<_> = files.flat_map(|f| Track::try_from(f.path())).collect();

    let mut player = Player::new()?;
    player.import_tracks(tracks)?;
    player.start();
    Ok(())
}
