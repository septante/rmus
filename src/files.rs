use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use cursive_table_view::TableViewItem;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::properties::FileProperties;
use lofty::tag::Tag;

#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Field {
    Title,
    Artist,
    Duration,
}

#[non_exhaustive]
#[derive(Clone)]
pub(crate) struct Metadata {
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

    pub(crate) fn title(&self) -> Option<String> {
        Self::tag_to_string(self.tag.title())
    }

    pub(crate) fn artist(&self) -> Option<String> {
        Self::tag_to_string(self.tag.artist())
    }

    pub(crate) fn duration(&self) -> Duration {
        self.properties.duration()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Track {
    pub(crate) path: PathBuf,
    pub(crate) metadata: Metadata,
}

impl Track {
    pub(crate) fn field_string(&self, field: Field) -> String {
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
                self.field_string(column)
                    .to_lowercase()
                    .cmp(&other.field_string(column).to_lowercase())
            }
            Field::Duration => self.metadata.duration().cmp(&other.metadata.duration()),
        }
    }
}
