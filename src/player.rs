use crate::files::Track;
use crate::views::{PlayerView, SharedState, TrackTable, TRACKS_TABLE_VIEW_SELECTOR};

use std::fs;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use cursive::traits::*;
use cursive::CursiveRunnable;
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
        let shared_sink = Arc::new(sink);
        let mut siv = cursive::default();
        let shared_state = SharedState::new(shared_sink.clone());

        let player_view = PlayerView::new(shared_state.clone());

        siv.add_fullscreen_layer(player_view.with_name("player").full_screen());

        siv.add_global_callback('q', |s| s.quit());

        let sink = shared_sink.clone();
        siv.add_global_callback('p', move |_| {
            if sink.is_paused() {
                sink.play();
            } else {
                sink.pause();
            }
        });

        let sink = shared_sink.clone();
        let state = shared_state.clone();
        siv.add_global_callback('n', move |_| {
            sink.skip_one();
            *state.queue_index.lock().unwrap() += 1;
        });
        siv.set_user_data(shared_state.clone());

        siv.set_fps(10);

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
        let siv = &mut self.ui.siv;

        siv.call_on(&TRACKS_TABLE_VIEW_SELECTOR, |s: &mut TrackTable| {
            s.set_items(tracks.clone());
        })
        .ok_or(anyhow!("Couldn't find tracks view while importing files?"))?;

        let state = siv.user_data::<SharedState>().expect("Missing state?");
        *state.tracks.lock().unwrap() = tracks;
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

    pub fn run(&mut self) -> Result<()> {
        let mut path = dirs::cache_dir().expect("Missing cache dir?");
        path.push("minim");
        if !fs::exists(&path)? {
            fs::create_dir_all(&path)?;
        }
        path.push("library.csv");

        self.ui.siv.run();

        // Cleanup
        let state = self.ui.siv.user_data::<SharedState>().unwrap();
        let tracks = state.tracks.lock().unwrap().clone();

        crate::cache::write_cache(&path, tracks)?;

        Ok(())
    }
}
