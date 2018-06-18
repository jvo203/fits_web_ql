use std::collections::{HashMap, HashSet};

use actix::*;
use uuid::Uuid;

/// the server sends this messages to session
#[derive(Message)]
pub struct Message(pub String);

/// New session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Syn, Message>,
}

/// Session is disconnected
#[derive(Message)]
pub struct Disconnect {
    pub id: Uuid,
}

/// `WsServer` manages sending messages from the FITSWebQL host server to WebSocket clients
pub struct WsServer {
    sessions: HashMap<Uuid, Recipient<Syn, Message>>,
    datasets: HashMap<String, HashSet<Uuid>>,    
}

impl Default for WsServer {
    fn default() -> WsServer {
        let mut datasets = HashMap::new();
        datasets.insert("all".to_owned(), HashSet::new());//for now group all connections together; in the future will be grouped by dataset_id

        WsServer {
            sessions: HashMap::new(),
            datasets: datasets,            
        }
    }
}

/// Make actor from `WsServer`
impl Actor for WsServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}