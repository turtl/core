//! The operation module defines a *safe space* for granularly mutating Turtl model data.
//! Operations could probably be thought of as CRDT or CRDT-like.
//!
//! Instead of setting full objects, Turtl allows issuing mutations against those objects and
//! tracks each of the changes in order. When replayed in order, the full objects can be
//! constructed in their entirety. This allows collaboration on data within Turtl with minimal
//! conflict.
//!
//! Ordering is done using signed DAGs (aka, Merkle-DAGs) via the Stamp protocol. Stamp to the
//! rescue!

use crate::{
    error::{Error, Result},
    models::{
        Encryptable, ObjectID,

        file::{File, FileChunk, FileChunkID, FileID},
        note::{Note, NoteID, Section, SectionID, Tag},
        page::{Display, Page, PageID, Slice},
        space::{Member, MemberID, Role, Space, SpaceID},
        user::{UserSettings},
    },
};
use getset::Getters;
use rasn::{AsnType, Decode, Encode};
use serde::{Deserialize, Serialize};
use stamp_core::{
    crypto::{
        base::{HashAlgo, Sealed, SecretKey},
        seal,
    },
    dag::{Dag, Transaction, TransactionBody, TransactionID, Transactions},
    util::Timestamp,
};
use std::collections::HashMap;
use std::ops::Deref;

/// Defines an operation that runs at an acceptable level of granularity such that, for each
/// object, when run *in order* the operations can construct the object in its entirety.
///
/// You might notice that the operations don't reference the contexts they run in (note id, space
/// id, etc). These are stored at a higher level in `Operation.context` and are used for routing
/// and ordering.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum OperationAction {
    /// Add a file
    #[rasn(tag(explicit(0)))]
    FileSetV1(File),
    /// Create a file chunk
    #[rasn(tag(explicit(1)))]
    FileSetChunkV1(FileChunk),
    /// Set a file's name
    #[rasn(tag(explicit(2)))]
    FileSetNameV1(String),
    /// Remove a file
    #[rasn(tag(explicit(3)))]
    FileUnsetV1,
    /// Create a full note.
    #[rasn(tag(explicit(4)))]
    NoteSetV1(Note),
    /// Add a new section to this note
    #[rasn(tag(explicit(5)))]
    NoteSetBodySectionV1 {
        #[rasn(tag(explicit(0)))]
        section_id: SectionID,
        #[rasn(tag(explicit(1)))]
        section: Section,
        #[rasn(tag(explicit(2)))]
        after: Option<SectionID>,
    },
    /// Set the indent on a section
    #[rasn(tag(explicit(6)))]
    NoteSetBodySectionIndentV1 {
        #[rasn(tag(explicit(0)))]
        section_id: SectionID,
        #[rasn(tag(explicit(1)))]
        indent: u8,
    },
    /// Re-order a section
    #[rasn(tag(explicit(7)))]
    NoteSetBodySectionOrderV1 {
        #[rasn(tag(explicit(0)))]
        section_id: SectionID,
        #[rasn(tag(explicit(1)))]
        after: Option<SectionID>,
    },
    /// Mark a note as deleted. This is effectively putting it into the trash as opposed to
    /// deleting it outright. Full deletion is done via `NoteUnsetV1`.
    NoteSetDeletedV1(bool),
    /// Add a tag to this note
    #[rasn(tag(explicit(8)))]
    NoteSetTagV1(Tag),
    /// Set this note's title LOL
    #[rasn(tag(explicit(9)))]
    NoteSetTitleV1(Option<String>),
    /// Remove a note
    #[rasn(tag(explicit(10)))]
    NoteUnsetV1,
    /// Remove a section
    #[rasn(tag(explicit(11)))]
    NoteUnsetBodySectionV1(SectionID),
    /// Remove a tag
    #[rasn(tag(explicit(12)))]
    NoteUnsetTagV1(Tag),
    /// Create a page
    #[rasn(tag(explicit(13)))]
    PageSetV1(Page),
    /// Mark a page as deleted. This moves it to the trash as opposed to deleting it outright. A
    /// full delete happens via `PageUnsetV1`.
    PageSetDeleted(bool),
    /// Set a page's display
    #[rasn(tag(explicit(14)))]
    PageSetDisplayV1(Display),
    /// Set a page's slice
    #[rasn(tag(explicit(15)))]
    PageSetSliceV1(Slice),
    /// Set a page's title
    #[rasn(tag(explicit(16)))]
    PageSetTitleV1(String),
    /// Delete a page
    #[rasn(tag(explicit(17)))]
    PageUnsetV1,
    /// Set a space into existence
    #[rasn(tag(explicit(18)))]
    SpaceSetV1(Space),
    /// Set the space's color
    #[rasn(tag(explicit(19)))]
    SpaceSetColorV1(Option<String>),
    /// Sets a full member object
    #[rasn(tag(explicit(20)))]
    SpaceSetMemberV1(Member),
    /// Set a member's role
    #[rasn(tag(explicit(21)))]
    SpaceSetMemberRoleV1 {
        #[rasn(tag(explicit(0)))]
        member_id: MemberID,
        #[rasn(tag(explicit(1)))]
        role: Role,
    },
    /// Set the space's title
    #[rasn(tag(explicit(22)))]
    SpaceSetTitleV1(String),
    /// Delete a space.
    #[rasn(tag(explicit(23)))]
    SpaceUnsetV1,
    /// Remove a member from this space
    #[rasn(tag(explicit(24)))]
    SpaceUnsetMemberV1(MemberID),
    /// Set all settings
    #[rasn(tag(explicit(25)))]
    UserSetSettingsV1(UserSettings),
    /// Set the default space in the user's settings LOL
    #[rasn(tag(explicit(26)))]
    UserSetSettingsDefaultSpaceV1(Option<SpaceID>),
}

