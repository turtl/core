//! This module houses our file system, which allows [notes][crate::models::note] to have file
//! attachment data.
//!
//! Files are set up into two parts: a file object that describes the file metedata, and a
//! collection of chunks of the file that when put in order and decrypted will allow the full file
//! to be reconstructed.

use crate::models::{
    object_id,
    space::SpaceID,
};
use getset::Getters;
use rasn::{AsnType, Decode, Encode};
use serde::{Deserialize, Serialize};
use stamp_core::crypto::base::Hash;

object_id! {
    /// A unique id for files
    FileID
}

object_id! {
    /// An ID for file chunks
    FileChunkID
}

/// Defines the actions we can perform on a file
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum FileCrdt {
    /// Add a file
    #[rasn(tag(explicit(0)))]
    Set(File),
    /// Create a chunk
    #[rasn(tag(explicit(1)))]
    SetChunk(FileChunk),
    /// Set a file's name
    #[rasn(tag(explicit(2)))]
    SetName(String),
    /// Remove a file
    #[rasn(tag(explicit(3)))]
    Unset,
}

/// A single chunk of a file
#[derive(AsnType, Encode, Decode, Serialize, Deserialize, Getters)]
#[getset(get = "pub")]
pub struct FileChunk {
    /// The chunk's ID
    #[rasn(tag(explicit(0)))]
    id: FileChunkID,
    /// The file this chunk belongs to.
    #[rasn(tag(explicit(1)))]
    file_id: FileID,
    /// The hash of this chunk's pre-encrypted content
    #[rasn(tag(explicit(2)))]
    hash: Hash,
    /// The zero-based index of this chunk within the file.
    #[rasn(tag(explicit(3)))]
    index: u32,
}

/// A file that can be linked to or embeded into a note.
#[derive(AsnType, Encode, Decode, Serialize, Deserialize, Getters)]
#[getset(get = "pub")]
pub struct File {
    /// The file's ID
    #[rasn(tag(explicit(0)))]
    id: FileID,
    /// The space this file lives in
    #[rasn(tag(explicit(1)))]
    space_id: SpaceID,
    /// The filename
    #[rasn(tag(explicit(2)))]
    name: String,
    /// The optional mime type
    #[rasn(tag(explicit(3)))]
    ty: Option<String>,
    /// The number of chunks this file has
    #[rasn(tag(explicit(4)))]
    num_chunks: u32,
}

