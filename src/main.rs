use minim::{Player, Track};

use anyhow::{anyhow, Result};
use walkdir::WalkDir;

fn main() -> Result<()> {
    // TODO: allow user to configure library location
    let library_root = dirs::audio_dir().ok_or(anyhow!("Couldn't find music folder"))?;
    let files = WalkDir::new(&library_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|f| f.file_type().is_file());

    let tracks: Vec<_> = files.flat_map(|f| Track::try_from(f.path())).collect();

    let mut player = Player::new()?;
    player.import_tracks(tracks)?;
    player.start();
    Ok(())
}
