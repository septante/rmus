use crate::files::{Field, Track};

use cursive::{view::ViewWrapper, View};
use cursive_table_view::{TableView, TableViewItem};
pub type TrackTable = TableView<Track, Field>;

struct LibrarySidebarView {}

struct LibraryTracksView {
    table: TrackTable,
}

impl LibraryTracksView {
    pub(crate) fn new(table: TrackTable) -> Self {
        Self { table }
    }

    cursive::inner_getters!(self.table: TrackTable);
}

pub(crate) struct LibraryView {
    tracks: LibraryTracksView,
    sidebar: LibrarySidebarView,
}

impl ViewWrapper for LibraryTracksView {
    cursive::wrap_impl!(self.table: TrackTable);
}
