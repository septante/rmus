use crate::files::{CachedField, Track, WrappedSource};

use std::{
    fs,
    io::BufReader,
    sync::{Arc, Mutex},
};

use cursive::{
    align::HAlign,
    view::{Nameable, Resizable, Scrollable, Selector, ViewWrapper},
    views::{LinearLayout, NamedView, Panel, ScrollView, TextContent, TextView},
    View,
};
use cursive_table_view::{TableView, TableViewItem};
use cursive_tabs::TabPanel;
use lofty::tag::ItemKey;
use rodio::Sink;

pub(crate) const TRACKS_TABLE_VIEW_SELECTOR: Selector = Selector::Name("tracks");
pub(crate) const QUEUE_VIEW_SELECTOR: Selector = Selector::Name("queue_list");

pub(crate) type TrackTable = TableView<Track, CachedField>;

type ScrollNamedText = ScrollView<NamedView<TextView>>;
type NamedPanel<T> = Panel<NamedView<T>>;
type QueueTable = TableView<QueueEntry, QueueField>;

struct LibraryTracksView {
    inner: NamedPanel<TrackTable>,
}

impl LibraryTracksView {
    fn new(state: SharedState) -> Self {
        let mut table = TrackTable::new()
            .column(CachedField::Artist, "Artist", |c| c)
            .column(CachedField::Title, "Title", |c| c)
            .column(CachedField::Duration, "Length", |c| c.width(10));

        table.set_on_submit(move |siv, _row, index| {
            let mut title = String::new();
            let mut valid_file = false;
            let queue_index = state.queue_index.clone();
            let queue = state.queue.clone();

            // Play song
            siv.call_on(&TRACKS_TABLE_VIEW_SELECTOR, |v: &mut TrackTable| {
                let track = v
                    .borrow_item(index)
                    .expect("Index given by submit event should always be valid");

                title = track.cached_field_string(CachedField::Title);

                // TODO: handle case where file is removed while player is running, e.g., by prompting user to remove
                // from library view. This could be useful if we ever switch to persisting the library in a database
                let file = fs::File::open(track.path.clone())
                    .expect("Path should be valid, since we imported these files at startup");

                // Add song to queue. TODO: display error message when attempting to open an unsupported file
                if let Ok(decoder) = rodio::Decoder::new(BufReader::new(file)) {
                    let source = WrappedSource::new(decoder, move || {
                        *queue_index.lock().unwrap() += 1;
                    });
                    queue.lock().unwrap().push(track.clone());
                    state.sink.append(source);

                    valid_file = true;
                }
            })
            .expect("tracks view must exist");

            if valid_file {
                // Add to queue list
                siv.call_on(&QUEUE_VIEW_SELECTOR, |v: &mut QueueTable| {
                    let queue = queue.lock().unwrap();
                    v.insert_item(QueueEntry {
                        index: v.len() + 1,
                        track: queue.last().unwrap().clone(),
                    })
                })
                .expect("queue list view must exist");
            }
        });

        let panel = Panel::new(table.with_name("tracks"));

        Self { inner: panel }
    }

    cursive::inner_getters!(self.inner: NamedPanel<TrackTable>);
}

impl ViewWrapper for LibraryTracksView {
    cursive::wrap_impl!(self.inner: NamedPanel<TrackTable>);
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct QueueEntry {
    index: usize,
    track: Track,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum QueueField {
    Index,

    Track,
}

impl TableViewItem<QueueField> for QueueEntry {
    fn to_column(&self, column: QueueField) -> String {
        match column {
            QueueField::Index => format!("{}", self.index),
            QueueField::Track => self.track.cached_field_string(CachedField::Title),
        }
    }

    fn cmp(&self, other: &Self, column: QueueField) -> std::cmp::Ordering
    where
        Self: Sized,
    {
        match column {
            QueueField::Index => self.index.cmp(&other.index),
            QueueField::Track => self
                .track
                .cached_field_string(CachedField::Title)
                .cmp(&other.track.cached_field_string(CachedField::Title)),
        }
    }
}

struct LibrarySidebarView {
    inner: NamedPanel<QueueTable>,
}

impl LibrarySidebarView {
    fn new(state: SharedState) -> Self {
        let table = TableView::new()
            .column(QueueField::Index, "", |c| c.width(4).align(HAlign::Right))
            .column(QueueField::Track, "Track", |c| c);

        let panel = Panel::new(table.with_name("queue_list"));

        Self { inner: panel }
    }

    cursive::inner_getters!(self.inner: NamedPanel<QueueTable>);
}

impl ViewWrapper for LibrarySidebarView {
    cursive::wrap_impl!(self.inner: NamedPanel<QueueTable>);
}

struct LibraryView {
    inner: LinearLayout,
}

impl LibraryView {
    fn new(state: SharedState) -> Self {
        let linear_layout = LinearLayout::horizontal()
            .child(LibraryTracksView::new(state.clone()).full_screen())
            .child(LibrarySidebarView::new(state.clone()).min_width(40));

        Self {
            inner: linear_layout,
        }
    }

    cursive::inner_getters!(self.inner: LinearLayout);
}

impl ViewWrapper for LibraryView {
    cursive::wrap_impl!(self.inner: LinearLayout);
}

#[derive(Clone)]
pub(crate) struct SharedState {
    pub(crate) sink: Arc<Sink>,
    pub(crate) queue: Arc<Mutex<Vec<Track>>>,
    pub(crate) queue_index: Arc<Mutex<usize>>,
}

impl SharedState {
    pub(crate) fn new(sink: Arc<Sink>) -> Self {
        Self {
            sink,
            queue: Arc::new(Mutex::new(Vec::new())),
            queue_index: Arc::new(Mutex::new(0)),
        }
    }
}

struct LyricsView {
    state: SharedState,
    content: TextContent,
    inner: ScrollNamedText,
}

impl LyricsView {
    fn new(state: SharedState) -> Self {
        let content = TextContent::new("");
        let view = TextView::new_with_content(content.clone())
            .with_name("lyrics")
            .scrollable();
        Self {
            state,
            content,
            inner: view,
        }
    }

    cursive::inner_getters!(self.inner: ScrollNamedText);
}

impl ViewWrapper for LyricsView {
    cursive::wrap_impl!(self.inner: ScrollNamedText);

    fn wrap_draw(&self, printer: &cursive::Printer) {
        let queue = self.state.queue.lock().unwrap();
        let mut content = String::new();

        if let Some(track) = queue.get(*self.state.queue_index.lock().unwrap()) {
            content = track
                .tag_string_from_track(ItemKey::Lyrics)
                .unwrap_or_default();
        }

        self.content.set_content(content);
        self.with_view(|v| v.draw(printer));
    }
}

pub(crate) struct PlayerView {
    inner: TabPanel,
    state: SharedState,
}

impl PlayerView {
    pub(crate) fn new(state: SharedState) -> Self {
        let mut tab_view = TabPanel::new()
            .with_tab(LibraryView::new(state.clone()).with_name("Library"))
            .with_tab(LyricsView::new(state.clone()).with_name("Lyrics"));

        // We can't use .with_active_tab() when constructing because it uses Self as the Err type,
        // which doesn't implement Debug, meaning we can't call .expect() on it
        tab_view
            .set_active_tab("Library")
            .expect("Setting default tab shouldn't fail");

        Self {
            inner: tab_view,
            state,
        }
    }

    cursive::inner_getters!(self.inner: TabPanel);
}

impl ViewWrapper for PlayerView {
    cursive::wrap_impl!(self.inner: TabPanel);
}