/// Defines a context an operation belongs to. Allows an application to determine which ops it cares
/// about quickly without having to decrypt the entire thing which could potentially be large.
#[derive(Default, AsnType, Encode, Decode, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct OperationContext {
    #[rasn(tag(explicit(0)))]
    chunk: Option<FileChunkID>,
    #[rasn(tag(explicit(1)))]
    file: Option<FileID>,
    #[rasn(tag(explicit(2)))]
    note: Option<NoteID>,
    #[rasn(tag(explicit(3)))]
    page: Option<PageID>,
    #[rasn(tag(explicit(4)))]
    space: Option<SpaceID>,
}

impl OperationContext {
    fn new(space: Option<SpaceID>, chunk: Option<FileChunkID>, file: Option<FileID>, note: Option<NoteID>, page: Option<PageID>) -> Self {
        Self { chunk, file, note, page, space }
    }
}

/// Defines an operation, and the context(s) it runs within.
///
/// This doesn't have an ID because it will essentially use the Stamp protocol's
/// [`TransactionID`] as its id.
#[derive(Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct Operation {
    /// This stores the operation's contexts (space id, note id, etc)
    context: OperationContext,
    /// The actual operation we're running.
    action: OperationAction,
}

impl Operation {
    /// Consume this operation, returning the context and action.
    pub fn consume(self) -> (OperationContext, OperationAction) {
        let Operation { context, action } = self;
        (context, action)
    }

