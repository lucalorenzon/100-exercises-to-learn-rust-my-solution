use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::status::Status;
use crate::ticket::{DraftTicket, Ticket, TicketId};

#[derive(Clone)]
pub struct SimpleTicketStore {
    storage: Arc<RwLock<TicketStore>>,
}

impl SimpleTicketStore {
    pub fn new() -> Self {
        SimpleTicketStore {
            storage: Arc::new(RwLock::new(TicketStore::new())),
        }
    }

    pub async fn add_ticket(&mut self, ticket: &DraftTicket) -> TicketId {
        self.storage.write().await.add_ticket(ticket.clone())
    }

    pub async fn get(&self, id: &TicketId) -> Option<Ticket> {
        let ticket = {
            let storage = self.storage.read().await;
            storage.get(*id)?
        };

        let ticket = ticket.read().await.clone();
        Some(ticket)
    }

    pub async fn patch(&self, ticket_patch: &Ticket) -> Option<Ticket> {
        let ticket = {
            let storage = self.storage.read().await;
            storage.get(ticket_patch.id)?
        };

        let ticket = {
            let mut ticket = ticket.write().await;
            *ticket = ticket_patch.clone();
            ticket.clone()
        };
        Some(ticket)
    }
}

struct TicketStore {
    tickets: BTreeMap<TicketId, Arc<RwLock<Ticket>>>,
    counter: u64,
}

impl TicketStore {
    fn new() -> Self {
        Self {
            tickets: BTreeMap::new(),
            counter: 0,
        }
    }

    fn add_ticket(&mut self, ticket: DraftTicket) -> TicketId {
        let id = TicketId::from(self.counter);
        self.counter += 1;

        let ticket = Ticket {
            id,
            title: ticket.title,
            description: ticket.description,
            status: Status::ToDo,
        };
        self.tickets.insert(id, Arc::new(RwLock::new(ticket)));
        id
    }

    fn get(&self, id: TicketId) -> Option<Arc<RwLock<Ticket>>> {
        self.tickets.get(&id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        description::TicketDescription, status::Status, ticket::test::valid_draft_ticket_with,
        ticket_store::SimpleTicketStore, title::TicketTitle,
    };

    #[tokio::test]
    async fn should_retrive_the_stored_ticket() {
        let mut ticket_storage_under_test = SimpleTicketStore::new();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test.add_ticket(&expected_ticket).await;
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();
        assert_eq!(TicketTitle::try_from("title").unwrap(), actual_ticket.title);
        assert_eq!(
            TicketDescription::try_from("description").unwrap(),
            actual_ticket.description
        );
    }

    #[tokio::test]
    async fn when_added_the_stored_ticket_should_have_status_todo() {
        let mut ticket_storage_under_test = SimpleTicketStore::new();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test.add_ticket(&expected_ticket).await;
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();

        assert_eq!(Status::ToDo, actual_ticket.status);
    }

    #[tokio::test]
    async fn when_added_the_stored_ticket_should_have_the_unique_id_returned() {
        let mut ticket_storage_under_test = SimpleTicketStore::new();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test.add_ticket(&expected_ticket).await;
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();

        assert_eq!(ticket_id, actual_ticket.id);
    }

    #[tokio::test]
    async fn should_store_and_retrieve_multiple_tickets() {
        let mut ticket_storage_under_test = SimpleTicketStore::new();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test.add_ticket(&expected_ticket).await;
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();
        assert_eq!(TicketTitle::try_from("title").unwrap(), actual_ticket.title);
        assert_eq!(
            TicketDescription::try_from("description").unwrap(),
            actual_ticket.description
        );
    }
}
