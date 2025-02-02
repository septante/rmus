use std::borrow::Cow;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fmt, fs};

use cursive::CursiveRunnable;
use cursive::{traits::*, views::Dialog};
use cursive_table_view::{TableView, TableViewItem};
use dirs;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::properties::FileProperties;
use lofty::tag::Tag;
use rodio::OutputStream;

#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum Field {
    Title,
    Artist,
    Duration,
}

impl Field {
    fn default_value(&self) -> String {
        match self {
            Field::Title => "Unknown Title".to_owned(),
            Field::Artist => "Unknown Artist".to_owned(),
            _ => "".to_owned(),
        }
    }
}

type FieldData = Option<String>;

#[non_exhaustive]
#[derive(Clone)]
struct Metadata {
    tag: Tag,
    properties: FileProperties,
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Metadata {
    fn field_string(&self, field: Field) -> String {
        match field {
            Field::Title => Self::unwrap_field(field, Self::tag_to_string(self.tag.title())),
            Field::Artist => Self::unwrap_field(field, Self::tag_to_string(self.tag.artist())),
            Field::Duration => {
                let secs = self.properties.duration().as_secs();
                let mins = secs / 60;
                let secs = secs - mins * 60;
                format!("{mins}:{:0>2}", secs)
            }
            #[allow(unreachable_patterns)]
            _ => "".to_owned(),
        }
    }

    fn unwrap_field(field: Field, data: FieldData) -> String {
        if let Some(s) = data {
            s.clone()
        } else {
            field.default_value()
        }
    }

    fn tag_to_string(tag: Option<Cow<str>>) -> Option<String> {
        tag.as_deref().map(|x| x.to_owned())
    }
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
        }
        .to_owned();
        let properties = tagged_file.properties().to_owned();

        Ok(Track {
            path,
            metadata: Metadata { tag, properties },
        })
    }
}

impl TableViewItem<Field> for Track {
    fn to_column(&self, column: Field) -> String {
        self.metadata.field_string(column)
    }

    fn cmp(&self, other: &Self, column: Field) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            Field::Title | Field::Artist => self
                .metadata
                .field_string(column)
                .cmp(&other.metadata.field_string(column)),
            Field::Duration => self
                .metadata
                .properties
                .duration()
                .cmp(&other.metadata.properties.duration()),
        }
    }
}

struct Player {
    // We need to hold the stream to prevent it from being dropped, even if we don't access it otherwise
    // See https://github.com/RustAudio/rodio/issues/525
    _stream: OutputStream,
    ui: Interface,
}

impl Player {
    fn new() -> Self {
        let (stream, handle) =
            rodio::OutputStream::try_default().expect("Error opening rodio output stream");
        let sink = rodio::Sink::try_new(&handle).expect("Error creating new sink");
        let sink_ptr = Arc::new(sink);
        let mut siv = cursive::default();

        let mut table = TableView::<Track, Field>::new()
            .column(Field::Title, "Title", |c| c.width_percent(20))
            .column(Field::Artist, "Artist", |c| c.width_percent(20))
            .column(Field::Duration, "Length", |c| c.width(10));

        let sink = sink_ptr.clone();
        table.set_on_submit(move |siv, _row, index| {
            // Play song
            siv.call_on_name("tracks", |v: &mut TableView<Track, Field>| {
                let track = v
                    .borrow_item(index)
                    .expect("Error getting track from table");
                let file =
                    fs::File::open(track.path.clone()).expect("Error opening file for playback");
                sink.append(
                    rodio::Decoder::new(BufReader::new(file)).expect("Error creating new decoder"),
                );
            })
            .expect("bad view");
        });

        siv.add_fullscreen_layer(
            Dialog::around(table.with_name("tracks").full_screen()).title("Library"),
        );

        siv.add_global_callback('q', |s| s.quit());
        let sink = sink_ptr.clone();
        siv.add_global_callback('p', move |_s| {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        });

        Player {
            _stream: stream,
            ui: Interface { siv },
        }
    }

    fn import_tracks(&mut self, tracks: Vec<Track>) {
        self.ui
            .siv
            .call_on_name("tracks", |s: &mut TableView<Track, Field>| {
                s.set_items(tracks);
            });
    }

    fn start(&mut self) {
        self.ui.siv.run();
    }
}

struct Interface {
    siv: CursiveRunnable,
}

fn main() {
    let library_root = dirs::audio_dir().expect("couldn't find music folder");
    let files = fs::read_dir(library_root)
        .expect("Error reading directory")
        .flatten()
        .filter(|x| x.file_type().expect("Error getting file type").is_file());

    let tracks: Vec<_> = files.map(|f| Track::try_from(f.path())).flatten().collect();

    let mut player = Player::new();
    player.import_tracks(tracks);
    player.start();
}
