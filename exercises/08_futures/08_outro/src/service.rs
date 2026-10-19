use std::future::Future;

use crate::{
    ticket::{DraftTicket, PatchTicket, TicketId, TicketVersion, VersionedTicket},
    ticket_store::{ForTicketStorage, TicketUpdateError},
};

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

pub trait ForTicketMgr: Send + Sync {
    fn add(&self, draft_ticket: &DraftTicket) -> impl Future<Output = TicketId> + Send;
    fn get(&self, id: &TicketId) -> impl Future<Output = Option<VersionedTicket>> + Send;
    fn patch(
        &self,
        id: &TicketId,
        version: &TicketVersion,
        patch: PatchTicket,
    ) -> impl Future<Output = Result<VersionedTicket, TicketUpdateError>> + Send;
}

impl<TS: ForTicketStorage> ForTicketMgr for TicketSystemService<TS> {
    fn add(&self, draft_ticket: &DraftTicket) -> impl Future<Output = TicketId> + Send {
        self.ticket_storage.add_ticket(draft_ticket)
    }

    fn get(&self, id: &TicketId) -> impl Future<Output = Option<VersionedTicket>> + Send {
        self.ticket_storage.get(id)
    }

    fn patch(
        &self,
        id: &TicketId,
        version: &TicketVersion,
        patch: PatchTicket,
    ) -> impl Future<Output = Result<VersionedTicket, TicketUpdateError>> + Send {
        self.ticket_storage.patch(id, version, patch)
    }
}
