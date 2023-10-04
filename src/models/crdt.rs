//! The CRDT module defines a *safe space* for granularly mutating Turtl model data.
//!
//! Instead of setting full objects, Turtl allows issuing mutations against those objects and
//! tracks each of the changes in order. When replayed in order, the full objects can be
//! constructed in their entirety. This allows collaboration on data within Turtl with minimal
//! conflict.
//!
//! When object reach a certain threshold of the number of changes they track, they remove old CRDT
//! records and issue "checkpoints" that essentially bundle/run a number of CRDTs as a group and
//! collapse them all together. These checkpoints allow the CRDT chain to "break" without harming
//! the integrity of the hash chains (see Stamp for details since CRDTs are Stamp transactions) but
//! offer a way to compress old data so we don't have millions of CRDTs lying around because you
//! typed a handful of paragraphs into a Turtl note.

use crate::{
    error::{Error, Result},
    models::{
        Encryptable,
        file::{File, FileChunk, FileCrdt, FileID},
        note::{Note, NoteCrdt, NoteID, Section, SectionID, Tag},
        page::{Display, Page, PageCrdt, PageID, Slice},
        space::{Member, MemberID, Role, Space, SpaceCrdt, SpaceID},
        user::UserCrdt,
    },
};
use getset::Getters;
use rasn::{AsnType, Decode, Encode};
use serde::{Deserialize, Serialize};
use stamp_core::{
    crypto::{
        base::{Sealed, SecretKey},
        seal,
    },
    dag::TransactionID,
};

/// Defines an operation that runs at an acceptable level of granularity such that, for each
/// object, when run *in order* the operations can construct the object in its entirety.
///
/// This is an outer container that contains CRDTs of the main inner types/models, as well as a
/// checkpoint CRDT that is used to roll-up/compress many CRDTs into one so older records can be
/// expunged.
///
/// You might notice that in the per-type CRDTs (`NoteCrdt`, `PageCrdt`, etc) that each action
/// doesn't reference the object id itself. For instance, `PageCrdt::PageTitleSet` doesn't have a
/// `PageID` member...this is because the context of which object this is happening on is stored at
/// a higher level in `Crdt.context`. So this is intentional. This design allows us to store the
/// context separately from the CRDT body itself so the application can determine what things a
/// CRDT applies to without having to decrypt the (potentially large) full CRDT.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize)]
#[rasn(choice)]
pub enum CrdtAction {
    #[rasn(tag(explicit(0)))]
    Checkpoint {
        #[rasn(tag(explicit(0)))]
        action: Box<CrdtAction>,
        #[rasn(tag(explicit(1)))]
        replaces: Vec<TransactionID>,
    },
    #[rasn(tag(explicit(1)))]
    File(FileCrdt),
    #[rasn(tag(explicit(2)))]
    Note(NoteCrdt),
    #[rasn(tag(explicit(3)))]
    Page(PageCrdt),
    #[rasn(tag(explicit(4)))]
    Space(SpaceCrdt),
    #[rasn(tag(explicit(5)))]
    User(UserCrdt),
}

/// Defines a context a CRDT belongs to. Allows an application to determine which CRDTs it cares
/// about quickly without having to decrypt the entire CRDT which could potentially be large.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct CrdtContext {
    #[rasn(tag(explicit(0)))]
    is_checkpoint: bool,
    #[rasn(tag(explicit(1)))]
    file: Option<FileID>,
    #[rasn(tag(explicit(2)))]
    note: Option<NoteID>,
    #[rasn(tag(explicit(3)))]
    page: Option<PageID>,
    #[rasn(tag(explicit(4)))]
    space: Option<SpaceID>,
}

impl CrdtContext {
    fn new(space: Option<SpaceID>, file: Option<FileID>, note: Option<NoteID>, page: Option<PageID>) -> Self {
        Self { is_checkpoint: false, file, note, page, space }
    }

    fn new_with_checkpoint(is_checkpoint: bool, space: Option<SpaceID>, file: Option<FileID>, note: Option<NoteID>, page: Option<PageID>) -> Self {
        Self { is_checkpoint, file, note, page, space }
    }
}

/// Defines a CRDT, and the context(s) it runs within.
///
/// This doesn't have an ID because it will essentially use the Stamp protocol's
/// [`TransactionID`] as its id.
#[derive(Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct Crdt {
    /// Contexts for this CRDT. The idea here is that we can read which note, file, page, space, etc
    /// this CRDT applies to *without having to decrypt the entire object* which is potentially
    /// large. This helps a client determine if a CRDT is worth decrypting. For instance, on
    /// initial load, we might want to index spaces, pages, and notes but ignore files. This allows
    /// us to decrypt our CRDT context quickly and determine WHAT it is without having to open
    /// everything.
    ///
    /// Note that because the `SpaceID`s are important for routing, they are moved out of the
    /// context and put *unencrypted* in the [`CrdtEncrypted`] container, while the other contexts
    /// *are* encrypted, but put into a separate field. They can be accessed using
    /// [`CrdtEncrypted::get_full_context`].
    context: CrdtContext,
    /// The actual CRDT action/operation we're running.
    action: CrdtAction,
}

