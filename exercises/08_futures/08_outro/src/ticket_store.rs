use std::collections::BTreeMap;
use std::future::Future;
use std::sync::Arc;

use serde::Serialize;
use tokio::sync::RwLock;

use crate::status::Status;
use crate::ticket::{
    DraftTicket, PatchTicket, Ticket, TicketId, TicketVersion, TicketVersionError,
    TicketVersionMaxReachedError, TicketVersionMismatchError, VersionedTicket,
};

#[derive(Clone)]
pub struct SimpleTicketStore {
    storage: Arc<RwLock<TicketStorage>>,
}

#[derive(Debug, thiserror::Error)]
pub enum TicketUpdateError {
    #[error(transparent)]
    Conflict(#[from] TicketVersionMismatchError),
    #[error(transparent)]
    VersionExausted(#[from] TicketVersionMaxReachedError),
    #[error("Ticket to update not found: {0}")]
    NotFound(TicketId),
}

pub trait ForTicketStorage: Send + Sync {
    fn add_ticket(
        &self,
        draft_ticket: &DraftTicket,
    ) -> impl Future<Output = Result<TicketId, TicketInsertError>> + Send;
    fn get(&self, id: &TicketId) -> impl Future<Output = Option<VersionedTicket>> + Send;
    fn patch(
        &self,
        id: &TicketId,
        version: &TicketVersion,
        patch: PatchTicket,
    ) -> impl Future<Output = Result<VersionedTicket, TicketUpdateError>> + Send;
}

impl SimpleTicketStore {
    pub fn new() -> Self {
        SimpleTicketStore {
            storage: Arc::new(RwLock::new(TicketStorage::new())),
        }
    }
}

impl Default for SimpleTicketStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ForTicketStorage for SimpleTicketStore {
    async fn add_ticket(&self, draft_ticket: &DraftTicket) -> Result<TicketId, TicketInsertError> {
        self.storage.write().await.add_ticket(draft_ticket.clone())
    }

    async fn get(&self, id: &TicketId) -> Option<VersionedTicket> {
        let stored_ticket = {
            let storage = self.storage.read().await;
            storage.get(id)?
        };

        let ticket = stored_ticket.read().await.clone();
        Some(ticket)
    }

    async fn patch(
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
        stored_ticket
            .try_patch(version, patch)
            .map_err(|e| match e {
                TicketVersionError::Mismatch(e) => TicketUpdateError::Conflict(e),
                TicketVersionError::MaxReached(e) => TicketUpdateError::VersionExausted(e),
            })?;
        Ok(stored_ticket.clone())
    }
}

struct TicketStorage {
    tickets: BTreeMap<TicketId, Arc<RwLock<VersionedTicket>>>,
    counter: u64,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum TicketInsertError {
    #[error("Max Storage capability reached")]
    StorageFull,
}

impl TicketStorage {
    fn new() -> Self {
        Self {
            tickets: BTreeMap::new(),
            counter: 0,
        }
    }

    fn add_ticket(&mut self, ticket: DraftTicket) -> Result<TicketId, TicketInsertError> {
        let id = TicketId::from(self.counter);
        self.counter = self
            .counter
            .checked_add(1)
            .ok_or(TicketInsertError::StorageFull)?;

        let ticket = Ticket {
            id,
            title: ticket.title,
            description: ticket.description,
            status: Status::ToDo,
        };
        self.tickets
            .insert(id, Arc::new(RwLock::new(VersionedTicket::new(ticket))));
        Ok(id)
    }

    fn get(&self, id: &TicketId) -> Option<Arc<RwLock<VersionedTicket>>> {
        self.tickets.get(id).cloned()
    }
}

#[cfg(test)]
pub mod tests {
    use std::{collections::BTreeMap, sync::Arc};

    use tokio::sync::RwLock;

    use crate::{
        description::TicketDescription,
        status::Status,
        ticket::{
            test::{valid_draft_ticket, valid_draft_ticket_with, valid_versioned_ticket_with},
            PatchTicket,
        },
        ticket_store::{ForTicketStorage, SimpleTicketStore, TicketInsertError, TicketStorage},
        title::TicketTitle,
    };

    #[tokio::test]
    async fn should_retrive_the_stored_ticket() {
        let ticket_storage_under_test = create_simple_storage();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test
            .add_ticket(&expected_ticket)
            .await
            .unwrap();
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
        let ticket_storage_under_test = create_simple_storage();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test
            .add_ticket(&expected_ticket)
            .await
            .unwrap();
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();

        assert_eq!(Status::ToDo, actual_ticket.status());
    }

    #[tokio::test]
    async fn when_added_the_stored_ticket_should_have_the_unique_id_returned() {
        let ticket_storage_under_test = create_simple_storage();
        let expected_ticket = valid_draft_ticket_with("title", "description");
        let ticket_id = ticket_storage_under_test
            .add_ticket(&expected_ticket)
            .await
            .unwrap();
        let maybe_actual_ticket = ticket_storage_under_test.get(&ticket_id).await;

        assert!(!maybe_actual_ticket.is_none());
        let actual_ticket = maybe_actual_ticket.unwrap();

        assert_eq!(ticket_id, actual_ticket.id());
    }

    #[tokio::test]
    async fn should_store_and_retrieve_multiple_tickets() {
        let ticket_storage_under_test = create_simple_storage();
        let expected_ticket1 = valid_draft_ticket_with("title1", "description1");
        let expected_ticket2 = valid_draft_ticket_with("title2", "description2");

        let ticket_id1 = ticket_storage_under_test
            .add_ticket(&expected_ticket1)
            .await
            .unwrap();
        let ticket_id2 = ticket_storage_under_test
            .add_ticket(&expected_ticket2)
            .await
            .unwrap();
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
    async fn should_return_error_if_max_storage_capacity_is_reached() {
        let ticket_storage_under_test = create_simple_storage_with_counter(u64::MAX);
        let draft_ticket = valid_draft_ticket();
        let err = ticket_storage_under_test
            .add_ticket(&draft_ticket)
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), TicketInsertError::StorageFull.to_string());
    }

    #[tokio::test]
    async fn should_patch_an_existing_ticket() {
        let ticket_storage_under_test = create_simple_storage();
        let draft_ticket =
            valid_draft_ticket_with("title to be modified", "description to be modified");
        let ticket_id = ticket_storage_under_test
            .add_ticket(&draft_ticket)
            .await
            .unwrap();
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

    pub fn create_simple_storage() -> SimpleTicketStore {
        SimpleTicketStore::new()
    }

    pub fn create_simple_storage_with_counter(count: u64) -> impl ForTicketStorage {
        SimpleTicketStore {
            storage: Arc::new(RwLock::new(TicketStorage {
                tickets: BTreeMap::new(),
                counter: count,
            })),
        }
    }

    pub fn create_simple_storage_with_ticket_0_with_specified_version(
        version: u64,
    ) -> impl ForTicketStorage {
        let special_ticket = Arc::new(RwLock::new(valid_versioned_ticket_with(
            0,
            "my-titcle",
            "my-description",
            Status::ToDo,
            version,
        )));
        let mut tickets = BTreeMap::new();
        tickets.insert(0.into(), special_ticket);
        SimpleTicketStore {
            storage: Arc::new(RwLock::new(TicketStorage {
                tickets: tickets,
                counter: 1,
            })),
        }
    }
}
