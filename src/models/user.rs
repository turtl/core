//! The user system is basically just a user-global settings object. "Users" in Turtl are
//! effectively just Stamp identities, so there's no real concept of a user outside of a handful of
//! cross-device settings.

use crate::models::{
    space::SpaceID,
};
use getset::{Getters, MutGetters};
use rasn::{AsnType, Decode, Encode};
use serde::{Deserialize, Serialize};

/// A user's settings
#[derive(Default, AsnType, Encode, Decode, Deserialize, Getters, MutGetters, Serialize)]
#[getset(get = "pub", get_mut = "pub(crate)")]
pub struct UserSettings {
    /// The space we show when the user logs in
    #[rasn(tag(explicit(0)))]
    default_space: Option<SpaceID>
}

