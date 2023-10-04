//! The state is a collection of full models built from playing [CRDTs][crate::models::crdt] in
//! order. It has the ability to diff with another state and issue CRDTs to reconcile them. This
//! makes it easy to issue a new version of a state, compare to the previous version, and track all
//! the changes that need to happen to get from A -> B.

use crate::{
    error::{Error, Result},
    models::{
        crdt::Crdt,
        file::{File, FileID},
        note::{Note, NoteID},
        page::{Page, PageID},
        space::{Space, SpaceID},
        user::UserSettings,
    },
};
use getset::Getters;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An object that represents application state. This is built by applying CRDTs in order.
#[derive(Default, Serialize, Deserialize, Getters)]
#[getset(get = "pub")]
pub struct State {
    files: HashMap<FileID, File>,
    notes: HashMap<NoteID, Note>,
    pages: HashMap<PageID, Page>,
    spaces: HashMap<SpaceID, Space>,
    user_settings: UserSettings,
}

impl State {
    /// Apply a CRDT to this state object.
    pub fn apply_crdt(&mut self, crdt: &Crdt) -> Result<()> {
        Ok(())
    }
}

