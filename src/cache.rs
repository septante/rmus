use std::{fs, path::PathBuf};

use anyhow::Result;

use crate::files::Track;

pub(crate) fn read_cache(path: &PathBuf) -> Result<Vec<Track>> {
    let file = fs::File::open(path)?;
    let mut reader = csv::Reader::from_reader(file);
    let tracks: Vec<Track> = reader.deserialize().flatten().collect();

    Ok(tracks)
}

pub(crate) fn write_cache(path: &PathBuf, tracks: Vec<Track>) -> Result<()> {
    let file = fs::File::create(path)?;
    let mut writer = csv::Writer::from_writer(file);
    for track in tracks {
        writer.serialize(track)?;
    }

    Ok(())
}
