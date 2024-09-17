//! Holds all the data models for Turtl.
//!
//! This is things like notes, files, spaces, etc. This module also houses utilities for
//! constructing models and implementing traits useful to them.

use crate::error::Result;
use rasn::{AsnType, Encode, Decode, Tag};
use serde::{Deserialize, Serialize};
use stamp_core::crypto::base::SecretKey;
use uuid::Uuid;

pub mod file;
pub mod note;
pub mod operation;
pub mod page;
pub mod space;
pub mod state;
pub mod user;

/// Allows an object to be converted into its encrypted system type.
///
/// Ie, `Note` becomes `NoteEncrypted`
pub trait Encryptable: Sized {
    /// Defines the type that we are encrypting into.
    type Output;

    /// Encrypt the current object.
    fn encrypt(self, secret_key: &SecretKey) -> Result<Self::Output>;

    /// Decrypt the encrypted value and return the origin.
    fn decrypt(secret_key: &SecretKey, encrypted: &Self::Output) -> Result<Self>;
}

/// A globally-unique identifier that can be lexographically sorted once serialized.
///
/// This is a thin wrapper around [Uuid].
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ObjectID(Uuid);

impl AsnType for ObjectID {
    const TAG: Tag = Tag::UTF8_STRING;
}

impl Encode for ObjectID {
    fn encode_with_tag_and_constraints<E: rasn::Encoder>(&self, encoder: &mut E, tag: rasn::Tag, constraints: rasn::types::constraints::Constraints) -> std::result::Result<(), E::Error> {
        // Accepts a closure that encodes the contents of the sequence.
        encoder.encode_octet_string(tag, constraints, &self.0.as_bytes()[..])?;
        Ok(())
    }
}

impl Decode for ObjectID {
    fn decode_with_tag_and_constraints<D: rasn::Decoder>(decoder: &mut D, tag: rasn::Tag, constraints: rasn::types::constraints::Constraints) -> std::result::Result<Self, D::Error> {
        let vec = decoder.decode_octet_string(tag, constraints)?;
        let arr: [u8; 16] = vec.try_into()
            .map_err(|_| rasn::de::Error::no_valid_choice("octet string is incorrect length", rasn::Codec::Der))?;
        Ok(Self(Uuid::from_bytes(arr)))
    }
}

macro_rules! object_id {
    (
        $(#[$meta:meta])*
        $name:ident
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize, rasn::AsnType, rasn::Encode, rasn::Decode)]
        #[rasn(delegate)]
        pub struct $name(crate::models::ObjectID);
    }
}
pub(crate) use object_id;

