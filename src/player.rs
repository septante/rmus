use crate::files::Track;
use crate::views::{PlayerView, SharedState, TrackTable, TRACKS_TABLE_VIEW_SELECTOR};

use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cursive::traits::*;
use cursive::CursiveRunnable;
use rodio::OutputStream;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Reset library cache
    #[arg(short = 'c', long = "clean")]
    disable_cache: bool,
}

struct Interface {
    siv: CursiveRunnable,
}

pub struct Player {
    // We need to hold the stream to prevent it from being dropped, even if we don't access it otherwise
    // See https://github.com/RustAudio/rodio/issues/525
    _stream: OutputStream,
    args: Args,
    library_root: PathBuf,
    ui: Interface,
}

impl Player {
    pub fn new(args: Args) -> Result<Self> {
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
            args,
            library_root: PathBuf::from_str(".").unwrap(),
            ui: Interface { siv },
        };

        player
            .load_user_theme()
            .or_else(|_| player.load_default_theme())?;

        Ok(player)
    }

    pub fn set_library_root(&mut self, dir: &PathBuf) {
        self.library_root = dir.to_owned();
    }

    fn get_tracks_from_disk(&self) -> Vec<Track> {
        let files = WalkDir::new(&self.library_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|f| f.file_type().is_file());

        files.flat_map(|f| Track::try_from(f.path())).collect()
    }

    fn import_metadata(&mut self) -> Result<()> {
        let mut path = dirs::cache_dir().expect("Missing cache dir?");
        path.push("minim");
        path.push("library.csv");

        let tracks;
        if !self.args.disable_cache {
            if let Ok(t) = crate::cache::read_cache(&path) {
                tracks = t;
            } else {
                tracks = self.get_tracks_from_disk();
            }
        } else {
            tracks = self.get_tracks_from_disk();
        }

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

        self.import_metadata()?;

        self.ui.siv.run();

        // Cleanup
        let state = self.ui.siv.user_data::<SharedState>().unwrap();
        let tracks = state.tracks.lock().unwrap().clone();

        crate::cache::write_cache(&path, tracks)?;

        Ok(())
    }
}
