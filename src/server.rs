use std::collections::{HashMap, HashSet};

use actix::*;
use uuid::Uuid;

/// the server sends this messages to session
#[derive(Message)]
pub struct Message(pub String);

/// New session is created
#[derive(Message)]
#[rtype(String)]
pub struct Connect {
    pub addr: Recipient<Syn, Message>,
    pub dataset_id: String,
}

/// Session is disconnected
#[derive(Message)]
pub struct Disconnect {
    pub dataset_id: String,
    pub id: String,    
}

/// broadcast a message to a dataset
#[derive(Message)]
pub struct WsMessage {    
    /// a WebSocket text message
    pub msg: String,
    /// dataset
    pub dataset_id: String,
}

/// `SessionServer` manages sending messages from the FITSWebQL host server to WebSocket clients
pub struct SessionServer {
    sessions: HashMap<Uuid, Recipient<Syn, Message>>,
    datasets: HashMap<String, HashSet<Uuid>>,    
}

impl Default for SessionServer {
    fn default() -> SessionServer {
        //let mut datasets = HashMap::new();
        //datasets.insert("all".to_owned(), HashSet::new());//for now group all connections together; in the future will be grouped by dataset_id

        SessionServer {
            sessions: HashMap::new(),
            datasets: HashMap::new(),            
        }
    }
}

/// Make actor from `SessionServer`
impl Actor for SessionServer {    
    type Context = SyncContext<Self>;
}

/// Handler for Connect message.
/// Register new session and assign unique id to this session
impl Handler<Connect> for SessionServer {
    type Result = String;

    fn handle(&mut self, msg: Connect, _: &mut SyncContext<Self>) -> Self::Result {        
        // register a new session with a random uuid
        let id = Uuid::new_v4();        
        self.sessions.insert(id, msg.addr);

        println!("[SessionServer]: registering a new session {}/{}", msg.dataset_id, id); 

        self.datasets.entry(msg.dataset_id).or_insert(HashSet::new()).insert(id);       

        // return the session id
        id.to_string()
    }
}

/// remove (unregister) a given session
impl Handler<Disconnect> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut SyncContext<Self>) {
        let id = Uuid::parse_str(&msg.id).unwrap();

        if self.sessions.remove(&id).is_some() {
            println!("[SessionServer]: removing a session {}/{}", &msg.dataset_id, id);
          
            let remove_entry = {
                match self.datasets.get_mut(&msg.dataset_id) {
                    Some(dataset) => {                        
                        dataset.remove(&id);
                        dataset.is_empty()
                    },
                    None => {
                        println!("[SessionServer]: {} not found", &msg.dataset_id);
                        false
                    }
                }
            };

            if remove_entry {
                println!("[SessionServer]: unlinking a dataset {}", &msg.dataset_id);
                self.datasets.remove(&msg.dataset_id);
            }
        }     
    }
}

/// Handler for WsMessage message.
impl Handler<WsMessage> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, _: &mut SyncContext<Self>) {
        //println!("[SessionServer]: received a WsMessage '{}' bound for '{}'", &msg.msg, &msg.dataset_id);

        match self.datasets.get(&msg.dataset_id) {
            Some(dataset) => {
                for id in dataset {                   
                    if let Some(addr) = self.sessions.get(id) {
                        let _ = addr.do_send(Message(msg.msg.to_owned()));
                    }
                }
            },            
            None => {
                //println!("[SessionServer]: {} not found", &msg.dataset_id);                        
            }
        }
    }
}