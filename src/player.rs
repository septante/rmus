use crate::files::{Field, Track};
use crate::views::{LibraryView, TrackTable};

use std::fs;
use std::io::BufReader;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use cursive::CursiveRunnable;
use cursive::{traits::*, views::Dialog};
use cursive_table_view::TableView;
use rodio::OutputStream;

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

        let mut table = TrackTable::new()
            .column(Field::Title, "Title", |c| c.width_percent(20))
            .column(Field::Artist, "Artist", |c| c.width_percent(20))
            .column(Field::Duration, "Length", |c| c.width(10));

        let sink = sink_ptr.clone();
        table.set_on_submit(move |siv, _row, index| {
            // Play song
            siv.call_on_name("tracks", |v: &mut TrackTable| {
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
            .call_on_name("tracks", |s: &mut TrackTable| {
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