    /// Create a file
    pub fn file_set(space_id: SpaceID, file: File) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, Some(file.id().clone()), None, None),
            action: OperationAction::FileSetV1(file),
        }
    }

    /// Create a file chunk
    pub fn file_set_chunk(space_id: SpaceID, file_id: FileID, chunk: FileChunk) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), Some(chunk.id().clone()), Some(file_id), None, None),
            action: OperationAction::FileSetChunkV1(chunk),
        }
    }

    /// Set a file's name
    pub fn file_set_name(space_id: SpaceID, file_id: FileID, name: String) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, Some(file_id), None, None),
            action: OperationAction::FileSetNameV1(name),
        }
    }

    /// Delete a file
    pub fn file_unset(space_id: SpaceID, file_id: FileID) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, Some(file_id), None, None),
            action: OperationAction::FileUnsetV1,
        }
    }

    /// Set/create a whole note. Mainly useful for moving notes across space lines, or for creating
    /// checkpoints.
    pub fn note_set(space_id: SpaceID, note: Note) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, Some(note.id().clone()), None),
            action: OperationAction::NoteSetV1(note),
        }
    }

    /// Create a body section in a note
    pub fn note_set_body_section(space_id: SpaceID, note_id: NoteID, section_id: SectionID, section: Section, after: Option<SectionID>) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, Some(note_id), None),
            action: OperationAction::NoteSetBodySectionV1 {
                section_id,
                section,
                after,
            },
        }
    }

    /// Mark a note as (un)deleted
    pub fn note_set_deleted(space_id: SpaceID, node_id: NoteID, deleted: bool) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, Some(note_id), None),
            action: OperationAction::NoteSetDeletedV1(deleted),
        }
    }

    /// Attach a tag to a note
    pub fn note_set_tag(space_id: SpaceID, note_id: NoteID, tag: Tag) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, Some(note_id), None),
            action: OperationAction::NoteSetTagV1(tag),
        }
    }

    /// Set a note's title
    pub fn note_set_title(space_id: SpaceID, note_id: NoteID, title: Option<String>) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, Some(note_id), None),
            action: OperationAction::NoteSetTitleV1(title),
        }
    }

    /// Remove a note
    pub fn note_unset(space_id: SpaceID, note_id: NoteID) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, Some(note_id), None),
            action: OperationAction::NoteUnsetV1,
        }
    }

    /// Remove a body section
    pub fn note_unset_body_section(space_id: SpaceID, note_id: NoteID, section_id: SectionID) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, Some(note_id), None),
            action: OperationAction::NoteUnsetBodySectionV1(section_id),
        }
    }

    /// Detach a tag from a note
    pub fn note_unset_tag(space_id: SpaceID, note_id: NoteID, tag: Tag) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, Some(note_id), None),
            action: OperationAction::NoteUnsetTagV1(tag),
        }
    }

    /// Create a full page, generally useful for moving across space lines or creating checkpoints.
    pub fn page_set(space_id: SpaceID, page: Page) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, Some(page.id().clone())),
            action: OperationAction::PageSetV1(page),
        }
    }

    /// Mark a page as (un)deleted
    pub fn page_set_deleted(space_id: SpaceID, page_id: PageID, deleted: bool) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, Some(page_id)),
            action: OperationAction::PageSetDeletedV1(deleted),
        }
    }

    /// Set a page's view
    pub fn page_set_display(space_id: SpaceID, page_id: PageID, display: Display) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, Some(page_id)),
            action: OperationAction::PageSetDisplayV1(display),
        }
    }

    /// Set a page's slice
    pub fn page_set_slice(space_id: SpaceID, page_id: PageID, slice: Slice) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, Some(page_id)),
            action: OperationAction::PageSetSliceV1(slice),
        }
    }

    /// Set a page's title
    pub fn page_set_title(space_id: SpaceID, page_id: PageID, title: String) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, Some(page_id)),
            action: OperationAction::PageSetTitleV1(title),
        }
    }

    /// Unalive a page
    pub fn page_unset(space_id: SpaceID, page_id: PageID) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, Some(page_id)),
            action: OperationAction::PageUnsetV1,
        }
    }

    /// Set a full space. Mainly for checkpointing.
    pub fn space_set(space: Space) -> Self {
        Self {
            context: OperationContext::new(Some(space.id().clone()), None, None, None, None),
            action: OperationAction::SpaceSetV1(space),
        }
    }

    /// Set a space's color, although the only color allowed is black. Like my soul.
    pub fn space_set_color(space_id: SpaceID, color: Option<String>) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, None),
            action: OperationAction::SpaceSetColorV1(color),
        }
    }

    /// Create a new member in this space.
    pub fn space_set_member(member: Member) -> Self {
        Self {
            context: OperationContext::new(Some(member.space_id().clone()), None, None, None, None),
            action: OperationAction::SpaceSetMemberV1(member),
        }
    }

    /// Set a new role for a member.
    pub fn space_set_member_role(space_id: SpaceID, member_id: MemberID, role: Role) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, None),
            action: OperationAction::SpaceSetMemberRoleV1 {
                member_id,
                role,
            },
        }
    }

    /// Set this space's title
    pub fn space_set_title(space_id: SpaceID, title: String) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, None),
            action: OperationAction::SpaceSetTitleV1(title),
        }
    }

    /// Remove this space, including all data held within it. Careful!
    pub fn space_unset(space_id: SpaceID) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, None),
            action: OperationAction::SpaceUnsetV1,
        }
    }

    /// Eject someone from the space.
    pub fn space_unset_member(space_id: SpaceID, member_id: MemberID) -> Self {
        Self {
            context: OperationContext::new(Some(space_id), None, None, None, None),
            action: OperationAction::SpaceUnsetMemberV1(member_id),
        }
    }

    /// Sets all user settings
    pub fn user_set_settings(settings: UserSettings) -> Self {
        Self {
            context: OperationContext::new(None, None, None, None, None),
            action: OperationAction::UserSetSettingsV1(settings),
        }
    }

    /// Set the user's default space.
    pub fn user_set_settings_default_space(space_id: Option<SpaceID>) -> Self {
        Self {
            context: OperationContext::new(None, None, None, None, None),
            action: OperationAction::UserSetSettingsDefaultSpaceV1(space_id),
        }
    }
}

