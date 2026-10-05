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

#[cfg(test)]
pub mod test {
    use crate::{
        description::{tests::valid_description, TicketDescription},
        ticket::DraftTicket,
        title::{tests::valid_title, TicketTitle},
    };

    pub fn valid_draft_ticket() -> DraftTicket {
        DraftTicket {
            title: valid_title(),
            description: valid_description(),
        }
    }

    pub fn valid_draft_ticket_with(title: &str, description: &str) -> DraftTicket {
        let title = TicketTitle::try_from(title).unwrap_or(valid_title());
        let description = TicketDescription::try_from(description).unwrap_or(valid_description());
        DraftTicket { title, description }
    }
}
