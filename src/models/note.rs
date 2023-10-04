//! The note module houses the note model. Notes are a collection of [`Section`] structs,
//! which altogether create the body of the note.

use crate::{
    models::{
        object_id,
        file::FileID,
        page::PageID,
        space::SpaceID,
    },
};
use getset::Getters;
use rasn::{AsnType, Encode, Decode};
use serde::{Deserialize, Serialize};
use stamp_core::{
    util::{
        HashMapAsn1,
        Url,
    },
};

object_id! {
    /// A unique id for our note
    NoteID
}

object_id! {
    /// Defines a unique ID for a body section.
    ///
    /// Sections are given their ID regardless of their order within the note body, so edits to a
    /// section can happen independently of the *position* of that section within the body. This
    /// makes CRDT merges and updates more correct as opposed to dealing with weird shit like
    /// array indexes, which can move around.
    SectionID
}

/// Defines the actions we can perform on a note.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum NoteCrdt {
    /// Create a full note
    #[rasn(tag(explicit(0)))]
    Set(Note),
    /// Add a new section to this note
    #[rasn(tag(explicit(1)))]
    SetBodySection {
        #[rasn(tag(explicit(0)))]
        section_id: SectionID,
        #[rasn(tag(explicit(1)))]
        section: Section,
        #[rasn(tag(explicit(2)))]
        after: Option<SectionID>,
    },
    /// Add a tag to this note
    #[rasn(tag(explicit(2)))]
    SetTag(Tag),
    /// Set this note's title LOL
    #[rasn(tag(explicit(3)))]
    SetTitle(Option<String>),
    /// Remove a note
    #[rasn(tag(explicit(4)))]
    Unset,
    /// Remove a section
    #[rasn(tag(explicit(5)))]
    UnsetBodySection(SectionID),
    /// Remove a tag
    #[rasn(tag(explicit(6)))]
    UnsetTag(Tag),
}

/// Represents a tag that can be attached to a note
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(delegate)]
pub struct Tag(String);

#[derive(PartialEq, Eq, Hash, Deserialize, Serialize, AsnType, Encode, Decode, Getters)]
#[getset(get = "pub")]
pub struct TableCoord {
    #[rasn(tag(explicit(0)))]
    row: u32,
    #[rasn(tag(explicit(1)))]
    col: u8,
}

/// A section is a paragraph, bullet list, etc...any piece or component of a note's body.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum Section {
    /// A link to a note
    #[rasn(tag(explicit(0)))]
    NoteLink(NoteID),
    /// A link to a page
    #[rasn(tag(explicit(1)))]
    PageLink(PageID),
    /// Header 1
    #[rasn(tag(explicit(2)))]
    Heading1(String),
    /// Header 2
    #[rasn(tag(explicit(3)))]
    Heading2(String),
    /// Header 3
    #[rasn(tag(explicit(4)))]
    Heading3(String),
    /// Free-form text
    #[rasn(tag(explicit(5)))]
    Paragraph(String),
    /// A bullet item
    #[rasn(tag(explicit(6)))]
    Bullet(Vec<NoteBody>),
    /// A numbered list item
    #[rasn(tag(explicit(7)))]
    Numbered(Vec<NoteBody>),
    /// A checkbox item
    #[rasn(tag(explicit(8)))]
    Checkbox {
        #[rasn(tag(explicit(0)))]
        checked: bool,
        #[rasn(tag(explicit(1)))]
        text: String,
    },
    /// A Quote
    #[rasn(tag(explicit(9)))]
    Quote(String),
    /// Code block
    #[rasn(tag(explicit(10)))]
    Code(String),
    /// A bookmark
    #[rasn(tag(explicit(11)))]
    Bookmark(Url),
    /// Embed a photo/video/etc by URL (hotlinking...tsk tsk...)
    #[rasn(tag(explicit(12)))]
    Embed(Url),
    /// A secret value (obscured from view by default)
    #[rasn(tag(explicit(13)))]
    Secret(String),
    /// Ohhh look at me I'm a divider gee whizz guess I'll divide things don't mind me
    #[rasn(tag(explicit(14)))]
    Divider,
    /// A file...can be embedded (ie, photo, video, audio) or just a dumb download link
    #[rasn(tag(explicit(15)))]
    File {
        #[rasn(tag(explicit(0)))]
        id: FileID,
        #[rasn(tag(explicit(1)))]
        embed: bool,
    },
    /// A table
    #[rasn(tag(explicit(16)))]
    Table {
        #[rasn(tag(explicit(0)))]
        rows: u32,
        #[rasn(tag(explicit(1)))]
        cols: u8,
        #[rasn(tag(explicit(2)))]
        values: HashMapAsn1<TableCoord, String>,
    },
}

/// The body of a note, made from an ordered set of [`Section`]s
#[derive(AsnType, Encode, Decode, Getters, Deserialize, Serialize)]
#[getset(get = "pub")]
pub struct NoteBody {
    /// Our heroic body sections
    #[rasn(tag(explicit(0)))]
    sections: HashMapAsn1<SectionID, Section>,
    /// The sort order of our body sections, indexed by ID
    #[rasn(tag(explicit(1)))]
    order: Vec<SectionID>,
}


/// Represents a single note.
#[derive(AsnType, Encode, Decode, Serialize, Deserialize, Getters)]
#[getset(get = "pub")]
pub struct Note {
    /// Our ID
    #[rasn(tag(explicit(0)))]
    id: NoteID,
    /// The space this note is in
    #[rasn(tag(explicit(1)))]
    space_id: SpaceID,
    /// The note's optional title
    #[rasn(tag(explicit(2)))]
    title: Option<String>,
    /// The actual data within a note
    #[rasn(tag(explicit(3)))]
    body: NoteBody,
    /// The note's tags
    #[rasn(tag(explicit(4)))]
    tags: Vec<Tag>,
}

