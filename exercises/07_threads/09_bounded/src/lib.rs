// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use std::sync::mpsc::{Receiver, SyncSender};

pub mod data;
pub mod store;

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>,
    capacity: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("The store is overloaded")]
pub struct OverloadedError;

impl TicketStoreClient {
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, OverloadedError> {
        let (response_sender, response_receiver) = std::sync::mpsc::sync_channel(self.capacity);
        match self.sender.try_send(Command::Insert { draft, response_channel: response_sender }) {
            Ok(_) => Ok(response_receiver.recv().unwrap()),
            Err(_) => Err(OverloadedError),
        }
    }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, OverloadedError> {
        let (response_sender, response_receiver) = std::sync::mpsc::sync_channel(self.capacity);
        match self.sender.try_send(Command::Get { id, response_channel: response_sender }) {
            Ok(_) => Ok(response_receiver.recv().unwrap()),
            Err(_) => Err(OverloadedError),
        }
    }
}

pub fn launch(capacity: usize) -> TicketStoreClient {
    let (sender, receiver) = std::sync::mpsc::sync_channel(capacity);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient{sender, capacity}
}

enum Command {
    Insert {
        draft: TicketDraft,
        response_channel: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_channel: SyncSender<Option<Ticket>>,
    },
}

fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_channel,
            }) => {
                let id = store.add_ticket(draft);
                response_channel.try_send(id).unwrap_or_else(|err| println!("Response failed: {err}"))
            }
            Ok(Command::Get {
                id,
                response_channel,
            }) => {
                let ticket = store.get(id);
                response_channel.try_send(ticket.cloned()).unwrap_or_else(|err| println!("Response failed: {err}"))
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}
