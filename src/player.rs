use crate::views::{PlayerView, SharedState};

use std::path::PathBuf;
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
    pub fn new(library_root: PathBuf) -> Result<Self> {
        let (stream, handle) =
            rodio::OutputStream::try_default().context("Error opening rodio output stream")?;
        let sink = rodio::Sink::try_new(&handle).context("Error creating new sink")?;
        let shared_sink = Arc::new(sink);
        let mut siv = cursive::default();
        let shared_state = SharedState::new(shared_sink.clone());

        let mut player_view = PlayerView::new(shared_state.clone());
        player_view.add_library_root(library_root);

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

    pub fn start(&mut self) -> Result<()> {
        self.ui.siv.run();
        Ok(())
    }
}
