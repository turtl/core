//! Defines our error system.

use stamp_core::{
    dag::TransactionID,
    error::{Error as StampError}
};
use thiserror::Error;

/// Holds the various failures we can experience using the Turtl core.
#[derive(Debug, Error)]
pub enum Error {
    /// An error that happened during deserialization
    #[error("ASN deserialization error")]
    ASNDeserialize,

    /// An error that happened during serialization
    #[error("ASN serialization error")]
    ASNSerialize,

    /// A CRDT is missing much-needed context
    #[error("CRDT: {0} missing context {1}")]
    CrdtMissingContext(TransactionID, String),

    /// An error from the stamp core protocol
    #[error("stamp error: {0}")]
    Stamp(#[from] StampError)
}

/// Wraps `std::result::Result` around our `Error` enum
pub type Result<T> = std::result::Result<T, Error>;

