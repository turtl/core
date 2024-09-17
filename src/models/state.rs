//! The state is a collection of full models built from playing [operations][crate::models::operation]
//! in order.

use crate::{
    error::{Error, Result},
    models::{
        file::{File, FileChunk, FileChunkID, FileID},
        note::{Note, NoteID},
        operation::{Operation, OperationAction},
        page::{Page, PageID},
        space::{Space, SpaceID},
        user::UserSettings,
    },
};
use getset::{Getters, MutGetters};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An object that represents application state. This is built by applying operations in order.
#[derive(Default, Serialize, Deserialize, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub(crate)")]
pub struct State {
    chunks: HashMap<FileChunkID, FileChunk>,
    files: HashMap<FileID, File>,
    notes: HashMap<NoteID, Note>,
    pages: HashMap<PageID, Page>,
    spaces: HashMap<SpaceID, Space>,
    user_settings: UserSettings,
}

impl State {
    /// Create a new state object
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply an operation to this state object.
    pub fn apply_operation(&mut self, operation: Operation) -> Result<()> {
        let (context, action) = operation.consume();
        macro_rules! get_context {
            ($ty:ident) => {
                context.$ty().as_ref().ok_or_else(|| Error::OperationMissingContext(format!("Missing context {}", stringify!($ty))))
            }
        }
        if let Some(space_id) = context.space() {
            match action {
                OperationAction::FileSetV1(file) => {
                    self.files_mut().insert(file.id().clone(), file);
                }
                OperationAction::FileSetChunkV1(chunk) => {
                    self.chunks_mut().insert(chunk.id().clone(), chunk);
                }
                OperationAction::FileSetNameV1(name) => {
                    let file_id = get_context! { file }?;
                    if let Some(file) = self.files_mut().get_mut(file_id) {
                        *file.name_mut() = name;
                    }
                }
                OperationAction::FileUnsetV1 => {
                    let file_id = get_context! { file }?;
                    self.files_mut().remove(file_id);
                }
                OperationAction::NoteSetV1(note) => {
                    self.notes_mut().insert(note.id().clone(), note);
                }
                OperationAction::NoteSetBodySectionV1 { section_id, section, after } => {
                    let note_id = get_context! { note }?;
                    if let Some(note) = self.notes_mut().get_mut(note_id) {
                        note.body_mut().sections_mut().insert(section_id, section);
                    }
                }
                OperationAction::NoteSetTagV1(tag) => {
                }
                OperationAction::NoteSetTitleV1(title) => {
                }
                OperationAction::NoteUnsetV1 => {
                }
                OperationAction::NoteUnsetBodySectionV1(section_id) => {
                }
                OperationAction::NoteUnsetTagV1(tag) => {
                }
                OperationAction::PageSetV1(page) => {
                }
                OperationAction::PageSetDisplayV1(display) => {
                }
                OperationAction::PageSetSliceV1(slice) => {
                }
                OperationAction::PageSetTitleV1(title) => {
                }
                OperationAction::PageUnsetV1 => {
                }
                OperationAction::SpaceSetV1(space) => {
                }
                OperationAction::SpaceSetColorV1(color) => {
                }
                OperationAction::SpaceSetMemberV1(member) => {
                }
                OperationAction::SpaceSetMemberRoleV1 { member_id, role } => {
                }
                OperationAction::SpaceSetTitleV1(title) => {
                }
                OperationAction::SpaceUnsetV1 => {
                }
                OperationAction::SpaceUnsetMemberV1(member_id) => {
                }
                _ => Err(Error::OperationInvalid("User operation in non-user context".into()))?,
            }
        } else {
            // this operation has no space context, therefor it MUST be user-specific.
            match action {
                OperationAction::UserSetSettingsV1(settings) => {
                    *self.user_settings_mut() = settings;
                }
                OperationAction::UserSetSettingsDefaultSpaceV1(space) => {
                    *self.user_settings_mut().default_space_mut() = space;
                }
                _ => Err(Error::OperationInvalid("Non-user operation in user context".into()))?,
            }
        }
        Ok(())
    }
}

