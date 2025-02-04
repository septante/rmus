use crate::files::{Field, Track};

use std::{fs, io::BufReader, sync::Arc};

use cursive::{
    view::{Nameable, Resizable, ViewWrapper},
    views::{LinearLayout, NamedView, Panel},
};
use cursive_table_view::{TableView, TableViewItem};
use rodio::Sink;

pub type NamedPanel<T> = Panel<NamedView<T>>;
pub type TrackTable = TableView<Track, Field>;
type NowPlayingTable = TableView<NowPlayingEntry, NowPlayingField>;

struct LibraryTracksView {
    table: NamedPanel<TrackTable>,
}

impl LibraryTracksView {
    pub(crate) fn new(sink: Arc<Sink>) -> Self {
        let mut table = TrackTable::new()
            .column(Field::Artist, "Artist", |c| c)
            .column(Field::Title, "Title", |c| c)
            .column(Field::Duration, "Length", |c| c.width(10));

        table.set_on_submit(move |siv, _row, index| {
            let mut title = String::new();
            let mut valid_file = false;

            // Play song
            siv.call_on_name("tracks", |v: &mut TrackTable| {
                let track = v
                    .borrow_item(index)
                    .expect("Index given by submit event should always be valid");

                title = track.field_string(Field::Title);

                // TODO: handle case where file is removed while player is running, e.g., by prompting user to remove
                // from library view. This could be useful if we ever switch to persisting the library in a database
                let file = fs::File::open(track.path.clone())
                    .expect("Path should be valid, since we imported these files at startup");

                // Add song to queue. TODO: display error message when attempting to open an unsupported file
                if let Ok(decoder) = rodio::Decoder::new(BufReader::new(file)) {
                    sink.append(decoder);
                    valid_file = true;
                }
            })
            .expect("Couldn't find tracks view?");

            if valid_file {
                // Add to now playing list
                siv.call_on_name("now_playing", |v: &mut NowPlayingTable| {
                    v.insert_item(NowPlayingEntry {
                        index: v.len() + 1,
                        title,
                    })
                })
                .expect("Couldn't find now_playing view");
            }
        });

        let panel = Panel::new(table.with_name("tracks"));

        Self { table: panel }
    }

    cursive::inner_getters!(self.table: NamedPanel<TrackTable>);
}

impl ViewWrapper for LibraryTracksView {
    cursive::wrap_impl!(self.table: NamedPanel<TrackTable>);
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct NowPlayingEntry {
    index: usize,
    title: String,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum NowPlayingField {
    Index,
    Title,
}

impl TableViewItem<NowPlayingField> for NowPlayingEntry {
    fn to_column(&self, column: NowPlayingField) -> String {
        match column {
            NowPlayingField::Index => format!("{}", self.index),
            NowPlayingField::Title => self.title.clone(),
        }
    }

    fn cmp(&self, other: &Self, column: NowPlayingField) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            NowPlayingField::Index => self.index.cmp(&other.index),
            NowPlayingField::Title => self.title.cmp(&other.title),
        }
    }
}

struct LibrarySidebarView {
    now_playing_view: NamedPanel<NowPlayingTable>,
}

impl LibrarySidebarView {
    fn new() -> Self {
        let table = TableView::new()
            .column(NowPlayingField::Index, "#", |c| c.width(5))
            .column(NowPlayingField::Title, "Track", |c| c);

        let panel = Panel::new(table.with_name("now_playing"));

        Self {
            now_playing_view: panel,
        }
    }

    cursive::inner_getters!(self.now_playing_view: NamedPanel<NowPlayingTable>);
}

impl ViewWrapper for LibrarySidebarView {
    cursive::wrap_impl!(self.now_playing_view: NamedPanel<NowPlayingTable>);
}

pub(crate) struct LibraryView {
    view: LinearLayout,
}

impl LibraryView {
    pub(crate) fn new(sink: Arc<Sink>) -> Self {
        let linear_layout = LinearLayout::horizontal()
            .child(LibraryTracksView::new(sink).full_screen())
            .child(LibrarySidebarView::new().min_width(40));

        Self {
            view: linear_layout,
        }
    }

    cursive::inner_getters!(self.view: LinearLayout);
}

impl ViewWrapper for LibraryView {
    cursive::wrap_impl!(self.view: LinearLayout);
}
