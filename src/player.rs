use std::borrow::Cow;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, fs};

use anyhow::{anyhow, Context, Result};
use cursive::CursiveRunnable;
use cursive::{traits::*, views::Dialog};
use cursive_table_view::{TableView, TableViewItem};
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

#[non_exhaustive]
#[derive(Clone)]
struct Metadata {
    tag: Tag,
    properties: FileProperties,
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Metadata")
            .field("title", &self.tag.title())
            .field("artist", &self.tag.artist())
            .finish_non_exhaustive()
    }
}

impl Metadata {
    fn tag_to_string(tag: Option<Cow<str>>) -> Option<String> {
        tag.as_deref().map(|x| x.to_owned())
    }

    fn title(&self) -> Option<String> {
        Self::tag_to_string(self.tag.title())
    }

    fn artist(&self) -> Option<String> {
        Self::tag_to_string(self.tag.artist())
    }

    fn duration(&self) -> Duration {
        self.properties.duration()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Track {
    path: PathBuf,
    metadata: Metadata,
}

impl Track {
    fn field_string(&self, field: Field) -> String {
        match field {
            Field::Title => {
                if let Some(title) = self.metadata.title() {
                    title
                } else {
                    self.path
                        .file_name()
                        .expect("Path should be valid, since we imported these files at startup")
                        .to_string_lossy()
                        .into_owned()
                }
            }
            Field::Artist => self.metadata.artist().unwrap_or_default(),
            Field::Duration => {
                let secs = self.metadata.duration().as_secs();
                let mins = secs / 60;
                let secs = secs - mins * 60;
                format!("{mins}:{:0>2}", secs)
            }
            #[allow(unreachable_patterns)]
            _ => "".to_owned(),
        }
    }
}

impl TryFrom<PathBuf> for Track {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let tagged_file = Probe::open(&path)?.read()?;

        // Try to get primary tag, then try to find the first tag, otherwise
        // generate an empty tag if none exist
        let tag = if let Some(primary_tag) = tagged_file.primary_tag() {
            primary_tag.to_owned()
        } else if let Some(tag) = tagged_file.first_tag() {
            tag.to_owned()
        } else {
            Tag::new(tagged_file.file_type().primary_tag_type())
        };

        let properties = tagged_file.properties().to_owned();

        Ok(Track {
            path,
            metadata: Metadata { tag, properties },
        })
    }
}

impl TableViewItem<Field> for Track {
    fn to_column(&self, column: Field) -> String {
        self.field_string(column)
    }

    fn cmp(&self, other: &Self, column: Field) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            Field::Title | Field::Artist => {
                // TODO: Clean this up? Sort None values to the bottom
                self.field_string(column).cmp(&other.field_string(column))
            }
            Field::Duration => self.metadata.duration().cmp(&other.metadata.duration()),
        }
    }
}

struct Interface {
    siv: CursiveRunnable,
}

pub struct Player {
    // We need to hold the stream to prevent it from being dropped, even if we don't access it otherwise
    // See https://github.com/RustAudio/rodio/issues/525
    _stream: OutputStream,
    ui: Interface,
}

impl Player {
    pub fn new() -> Result<Self> {
        let (stream, handle) =
            rodio::OutputStream::try_default().context("Error opening rodio output stream")?;
        let sink = rodio::Sink::try_new(&handle).context("Error creating new sink")?;
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
                    .expect("Index given by submit event should always be valid");
                // TODO: handle case where file is removed while player is running, e.g., by prompting user to remove
                // from library view. This could be useful if we ever switch to persisting the library in a database
                let file = fs::File::open(track.path.clone())
                    .expect("Path should be valid, since we imported these files at startup");

                // Add song to queue. TODO: display error message when attempting to open an unsupported file
                if let Ok(decoder) = rodio::Decoder::new(BufReader::new(file)) {
                    sink.append(decoder);
                }
            })
            .expect("Couldn't find tracks view?");
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

        let mut player = Player {
            _stream: stream,
            ui: Interface { siv },
        };

        player
            .load_user_theme()
            .or_else(|_| player.load_default_theme())?;

        Ok(player)
    }

    pub fn import_tracks(&mut self, tracks: Vec<Track>) -> Result<()> {
        self.ui
            .siv
            .call_on_name("tracks", |s: &mut TableView<Track, Field>| {
                s.set_items(tracks);
            })
            .ok_or(anyhow!("Couldn't find tracks view while importing files?"))?;
        Ok(())
    }

    fn load_user_theme(&mut self) -> anyhow::Result<()> {
        let mut path = dirs::config_dir().ok_or(anyhow!("Error getting config dir path"))?;
        path.push("minim");
        path.push("theme.toml");

        let err: anyhow::Result<()> = match self.ui.siv.load_theme_file(path) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                cursive::theme::Error::Io(e) => Err(e.into()),
                cursive::theme::Error::Parse(e) => Err(e.into()),
            },
        };

        err.context("Couldn't find user theme, falling back to default")
    }

    fn load_default_theme(&mut self) -> anyhow::Result<()> {
        let err: anyhow::Result<()> = match self.ui.siv.load_toml(include_str!("../theme.toml")) {
            Ok(_) => Ok(()),
            Err(e) => match e {
                cursive::theme::Error::Io(e) => Err(e.into()),
                cursive::theme::Error::Parse(e) => Err(e.into()),
            },
        };

        err.context("Failed to load default theme")
    }

    pub fn start(&mut self) {
        self.ui.siv.run();
    }
}
