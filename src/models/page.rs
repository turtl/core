//! A page is a view of a particular slice of notes.
//!
//! Notes don't live IN pages, but instead pages can reference a collection of notes either
//! automatically (by some filter) or manually by a user curating a specific set of notes that a
//! page references.

use crate::models::{
    object_id,
    note::{NoteID, Tag},
    space::SpaceID,
};
use getset::Getters;
use rasn::{AsnType, Encode, Decode};
use serde::{Deserialize, Serialize};

object_id! {
    /// A unique ID for a page
    PageID
}

/// Defines the actions we can perform on a note
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum PageCrdt {
    /// Create a page
    #[rasn(tag(explicit(0)))]
    Set(Page),
    /// Set a page's display
    #[rasn(tag(explicit(1)))]
    SetDisplay(Display),
    /// Set a page's slice
    #[rasn(tag(explicit(2)))]
    SetSlice(Slice),
    /// Set a page's title
    #[rasn(tag(explicit(3)))]
    SetTitle(String),
    /// Delete a page
    #[rasn(tag(explicit(4)))]
    Unset,
}

/// Describes a slice of notes given a filter criteria
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum SliceFilter {
    /// An intersection of filters
    #[rasn(tag(explicit(0)))]
    And(Vec<SliceFilter>),
    /// A union of filters
    #[rasn(tag(explicit(1)))]
    Or(Vec<SliceFilter>),
    /// Filter by a specific tag
    #[rasn(tag(explicit(2)))]
    Tag(Tag),
    /// Filter by a text search
    #[rasn(tag(explicit(3)))]
    Search(String),
    /// Filter notes with attachments
    #[rasn(tag(explicit(4)))]
    HasFile(bool),
    /// Filter notes that link to a specific note
    #[rasn(tag(explicit(5)))]
    LinksTo(NoteID),
}

/// Defines sort order ascending or descending
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum AscDesc {
    #[rasn(tag(explicit(0)))]
    Ascending,
    #[rasn(tag(explicit(1)))]
    Descending,
}

/// Allows sorting a set of notes.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum Sort {
    #[rasn(tag(explicit(0)))]
    Created,
    #[rasn(tag(explicit(1)))]
    Modified,
    #[rasn(tag(explicit(2)))]
    Title,
    #[rasn(tag(explicit(3)))]
    HasFile,
}

/// Specifies a sort order
#[derive(AsnType, Encode, Decode, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct SortEntry {
    #[rasn(tag(explicit(0)))]
    sort: Sort,
    #[rasn(tag(explicit(1)))]
    asc: AscDesc,
}

/// A page slice is a sorted view of the notes in a space. It can be a manually created list,
/// or an automatically filtered list based on text, tag, etc.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum Slice {
    /// An automated view of notes in a space by some filtering and sorting criteria.
    #[rasn(tag(explicit(0)))]
    Filtered {
        #[rasn(tag(explicit(0)))]
        filter: SliceFilter,
        #[rasn(tag(explicit(1)))]
        sort: Vec<SortEntry>,
    },
    /// A manually-created list of notes with a manually-set sort order.
    #[rasn(tag(explicit(1)))]
    Manual(Vec<NoteID>),
}

/// A view determines how notes will be displayed within a page: a list, a grid, a masonry layout,
/// etc.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum Display {
    #[rasn(tag(explicit(0)))]
    ListSingleCol,
    #[rasn(tag(explicit(1)))]
    ListDoubleCol,
    #[rasn(tag(explicit(2)))]
    Grid,
    #[rasn(tag(explicit(3)))]
    Masonry,
    #[rasn(tag(explicit(4)))]
    Graph,
}

/// A space is a siloed container of notes and pages. It offers a way to keep these sets of data
/// completely separated from each other.
///
/// For instance, you might have a space for home, for work, for family, etc.
///
/// Spaces are also the mechanism for sharing data with other Turtl users.
#[derive(AsnType, Encode, Decode, Serialize, Deserialize, Getters)]
#[getset(get = "pub")]
pub struct Page {
    /// The pages's unique ID
    #[rasn(tag(explicit(0)))]
    id: PageID,
    /// The space this page lives in
    #[rasn(tag(explicit(1)))]
    space_id: SpaceID,
    /// The page's title. Gotta have a title.
    #[rasn(tag(explicit(2)))]
    title: String,
    /// The slice of the notes in a given space, filtered by some value: a text search, a
    /// union/intersection of tags, etc.
    #[rasn(tag(explicit(3)))]
    slice: Slice,
    /// Determines how notes in this page are displayed.
    #[rasn(tag(explicit(4)))]
    view: Display,
}

