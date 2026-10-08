use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::status::Status;
use crate::ticket::{
    DraftTicket, PatchTicket, Ticket, TicketId, TicketVersion, TicketVersionMismatchError,
    VersionedTicket,
};

#[derive(Clone)]
pub struct SimpleTicketStore {
    storage: Arc<RwLock<TicketStorage>>,
}

#[derive(Debug, thiserror::Error)]
pub enum TicketUpdateError {
    #[error("Version mismatch on ticket update")]
    Conflict(#[from] TicketVersionMismatchError),
    #[error("Ticket to update not found: {0}")]
    NotFound(TicketId),
}

impl SimpleTicketStore {
    pub fn new() -> Self {
        SimpleTicketStore {
            storage: Arc::new(RwLock::new(TicketStorage::new())),
        }
    }

    pub async fn add_ticket(&self, draft_ticket: &DraftTicket) -> TicketId {
        self.storage.write().await.add_ticket(draft_ticket.clone())
    }

    pub async fn get(&self, id: &TicketId) -> Option<VersionedTicket> {
        let stored_ticket = {
            let storage = self.storage.read().await;
            storage.get(id)?
        };

        let ticket = stored_ticket.read().await.clone();
        Some(ticket)
    }

    pub async fn patch(
        &self,
        id: &TicketId,
        version: &TicketVersion,
        patch: PatchTicket,
    ) -> Result<VersionedTicket, TicketUpdateError> {
        let stored_ticket = {
            let storage = self.storage.read().await;
            storage.get(id).ok_or(TicketUpdateError::NotFound(*id))?
        };

        let mut stored_ticket = stored_ticket.write().await;
        stored_ticket.try_patch(version, patch)?;
        Ok(stored_ticket.clone())
    }
}

struct TicketStorage {
    tickets: BTreeMap<TicketId, Arc<RwLock<VersionedTicket>>>,
    counter: u64,
}

impl TicketStorage {
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
        self.tickets
            .insert(id, Arc::new(RwLock::new(VersionedTicket::new(ticket))));
        id
    }

    fn get(&self, id: &TicketId) -> Option<Arc<RwLock<VersionedTicket>>> {
        self.tickets.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        description::TicketDescription,
        status::Status,
        ticket::{test::valid_draft_ticket_with, PatchTicket},
        ticket_store::SimpleTicketStore,
        title::TicketTitle,
    };

    #[tokio::test]
    async fn should_retrive_the_stored_ticket() {
        let ticket_storage_under_test = SimpleTicketStore::new();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test.add_ticket(&expected_ticket).await;
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();
        assert_eq!(
            TicketTitle::try_from("title").unwrap(),
            actual_ticket.title()
        );
        assert_eq!(
            TicketDescription::try_from("description").unwrap(),
            actual_ticket.description()
        );
    }

    #[tokio::test]
    async fn when_added_the_stored_ticket_should_have_status_todo() {
        let ticket_storage_under_test = SimpleTicketStore::new();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test.add_ticket(&expected_ticket).await;
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();

        assert_eq!(Status::ToDo, actual_ticket.status());
    }

    #[tokio::test]
    async fn when_added_the_stored_ticket_should_have_the_unique_id_returned() {
        let ticket_storage_under_test = SimpleTicketStore::new();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test.add_ticket(&expected_ticket).await;
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();

        assert_eq!(ticket_id, actual_ticket.id());
    }

    #[tokio::test]
    async fn should_store_and_retrieve_multiple_tickets() {
        let ticket_storage_under_test = SimpleTicketStore::new();
        let expected_ticket1 = valid_draft_ticket_with("title1", "description1");
        let expected_ticket2 = valid_draft_ticket_with("title2", "description2");

        let ticket_id1 = ticket_storage_under_test
            .add_ticket(&expected_ticket1)
            .await;
        let ticket_id2 = ticket_storage_under_test
            .add_ticket(&expected_ticket2)
            .await;
        let actual_ticket1 = ticket_storage_under_test.get(&ticket_id1).await.unwrap();
        let actual_ticket2 = ticket_storage_under_test.get(&ticket_id2).await.unwrap();

        assert_ne!(actual_ticket1.id(), actual_ticket2.id());

        assert_eq!(
            TicketTitle::try_from("title1").unwrap(),
            actual_ticket1.title()
        );
        assert_eq!(
            TicketDescription::try_from("description1").unwrap(),
            actual_ticket1.description()
        );

        assert_eq!(
            TicketTitle::try_from("title2").unwrap(),
            actual_ticket2.title()
        );
        assert_eq!(
            TicketDescription::try_from("description2").unwrap(),
            actual_ticket2.description()
        );
    }

    #[tokio::test]
    async fn should_patch_an_existing_ticket() {
        let ticket_storage_under_test = SimpleTicketStore::new();
        let draft_ticket =
            valid_draft_ticket_with("title to be modified", "description to be modified");
        let ticket_id = ticket_storage_under_test.add_ticket(&draft_ticket).await;
        let ticket_to_be_modified = ticket_storage_under_test.get(&ticket_id).await.unwrap();
        let patch = PatchTicket {
            status: Some(Status::InProgress),
            ..PatchTicket::default()
        };
        let modified_ticket = ticket_storage_under_test
            .patch(&ticket_id, &ticket_to_be_modified.version(), patch)
            .await
            .unwrap();
        assert_eq!(Status::InProgress, modified_ticket.status());
    }
}