impl Encryptable for Operation {
    type Output = OperationEncrypted;

    fn encrypt(self, secret_key: &SecretKey) -> Result<Self::Output> {
        let Self { context, action } = self;
        let OperationContext { chunk, file, note, page, space } = context;
        let context_no_space = OperationContext::new(None, chunk, file, note, page);
        let serialized_context = rasn::der::encode(&context_no_space).map_err(|_| Error::ASNSerialize)?;
        let serialized_action = rasn::der::encode(&action).map_err(|_| Error::ASNSerialize)?;
        let sealed_context = seal::seal(secret_key, &serialized_context[..])?;
        let sealed_action = seal::seal(secret_key, &serialized_action[..])?;
        Ok(Self::Output {
            context: space,
            ciphertext_context: sealed_context,
            ciphertext_action: sealed_action,
        })
    }

    fn decrypt(secret_key: &SecretKey, encrypted: &Self::Output) -> crate::error::Result<Self> {
        let Self::Output { context: ref context_space, ref ciphertext_context, ref ciphertext_action } = encrypted;
        let opened_context = seal::open(secret_key, ciphertext_context)?;
        let opened_action = seal::open(secret_key, ciphertext_action)?;
        let OperationContext { chunk, file, note, page, .. } = rasn::der::decode(&opened_context[..]).map_err(|_| crate::error::Error::ASNDeserialize)?;
        let action: OperationAction = rasn::der::decode(&opened_action[..]).map_err(|_| crate::error::Error::ASNDeserialize)?;

        let context = OperationContext::new(context_space.clone(), chunk, file, note, page);
        Ok(Self {
            context,
            action,
        })
    }
}

/// Basically, a [`Operation`] but with the `action` field serialized and encrypted, and the `context`
/// field also encrypted, but only after lifting `space` out of the context and shoving it into the
/// `context` field as a `Option<SpaceID>`.
///
/// To turn this into a [`Operation`], do:
///
/// ```ignore
/// let operation_encrypted: OperationEncrypted = ...;
/// let op = Operation::decrypt(&my_secret_key, &operation_encrypted)?;
/// ```
///
/// Make sure you have [`Encryptable`] imported.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct OperationEncrypted {
    /// The space context(s) this operation happens within.
    ///
    /// This is used for protocol routing, since sharing happens at the space level. Generally,
    /// this is a single space ID, but it can be blank if updating user settings (which is
    /// spaceless).
    #[rasn(tag(explicit(0)))]
    context: Option<SpaceID>,
    /// The (encrypted) context. This is separate from the action so we can determine and process
    /// the context without having to decrypt the entire operation which might be large/intensive.
    #[rasn(tag(explicit(1)))]
    #[getset(skip)]
    ciphertext_context: Sealed,
    /// The actual (encrypted) operation we're running
    #[rasn(tag(explicit(2)))]
    #[getset(skip)]
    ciphertext_action: Sealed,
}

impl OperationEncrypted {
    /// Decrypts this operation's full context and returns it on a platter with french fried potatoes.
    pub fn get_full_context(&self, secret_key: &SecretKey) -> Result<OperationContext> {
        let opened_context = seal::open(secret_key, &self.ciphertext_context)?;
        let OperationContext { chunk, file, note, page, .. } = rasn::der::decode(&opened_context[..]).map_err(|_| crate::error::Error::ASNDeserialize)?;
        Ok(OperationContext::new(self.context.clone(), chunk, file, note, page))
    }
}

/// Takes a flat list of stamp transactions, segments them by space, then converts them to DAGs.
pub fn group_operations_by_space<'a>(transactions: &'a Vec<Transaction>) -> (HashMap<Option<SpaceID>, Dag<'a>>, Vec<Error>) {
    let mut errors = Vec::new();
    let mut personal_transactions: Vec<&'a Transaction> = Vec::new();
    let mut space_group: HashMap<SpaceID, Vec<&'a Transaction>> = HashMap::new();
    for trans in transactions {
        match trans.entry().body() {
            TransactionBody::ExtV1 { ref creator, ref ty, ref context, ref payload, .. } => {
                if ty.as_ref().map(|x| x.deref().as_slice()) != Some(b"turtl/op/v1") {
                    errors.push(Error::TransactionWrongType(trans.id().clone()));
                    continue;
                }
                let space_id_ser = context.as_ref()
                    .and_then(|map| map.get(&b"space".to_vec().into()));
                let space_id = match space_id_ser {
                    Some(ser) => {
                        match rasn::der::decode::<SpaceID>(ser.as_slice()) {
                            Ok(s) => Some(s),
                            Err(e) => {
                                errors.push(Error::TransactionDeserializationError(trans.id().clone(), e));
                                continue;
                            }
                        }
                    },
                    None => None,
                };
                if let Some(space_id) = space_id {
                    space_group.entry(space_id).or_insert(Vec::new()).push(trans);
                } else {
                    personal_transactions.push(trans);
                }
            }
            _ => errors.push(Error::TransactionWrongVariant(trans.id().clone())),
        }
    }
    let mut result = HashMap::with_capacity(space_group.len() + 1);
    result.insert(None, Dag::from_transactions(&personal_transactions));
    for (space_id, transactions) in space_group {
        result.insert(Some(space_id), Dag::from_transactions(&transactions));
    }
    (result, errors)
}

