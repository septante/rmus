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

        let library_view = LibraryView::new(sink_ptr.clone());

        siv.add_fullscreen_layer(
            Dialog::around(library_view.with_name("library").full_screen()).title("Library"),
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
