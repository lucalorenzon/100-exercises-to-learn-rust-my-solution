use std::fmt::Display;

use crate::{description::TicketDescription, status::Status, title::TicketTitle};

#[derive(Clone)] // ticket dto => Clone
pub struct Ticket {
    pub id: TicketId,
    pub title: TicketTitle,
    pub description: TicketDescription,
    pub status: Status,
}

#[derive(Clone)] //dto => Clone
pub struct DraftTicket {
    pub title: TicketTitle,
    pub description: TicketDescription,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)] // BTreeMap => Ord => PartialOrd => Eq => PartialEq, assert_eq => Debug
pub struct TicketId(u64);

impl From<u64> for TicketId {
    fn from(value: u64) -> Self {
        TicketId(value)
    }
}

impl Display for TicketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct TicketVersion(u64);

#[derive(Debug, thiserror::Error)]
#[error("VersionTicket too old")]
pub struct TicketVersionMismatchError;

#[derive(Clone)]
pub struct VersionedTicket {
    ticket: Ticket,
    version: TicketVersion,
}

#[derive(Default)]
pub struct PatchTicket {
    pub title: Option<TicketTitle>,
    pub description: Option<TicketDescription>,
    pub status: Option<Status>,
}

impl VersionedTicket {
    pub fn new(ticket: Ticket) -> Self {
        VersionedTicket {
            ticket,
            version: TicketVersion(0),
        }
    }

    pub fn version(&self) -> TicketVersion {
        self.version
    }

    pub fn id(&self) -> TicketId {
        self.ticket.id
    }

    pub fn title(&self) -> TicketTitle {
        self.ticket.title.clone()
    }

    pub fn description(&self) -> TicketDescription {
        self.ticket.description.clone()
    }

    pub fn status(&self) -> Status {
        self.ticket.status.clone()
    }

    pub fn try_patch(
        &mut self,
        version: &TicketVersion,
        patch: PatchTicket,
    ) -> Result<(), TicketVersionMismatchError> {
        if self.version() == *version {
            self.apply_patch(patch);
            self.version = TicketVersion(self.version.0 + 1);
            Ok(())
        } else {
            Err(TicketVersionMismatchError)
        }
    }

    fn apply_patch(&mut self, patch: PatchTicket) {
        if let Some(title) = patch.title {
            self.ticket.title = title;
        }
        if let Some(description) = patch.description {
            self.ticket.description = description;
        }
        if let Some(status) = patch.status {
            self.ticket.status = status;
        }
    }
}

impl From<u64> for TicketVersion {
    fn from(value: u64) -> Self {
        TicketVersion(value)
    }
}

#[cfg(test)]
pub mod test {
    use std::assert_matches;

    use crate::{
        description::tests::valid_description,
        status::Status,
        ticket::{
            DraftTicket, PatchTicket, Ticket, TicketVersion, TicketVersionMismatchError,
            VersionedTicket,
        },
        title::tests::valid_title,
    };

    #[test]
    fn should_increase_version_at_update() {
        let mut example_ticket =
            valid_versioned_ticket_with(42, "Title", "Description", Status::ToDo, 0);
        let patch = PatchTicket {
            title: "Title modified".try_into().ok(),
            description: "Description modified".try_into().ok(),
            status: Some(Status::InProgress),
        };

        example_ticket.try_patch(&0u64.into(), patch).unwrap();

        assert_eq!(TicketVersion(1), example_ticket.version());
    }

    #[test]
    fn should_raise_error_if_try_to_update_with_wrong_version() {
        let mut example_ticket =
            valid_versioned_ticket_with(42, "Title", "Description", Status::ToDo, 0);
        let patch = PatchTicket {
            title: "Title modified".try_into().ok(),
            description: "Description modified".try_into().ok(),
            status: Some(Status::InProgress),
        };

        let result = example_ticket.try_patch(&1u64.into(), patch);

        assert_matches!(result, Err(TicketVersionMismatchError));
    }

    pub fn valid_draft_ticket() -> DraftTicket {
        DraftTicket {
            title: valid_title(),
            description: valid_description(),
        }
    }

    pub fn valid_draft_ticket_with(title: &str, description: &str) -> DraftTicket {
        let title = title.try_into().unwrap_or(valid_title());
        let description = description.try_into().unwrap_or(valid_description());
        DraftTicket { title, description }
    }

    pub fn valid_ticket_with(id: u64, title: &str, description: &str, status: Status) -> Ticket {
        let title = title.try_into().unwrap_or(valid_title());
        let description = description.try_into().unwrap_or(valid_description());
        Ticket {
            id: id.into(),
            title,
            description,
            status,
        }
    }

    pub fn valid_versioned_ticket_with(
        id: u64,
        title: &str,
        description: &str,
        status: Status,
        version: u64,
    ) -> VersionedTicket {
        let ticket = valid_ticket_with(id, title, description, status);
        VersionedTicket {
            ticket,
            version: TicketVersion(version),
        }
    }
}