impl Crdt {
    /// Create a file
    pub fn file_set(space_id: SpaceID, file: File) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), Some(file.id().clone()), None, None),
            action: CrdtAction::File(FileCrdt::Set(file)),
        }
    }

    /// Create a file chunk
    pub fn file_set_chunk(space_id: SpaceID, file_id: FileID, chunk: FileChunk) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), Some(file_id), None, None),
            action: CrdtAction::File(FileCrdt::SetChunk(chunk)),
        }
    }

    /// Set a file's name
    pub fn file_set_name(space_id: SpaceID, file_id: FileID, name: String) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), Some(file_id), None, None),
            action: CrdtAction::File(FileCrdt::SetName(name)),
        }
    }

    /// Delete a file
    pub fn file_unset(space_id: SpaceID, file_id: FileID) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), Some(file_id), None, None),
            action: CrdtAction::File(FileCrdt::Unset),
        }
    }

    /// Set/create a whole note. Mainly useful for moving notes across space lines, or for creating
    /// checkpoints.
    pub fn note_set(space_id: SpaceID, note: Note) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, Some(note.id().clone()), None),
            action: CrdtAction::Note(NoteCrdt::Set(note)),
        }
    }

    /// Create a body section in a note
    pub fn note_set_body_section(space_id: SpaceID, note_id: NoteID, section_id: SectionID, section: Section, after: Option<SectionID>) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, Some(note_id), None),
            action: CrdtAction::Note(NoteCrdt::SetBodySection {
                section_id,
                section,
                after,
            }),
        }
    }

    /// Attach a tag to a note
    pub fn note_set_tag(space_id: SpaceID, note_id: NoteID, tag: Tag) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, Some(note_id), None),
            action: CrdtAction::Note(NoteCrdt::SetTag(tag)),
        }
    }

    /// Set a note's title
    pub fn note_title_set(space_id: SpaceID, note_id: NoteID, title: Option<String>) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, Some(note_id), None),
            action: CrdtAction::Note(NoteCrdt::SetTitle(title)),
        }
    }

    /// Remove a note
    pub fn note_unset(space_id: SpaceID, note_id: NoteID) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, Some(note_id), None),
            action: CrdtAction::Note(NoteCrdt::Unset),
        }
    }

    /// Remove a body section
    pub fn note_unset_body_section(space_id: SpaceID, note_id: NoteID, section_id: SectionID) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, Some(note_id), None),
            action: CrdtAction::Note(NoteCrdt::UnsetBodySection(section_id)),
        }
    }

    /// Detach a tag from a note
    pub fn note_unset_tag(space_id: SpaceID, note_id: NoteID, tag: Tag) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, Some(note_id), None),
            action: CrdtAction::Note(NoteCrdt::UnsetTag(tag)),
        }
    }

    /// Create a full page, generally useful for moving across space lines or creating checkpoints.
    pub fn page_set(space_id: SpaceID, page: Page) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, Some(page.id().clone())),
            action: CrdtAction::Page(PageCrdt::Set(page)),
        }
    }

    /// Set a page's view
    pub fn page_set_display(space_id: SpaceID, page_id: PageID, display: Display) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, Some(page_id)),
            action: CrdtAction::Page(PageCrdt::SetDisplay(display)),
        }
    }

    /// Set a page's slice
    pub fn page_set_slice(space_id: SpaceID, page_id: PageID, slice: Slice) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, Some(page_id)),
            action: CrdtAction::Page(PageCrdt::SetSlice(slice)),
        }
    }

    /// Set a page's title
    pub fn page_set_title(space_id: SpaceID, page_id: PageID, title: String) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, Some(page_id)),
            action: CrdtAction::Page(PageCrdt::SetTitle(title)),
        }
    }

    /// Unalive a page
    pub fn page_unset(space_id: SpaceID, page_id: PageID) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, Some(page_id)),
            action: CrdtAction::Page(PageCrdt::Unset),
        }
    }

    /// Set a full space. Mainly for checkpointing.
    pub fn space_set(space: Space) -> Self {
        Self {
            context: CrdtContext::new(Some(space.id().clone()), None, None, None),
            action: CrdtAction::Space(SpaceCrdt::Set(space)),
        }
    }

    /// Set a space's color, although the only color allowed is black. Like my soul.
    pub fn space_set_color(space_id: SpaceID, color: Option<String>) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, None),
            action: CrdtAction::Space(SpaceCrdt::SetColor(color)),
        }
    }

    /// Create a new member in this space.
    pub fn space_set_member(member: Member) -> Self {
        Self {
            context: CrdtContext::new(Some(member.space_id().clone()), None, None, None),
            action: CrdtAction::Space(SpaceCrdt::SetMember(member)),
        }
    }

    /// Set a new role for a member.
    pub fn space_set_member_role(space_id: SpaceID, member_id: MemberID, role: Role) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, None),
            action: CrdtAction::Space(SpaceCrdt::SetMemberRole {
                member_id,
                role,
            }),
        }
    }

    /// Set this space's title
    pub fn space_set_title(space_id: SpaceID, title: String) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, None),
            action: CrdtAction::Space(SpaceCrdt::SetTitle(title)),
        }
    }

    /// Remove this space, including all data held within it. Careful!
    pub fn space_unset(space_id: SpaceID) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, None),
            action: CrdtAction::Space(SpaceCrdt::Unset),
        }
    }

    /// Eject someone from the space.
    pub fn space_unset_member(space_id: SpaceID, member_id: MemberID) -> Self {
        Self {
            context: CrdtContext::new(Some(space_id), None, None, None),
            action: CrdtAction::Space(SpaceCrdt::UnsetMember(member_id)),
        }
    }

    /// Set the user's default space.
    pub fn user_set_settings_default_space(space_id: Option<SpaceID>) -> Self {
        Self {
            context: CrdtContext::new(None, None, None, None),
            action: CrdtAction::User(UserCrdt::SetSettingsDefaultSpace(space_id)),
        }
    }
}

