//! Spaces are siloed containers for data, and are also the layer upon which collaboration is
//! implemented.
//!
//! Things in a space ONLY live in that space, which means spaces are how the routing layer of tp2p
//! knows which transactions go to which people.

use crate::models::object_id;
use getset::Getters;
use rasn::{AsnType, Decode, Encode};
use serde::{Deserialize, Serialize};
use stamp_core::identity::IdentityID;

object_id! {
    /// A unique space id
    SpaceID
}

object_id! {
    /// A unique ID for space members. In space, nobody hears you scream...
    MemberID
}

/// Defines a role a user can have within a space
#[derive(PartialEq, AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum Role {
    #[rasn(tag(explicit(0)))]
    #[serde(rename = "admin")]
    Admin,
    #[rasn(tag(explicit(1)))]
    #[serde(rename = "guest")]
    Guest,
    #[rasn(tag(explicit(2)))]
    #[serde(rename = "member")]
    Member,
    #[rasn(tag(explicit(3)))]
    #[serde(rename = "moderator")]
    Moderator,
    #[rasn(tag(explicit(4)))]
    #[serde(rename = "owner")]
    Owner,
}

/// A user that has access to a space
#[derive(AsnType, Encode, Decode, Deserialize, Getters, Serialize)]
#[getset(get = "pub")]
pub struct Member {
    /// This member's unique ID
    #[rasn(tag(explicit(0)))]
    id: MemberID,
    /// The space this member exists in
    #[rasn(tag(explicit(1)))]
    space_id: SpaceID,
    /// The user this member record points to
    #[rasn(tag(explicit(2)))]
    user_id: IdentityID,
    /// This member's role within the space
    #[rasn(tag(explicit(3)))]
    role: Role,
}

/// A space is a siloed container of notes and pages. It offers a way to keep these sets of data
/// completely separated from each other.
///
/// For instance, you might have a space for home, for work, for family, etc.
///
/// Spaces are also the mechanism for sharing data with other Turtl users.
#[derive(AsnType, Encode, Decode, Serialize, Deserialize, Getters)]
#[getset(get = "pub")]
pub struct Space {
    /// The space's unique ID
    #[rasn(tag(explicit(0)))]
    id: SpaceID,
    /// The members that can view, update, or manage this space.
    #[rasn(tag(explicit(1)))]
    members: Vec<Member>,
    /// The space's title
    #[rasn(tag(explicit(2)))]
    title: String,
    /// Sets the mood
    #[rasn(tag(explicit(3)))]
    color: Option<String>,
}

