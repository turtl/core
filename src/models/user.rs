//! The user system is basically just a user-global settings object. "Users" in Turtl are
//! effectively just Stamp identities, so there's no real concept of a user outside of a handful of
//! cross-device settings.

use crate::models::{
    space::SpaceID,
};
use getset::Getters;
use rasn::{AsnType, Decode, Encode};
use serde::{Deserialize, Serialize};

/// Defines the actions we can perform on user stuff
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum UserCrdt {
    /// Set the default space in the user's settings LOL
    #[rasn(tag(explicit(0)))]
    SetSettingsDefaultSpace(Option<SpaceID>),
}

/// A user's settings
#[derive(Default, AsnType, Encode, Decode, Deserialize, Getters, Serialize)]
#[getset(get = "pub")]
pub struct UserSettings {
    /// The space we show when the user logs in
    #[rasn(tag(explicit(0)))]
    default_space: Option<SpaceID>
}

