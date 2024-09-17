//! Defines our error system.

use crate::models::space::SpaceID;
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

    /// An operation is invalid.
    #[error("Invalid operation: {0}")]
    OperationInvalid(String),

    /// An operation is missing much-needed context
    #[error("Operation: missing context {0}")]
    OperationMissingContext(String),

    /// An error from the stamp core protocol
    #[error("Stamp error: {0}")]
    Stamp(#[from] StampError),

    /// Couldn't deserialize some serialized portion(s) of a transaction.
    #[error("Transaction {0} couldn't be deserialized")]
    TransactionDeserializationError(TransactionID, rasn::error::DecodeError),

    /// Couldn't find the space key to decrypt this transaction =[
    #[error("Transaction {0}: space key {:1} missing")]
    TransactionMissingSpaceKey(TransactionID, SpaceID),

    /// General error processing a transaction
    #[error("Transaction {0}: error: {1}")]
    TransactionStampError(TransactionID, Box<Error>),

    /// The given Stamp transaction was not the right type
    #[error("Transaction {0} is the wrong type (need turtl/op/*)")]
    TransactionWrongType(TransactionID),

    /// The given Stamp transaction was not the right type
    #[error("Transaction {0} is the wrong variant (need ExtV1)")]
    TransactionWrongVariant(TransactionID),
}

/// Wraps `std::result::Result` around our `Error` enum
pub type Result<T> = std::result::Result<T, Error>;