impl Encryptable for Crdt {
    type Output = CrdtEncrypted;

    fn encrypt(self, secret_key: &SecretKey) -> Result<Self::Output> {
        let Self { context, action } = self;
        let CrdtContext { is_checkpoint, file, note, page, space } = context;
        let context_no_space = CrdtContext::new_with_checkpoint(is_checkpoint, None, file, note, page);
        let serialized_context = rasn::der::encode(&context_no_space).map_err(|_| Error::ASNSerialize)?;
        let serialized_crdt = rasn::der::encode(&action).map_err(|_| Error::ASNSerialize)?;
        let sealed_context = seal::seal(secret_key, &serialized_context[..])?;
        let sealed_crdt = seal::seal(secret_key, &serialized_crdt[..])?;
        Ok(Self::Output {
            context: space,
            ciphertext_context: sealed_context,
            ciphertext_crdt: sealed_crdt,
        })
    }

    fn decrypt(encrypted: &Self::Output, secret_key: &SecretKey) -> crate::error::Result<Self> {
        let Self::Output { context: ref context_space, ref ciphertext_context, ref ciphertext_crdt } = encrypted;
        let opened_context = seal::open(secret_key, ciphertext_context)?;
        let opened_crdt = seal::open(secret_key, ciphertext_crdt)?;
        let CrdtContext { file, note, page, .. } = rasn::der::decode(&opened_context[..]).map_err(|_| crate::error::Error::ASNDeserialize)?;
        let action: CrdtAction = rasn::der::decode(&opened_crdt[..]).map_err(|_| crate::error::Error::ASNDeserialize)?;

        let context = CrdtContext::new(context_space.clone(), file, note, page);
        Ok(Self {
            context,
            action,
        })
    }
}

/// Basically, a [`Crdt`] but with the `action` field serialized and encrypted, and the `context`
/// field also encrypted, but only after lifting `space` out of the context and shoving it into the
/// `context` field as a `Option<SpaceID>`.
///
/// To turn this into a [`Crdt`], do:
///
/// ```ignore
/// let crdt_encrypted: CrdtEncrypted = ...;
/// let crdt = Crdt::decrypt(crdt_encrypted, &my_secret_key)?;
/// ```
///
/// Make sure you have [`Encryptable`] imported.
#[derive(AsnType, Encode, Decode, Deserialize, Serialize, Getters)]
#[getset(get = "pub")]
pub struct CrdtEncrypted {
    /// The space context(s) this CRDT happens within.
    ///
    /// This is used for protocol routing, since sharing happens at the space level. Generally,
    /// this is a single space ID, but it can be blank if updating user settings (which is
    /// spaceless) or can be multiple spaces if moving an object from one space to another.
    #[rasn(tag(explicit(0)))]
    context: Option<SpaceID>,
    /// Our encrypted [`CrdtContext`] which tells the application what kind of CRDT this is without
    /// having to decrypt the whole CRDT body data.
    #[rasn(tag(explicit(1)))]
    #[getset(skip)]
    ciphertext_context: Sealed,
    /// The actual CRDT action/operation we're running.
    #[rasn(tag(explicit(2)))]
    #[getset(skip)]
    ciphertext_crdt: Sealed,
}

impl CrdtEncrypted {
    /// Decrypts this CRDT's full context and returns it on a platter with french fried potatoes.
    pub fn get_full_context(&self, secret_key: &SecretKey) -> Result<CrdtContext> {
        let opened_context = seal::open(secret_key, &self.ciphertext_context)?;
        let CrdtContext { is_checkpoint, file, note, page, .. } = rasn::der::decode(&opened_context[..]).map_err(|_| crate::error::Error::ASNDeserialize)?;

        Ok(CrdtContext::new_with_checkpoint(is_checkpoint, self.context.clone(), file, note, page))
    }
}

