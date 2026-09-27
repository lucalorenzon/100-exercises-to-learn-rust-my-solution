// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use std::sync::mpsc::{sync_channel, Receiver, RecvError, SyncSender, TrySendError};

pub mod data;
pub mod store;

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>,
}

impl TicketStoreClient {
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, TicketStoreClientError> {
        let (response_channel, response_receiver) = sync_channel::<TicketId>(1);
        self.sender.try_send(Command::Insert {
            draft,
            response_channel,
        })?;
        Ok(response_receiver.recv()?)
    }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, TicketStoreClientError> {
        let (response_channel, response_receiver) = sync_channel::<Option<Ticket>>(1);
        self.sender.try_send(Command::Get {
            id,
            response_channel,
        })?;
        Ok(response_receiver.recv()?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TicketStoreClientError {
    #[error("ErrorSendingCommand: {0}")]
    CommandSendError(#[from] TrySendError<Command>),
    #[error("ErrorReceivingResponse: {0}")]
    ResponseReceiverError(#[from] RecvError),
}

pub fn launch(capacity: usize) -> TicketStoreClient {
    let (sender, receiver) = sync_channel::<Command>(capacity);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient { sender }
}

pub enum Command {
    Insert {
        draft: TicketDraft,
        response_channel: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_channel: SyncSender<Option<Ticket>>,
    },
}

pub fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_channel,
            }) => {
                let id = store.add_ticket(draft);
                let _ = response_channel.send(id);
            }
            Ok(Command::Get {
                id,
                response_channel,
            }) => {
                let ticket = store.get(id);
                let _ = response_channel.send(ticket.cloned());
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}
