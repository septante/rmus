use crate::files::{Field, Track};

use std::{fs, io::BufReader, sync::Arc};

use cursive::{
    view::{Nameable, Resizable, ViewWrapper},
    views::{LinearLayout, NamedView},
};
use cursive_table_view::TableView;
use rodio::Sink;

pub type TrackTable = TableView<Track, Field>;

struct LibraryTracksView {
    table: NamedView<TrackTable>,
}

impl LibraryTracksView {
    pub(crate) fn new(sink: Arc<Sink>) -> Self {
        let mut table = TrackTable::new()
            .column(Field::Title, "Title", |c| c.width_percent(20))
            .column(Field::Artist, "Artist", |c| c.width_percent(20))
            .column(Field::Duration, "Length", |c| c.width(10));

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

        Self {
            table: table.with_name("tracks"),
        }
    }

    cursive::inner_getters!(self.table: NamedView<TrackTable>);
}

impl ViewWrapper for LibraryTracksView {
    cursive::wrap_impl!(self.table: NamedView<TrackTable>);
}

struct LibrarySidebarView {}

pub(crate) struct LibraryView {
    view: LinearLayout,
}

impl LibraryView {
    pub(crate) fn new(sink: Arc<Sink>) -> Self {
        let linear_layout =
            LinearLayout::horizontal().child(LibraryTracksView::new(sink).full_screen());
        // .child(LibrarySidebarView {});

        Self {
            view: linear_layout,
        }
    }

    cursive::inner_getters!(self.view: LinearLayout);
}

impl ViewWrapper for LibraryView {
    cursive::wrap_impl!(self.view: LinearLayout);
}
