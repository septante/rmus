use std::borrow::Cow;
use std::{fs, path::PathBuf};

use cursive::{traits::*, views::Dialog};
use cursive_table_view::{TableView, TableViewItem};
use dirs;
use lofty::prelude::*;
use lofty::probe::Probe;

#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum Field {
    Title,
    Artist,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
struct Track {
    path: PathBuf,
    metadata: Metadata,
}

impl TryFrom<PathBuf> for Track {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let tagged_file = Probe::open(&path)?.read()?;
        let tag = match tagged_file.primary_tag() {
            Some(primary_tag) => primary_tag,
            // If the "primary" tag doesn't exist, we just grab the
            // first tag we can find. Realistically, a tag reader would likely
            // iterate through the tags to find a suitable one.
            None => tagged_file.first_tag().expect("ERROR: No tags found!"),
        };
        Ok(Track {
            path,
            metadata: Metadata {
                title: Metadata::tag_to_string(tag.title()),
                artist: Metadata::tag_to_string(tag.artist()),
            },
        })
    }
}

#[non_exhaustive]
#[derive(Clone, Debug)]
struct Metadata {
    title: Option<String>,
    artist: Option<String>,
}

impl Metadata {
    fn tag_to_string(tag: Option<Cow<str>>) -> Option<String> {
        tag.as_deref().map(|x| x.to_owned())
    }
}

impl TableViewItem<Field> for Track {
    fn to_column(&self, column: Field) -> String {
        match column {
            Field::Title => self
                .metadata
                .title
                .clone()
                .unwrap_or("Unknown Title".to_owned()),
            Field::Artist => self
                .metadata
                .artist
                .clone()
                .unwrap_or("Unknown Artist".to_owned()),
        }
    }

    fn cmp(&self, other: &Self, column: Field) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            Field::Title => self.metadata.title.cmp(&other.metadata.title),
            Field::Artist => self.metadata.artist.cmp(&other.metadata.artist),
        }
    }
}

fn main() {
    let library_root = dirs::audio_dir().expect("couldn't find music folder");
    let files = fs::read_dir(library_root)
        .expect("Error reading directory")
        .flatten()
        .filter(|x| x.file_type().expect("Error getting file type").is_file());

    let tracks = files.map(|f| Track::try_from(f.path())).flatten().collect();

    let mut siv = cursive::default();

    let mut table = TableView::<Track, Field>::new()
        .column(Field::Title, "Title", |c| c.width_percent(20))
        .column(Field::Artist, "Artist", |c| c.width_percent(20));

    table.set_items(tracks);

    table.set_on_submit(|siv, row, index| {
        // Play song
        todo!()
    });

    siv.add_fullscreen_layer(
        Dialog::around(table.full_screen().with_name("tracks")).title("Library"),
    );

    siv.add_global_callback('q', |s| s.quit());
    siv.run();
}