/*
/// Takes a flat list of stamp transactions, segments them by space, then orders them, then
/// segments by object ID.
pub fn order_operations_(space_keys: &HashMap<SpaceID, SecretKey>, transactions: &Vec<Transaction>) -> (HashMap<Option<SpaceID>, Vec<Vec<OperationEncrypted>>>, Vec<Error>) {
    #[derive(Getters)]
    #[getset(get = "pub(crate)")]
    struct OperationTransaction<'t> {
        id: &'t TransactionID,
        created: &'t Timestamp,
        previous_transactions: &'t Vec<TransactionID>,
        context: OperationContext,
        operation: OperationEncrypted,
    }

    impl<'t> OperationTransaction<'t> {
        fn try_from_parts(space_key: &SecretKey, transaction: &'t Transaction, operation: OperationEncrypted) -> Result<Self> {
            let context = operation.get_full_context(space_key)?;
            Ok(Self {
                id: transaction.id(),
                created: transaction.entry().created(),
                previous_transactions: transaction.entry().previous_transactions(),
                context,
                operation,
            })
        }
    }

    let mut errors = Vec::new();
    let mut personal_transactions: Vec<OperationTransaction> = Vec::new();
    let mut space_group: HashMap<SpaceID, Vec<OperationTransaction>> = HashMap::new();

    for trans in transactions {
        match trans.entry().body() {
            TransactionBody::ExtV1 { ref creator, ref ty, ref context, ref payload, .. } => {
                if ty.as_ref().map(|x| x.deref().as_slice()) != Some(b"turtl/op/v1") {
                    errors.push(Error::TransactionWrongType(trans.id().clone()));
                    continue;
                }
                let space_id_ser = context.as_ref()
                    .and_then(|map| map.get(&b"space".to_vec().into()));
                let space_id = match space_id_ser {
                    Some(ser) => {
                        match rasn::der::decode::<SpaceID>(ser.as_slice()) {
                            Ok(s) => Some(s),
                            Err(e) => {
                                errors.push(Error::TransactionDeserializationError(trans.id().clone(), e));
                                continue;
                            }
                        }
                    },
                    None => None,
                };
                let mut operation_enc = match rasn::der::decode::<OperationEncrypted>(payload.as_slice()) {
                    Ok(x) => x,
                    Err(e) => {
                        errors.push(Error::TransactionDeserializationError(trans.id().clone(), e));
                        continue;
                    }
                };
                operation_enc.context = space_id.clone();
                let optrans = if let Some(space_id) = operation_enc.context.as_ref() {
                    let space_key = match space_keys.get(space_id) {
                        Some(k) => k,
                        None => {
                            errors.push(Error::TransactionMissingSpaceKey(trans.id().clone(), space_id.clone()));
                            continue;
                        }
                    };
                    match OperationTransaction::try_from_parts(space_key, trans, operation_enc) {
                        Ok(x) => x,
                        Err(e) => {
                            errors.push(Error::TransactionStampError(trans.id().clone(), Box::new(e)));
                            continue;
                        },
                    }
                } else {
                    OperationTransaction {
                        id: trans.id(),
                        created: trans.entry().created(),
                        previous_transactions: trans.entry().previous_transactions(),
                        context: OperationContext::default(),
                        operation: operation_enc,
                    }
                };
                if let Some(space_id) = space_id {
                    space_group.entry(space_id).or_insert(Vec::new()).push(optrans);
                } else {
                    personal_transactions.push(optrans);
                }
            }
            _ => errors.push(Error::TransactionWrongVariant(trans.id().clone())),
        }
    }

    let mut result = HashMap::new();

    let personal_ordered = order_operations_inner(&personal_transactions);
    result.insert(None, personal_ordered);
    (result, errors)
}
*/

