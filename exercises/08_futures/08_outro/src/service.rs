use std::future::Future;

use crate::{
    ticket::{DraftTicket, PatchTicket, TicketId, TicketVersion, VersionedTicket},
    ticket_store::{ForTicketStorage, TicketInsertError, TicketUpdateError},
};

#[derive(Debug, thiserror::Error)]
pub enum TicketSystemError {
    #[error(transparent)]
    InsertError(#[from] TicketInsertError),
    #[error(transparent)]
    UpdateError(#[from] TicketUpdateError),
}

pub struct TicketSystemService<TS: ForTicketStorage> {
    ticket_storage: TS,
}

impl<TS: ForTicketStorage> TicketSystemService<TS> {
    pub fn new(for_ticket_storage: TS) -> Self {
        TicketSystemService {
            ticket_storage: for_ticket_storage,
        }
    }
}

pub trait ForAddTicket: Send + Sync {
    fn add(
        &self,
        draft_ticket: DraftTicket,
    ) -> impl Future<Output = Result<TicketId, TicketSystemError>> + Send;
}

pub trait ForGetTicket: Send + Sync {
    fn get(&self, id: &TicketId) -> impl Future<Output = Option<VersionedTicket>> + Send;
}

pub trait ForPatchTicket: Send + Sync {
    fn patch(
        &self,
        id: &TicketId,
        version: &TicketVersion,
        patch: PatchTicket,
    ) -> impl Future<Output = Result<VersionedTicket, TicketSystemError>> + Send;
}

pub trait ForTicketMgr: ForAddTicket + ForGetTicket + ForPatchTicket + Send + Sync {}

impl<TS: ForTicketStorage> ForAddTicket for TicketSystemService<TS> {
    async fn add(&self, draft_ticket: DraftTicket) -> Result<TicketId, TicketSystemError> {
        self.ticket_storage
            .add_ticket(draft_ticket)
            .await
            .map_err(TicketSystemError::InsertError)
    }
}

impl<TS: ForTicketStorage> ForGetTicket for TicketSystemService<TS> {
    fn get(&self, id: &TicketId) -> impl Future<Output = Option<VersionedTicket>> + Send {
        self.ticket_storage.get(id)
    }
}

impl<TS: ForTicketStorage> ForPatchTicket for TicketSystemService<TS> {
    async fn patch(
        &self,
        id: &TicketId,
        version: &TicketVersion,
        patch: PatchTicket,
    ) -> Result<VersionedTicket, TicketSystemError> {
        self.ticket_storage
            .patch(id, version, patch)
            .await
            .map_err(TicketSystemError::UpdateError)
    }
}

impl<TS: ForTicketStorage> ForTicketMgr for TicketSystemService<TS> {}

#[cfg(test)]
pub mod tests {
    use crate::{
        description::TicketDescription,
        service::{
            ForAddTicket, ForGetTicket, ForPatchTicket, ForTicketMgr, TicketSystemError,
            TicketSystemService,
        },
        status::Status,
        ticket::{
            DraftTicket, PatchTicket, TicketVersionMaxReachedError, TicketVersionMismatchError,
        },
        ticket_store::{
            tests::{
                create_simple_storage, create_simple_storage_with_counter,
                create_simple_storage_with_ticket_0_with_specified_version,
            },
            TicketInsertError, TicketUpdateError,
        },
        title::TicketTitle,
    };

    #[tokio::test]
    async fn should_insert_the_requested_ticket() {
        let draft_ticket = DraftTicket {
            title: "my-title".try_into().unwrap(),
            description: "my-description".try_into().unwrap(),
        };
        let service = build_service();
        let ticket_id = service.add(draft_ticket).await.unwrap();
        assert_eq!(ticket_id, 0.into());
    }

    #[tokio::test]
    async fn should_return_specific_error_when_insert_in_exhausted_storage() {
        let draft_ticket = DraftTicket {
            title: "my-title".try_into().unwrap(),
            description: "my-description".try_into().unwrap(),
        };
        let service = build_exhausted_service();
        let err = service.add(draft_ticket).await.unwrap_err();
        assert_eq!(
            err.to_string(),
            TicketSystemError::InsertError(TicketInsertError::StorageFull).to_string()
        );
    }

    #[tokio::test]
    async fn should_get_the_requested_ticket() {
        let expected_title: TicketTitle = "my-title".try_into().unwrap();
        let expected_description: TicketDescription = "my-description".try_into().unwrap();
        let draft_ticket = DraftTicket {
            title: expected_title.clone(),
            description: expected_description.clone(),
        };
        let service = build_service();
        let ticket_id = service.add(draft_ticket).await.unwrap();

        let versioned_ticket = service.get(&ticket_id).await.unwrap();
        assert_eq!(versioned_ticket.id(), ticket_id);
        assert_eq!(versioned_ticket.title(), expected_title);
        assert_eq!(versioned_ticket.description(), expected_description);
        assert_eq!(versioned_ticket.status(), Status::ToDo);
        assert_eq!(versioned_ticket.version(), 0.into());
    }

    #[tokio::test]
    async fn should_return_none_when_req_ticket_not_exist() {
        let service = build_service();
        let versioned_ticket = service.get(&100.into()).await;
        assert!(versioned_ticket.is_none());
    }

    #[tokio::test]
    async fn should_patch_the_requested_ticket_version() {
        let first_title: TicketTitle = "my-title".try_into().unwrap();
        let expected_title: TicketTitle = "modified_title".try_into().unwrap();
        let expected_description: TicketDescription = "my-description".try_into().unwrap();
        let draft_ticket = DraftTicket {
            title: first_title.clone(),
            description: expected_description.clone(),
        };
        let service = build_service();
        let ticket_id = service.add(draft_ticket).await.unwrap();

        let versioned_ticket = service.get(&ticket_id).await.unwrap();
        let versioned_ticket = service
            .patch(
                &versioned_ticket.id(),
                &versioned_ticket.version(),
                PatchTicket {
                    title: Some("modified_title".try_into().unwrap()),
                    ..PatchTicket::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(versioned_ticket.id(), ticket_id);
        assert_eq!(versioned_ticket.title(), expected_title);
        assert_eq!(versioned_ticket.description(), expected_description);
        assert_eq!(versioned_ticket.status(), Status::ToDo);
        assert_eq!(versioned_ticket.version(), 1.into());
    }

    #[tokio::test]
    async fn should_return_conflict_when_patching_a_wrong_version() {
        let first_title: TicketTitle = "my-title".try_into().unwrap();
        let expected_description: TicketDescription = "my-description".try_into().unwrap();
        let draft_ticket = DraftTicket {
            title: first_title.clone(),
            description: expected_description.clone(),
        };
        let service = build_service();
        let ticket_id = service.add(draft_ticket).await.unwrap();

        let versioned_ticket = service.get(&ticket_id).await.unwrap();
        let Err(err) = service
            .patch(
                &versioned_ticket.id(),
                &20.into(),
                PatchTicket {
                    title: Some("modified_title".try_into().unwrap()),
                    ..PatchTicket::default()
                },
            )
            .await
        else {
            panic!("");
        };

        assert_eq!(
            err.to_string(),
            TicketSystemError::UpdateError(TicketUpdateError::Conflict(TicketVersionMismatchError))
                .to_string()
        );
    }

    #[tokio::test]
    async fn should_return_error_when_patching_an_exhausted_version() {
        let service = build_service_that_return_ticket_0_with_specified_version(u64::MAX);
        let versioned_ticket = service.get(&0.into()).await.unwrap();
        let Err(err) = service
            .patch(
                &versioned_ticket.id(),
                &versioned_ticket.version(),
                PatchTicket {
                    title: Some("modified_title".try_into().unwrap()),
                    ..PatchTicket::default()
                },
            )
            .await
        else {
            panic!("Shold have reported an err");
        };

        assert_eq!(
            err.to_string(),
            TicketSystemError::UpdateError(TicketUpdateError::VersionExhausted(
                TicketVersionMaxReachedError
            ))
            .to_string()
        );
    }

    pub fn build_service() -> impl ForTicketMgr {
        TicketSystemService::new(create_simple_storage())
    }

    pub fn build_exhausted_service() -> impl ForTicketMgr {
        TicketSystemService::new(create_simple_storage_with_counter(u64::MAX))
    }

    pub fn build_service_that_return_ticket_0_with_specified_version(
        version: u64,
    ) -> impl ForTicketMgr {
        TicketSystemService::new(create_simple_storage_with_ticket_0_with_specified_version(
            version,
        ))
    }
}
