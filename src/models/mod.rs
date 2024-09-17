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

// DEBUG: rm probably
/*
/// Takes the repetition out of constructing encryptable models.
///
/// The format is as follows:
///
/// ```ignore
/// encryptable! {
///     /// Our heroic `Note` model.
///     pub struct Note -> NoteEncrypted {
///         /// The note's ID
///         id: NoteID,
///         /// Another public field
///         public_field: String,
///         /// And another public field
///         public_field2: u32,
///     }
///
///     struct NotePrivate {
///         /// The note's title
///         #[rasn(tag(explicit(0)))]
///         title: String,
///         /// Another private field
///         #[rasn(tag(explicit(1)))]
///         private_field1: u8,
///     }
/// }
/// ```
///
/// Here, `Note` is our unencrypted, opened container pub struct that contains *both the public and
/// private fields* (ie, `id`, `public_field`, `public_field`, `title`, `private_field1`).
/// `NoteEncrypted` is another public struct that holds all the public fields *and* one single
/// other field called `ciphertext` which is used to hold the *encrypted* private fields.
///
/// The `Note` struct implements [Encryptable][crate::models::crypto::Encryptable] with its output
/// set as `NoteEncrypted`. `NotePrivate` is a privately-scoped struct *only used as an
/// intermediate container for encryption*.
///
/// Note that all the fields within `NotePrivate` need to implement [AsnType], [Encode], and
/// [Decode] in order to allow for serialization.
macro_rules! encryptable {
    (
        $(#[$meta:meta])*
        pub struct $name:ident -> $name_encrypted:ident {
            $($(#[$pubmeta:meta])* $pubfield:ident : $pubty:ty,)* 
        }

        struct $name_private:ident {
            $($(#[doc=$privdoc:expr])* #[rasn(tag(explicit($privtag:expr)))] $privfield:ident : $privty:ty,) *
        }
    ) => {
        #[derive(Getters, Deserialize, Serialize)]
        #[getset(get = "pub")]
        $(#[$meta])*
        pub struct $name {
            $($(#[$pubmeta])* $pubfield: $pubty,)*
            $($(#[doc=$privdoc])* $privfield: $privty,)*
        }

        #[derive(rasn::AsnType, rasn::Encode, rasn::Decode)]
        struct $name_private {
            $($(#[doc=$privdoc])* #[rasn(tag(explicit($privtag)))] $privfield: $privty,)*
        }

        #[derive(Getters, Deserialize, Serialize)]
        #[getset(get = "pub")]
        pub struct $name_encrypted {
            $($pubfield: $pubty,)*
            ciphertext: stamp_core::crypto::base::Sealed,
        }

        impl crate::models::Encryptable for $name {
            type Output = $name_encrypted;

            fn encrypt(self, secret_key: &stamp_core::crypto::base::SecretKey) -> crate::error::Result<Self::Output> {
                let Self { $($pubfield,)* $($privfield,)* } = self;
                let private = $name_private { $($privfield,)* };
                let serialized = rasn::der::encode(&private).map_err(|_| crate::error::Error::ASNSerialize)?;
                let sealed = stamp_core::crypto::seal::seal(secret_key, &serialized[..])?;
                Ok(Self::Output {
                    $($pubfield,)*
                    ciphertext: sealed,
                })
            }

            fn decrypt(encrypted: Self::Output, secret_key: &stamp_core::crypto::base::SecretKey) -> crate::error::Result<Self> {
                let Self::Output { $($pubfield,)* ciphertext: sealed } = encrypted;
                let opened = stamp_core::crypto::seal::open(secret_key, &sealed)?;
                let $name_private { $($privfield,)* } = rasn::der::decode(&opened[..]).map_err(|_| crate::error::Error::ASNDeserialize)?;
                Ok(Self {
                    $($pubfield,)*
                    $($privfield,)*
                })
            }
        }
    }
}
// make it rain
pub(crate) use encryptable;
*/

