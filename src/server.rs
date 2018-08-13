use std::path;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};

use chrono;
use timer;

use actix::*;
use rusqlite;
use uuid::Uuid;

use serde_json;
use molecule::Molecule;

use DATASETS;

#[cfg(feature = "server")]
const GARBAGE_COLLECTION_TIMEOUT: i64 = 60;//[s]; a dataset inactivity timeout

#[cfg(not(feature = "server"))]
const GARBAGE_COLLECTION_TIMEOUT: i64 = 10;//[s]; a dataset inactivity timeout

/// the server sends this messages to session
#[derive(Message)]
pub struct Message(pub String);

/// New session is created
#[derive(Message)]
#[rtype(String)]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub dataset_id: String,
    pub id: Uuid,
}

/// Session is disconnected
#[derive(Message)]
pub struct Disconnect {
    pub dataset_id: String,
    pub id: Uuid,    
}

//remove a dataset
#[derive(Message)]
pub struct Remove {
    pub dataset_id: String,        
}

/// broadcast a message to a dataset
#[derive(Message)]
pub struct WsMessage {    
    /// a WebSocket text message
    pub msg: String,
    /// dataset
    pub dataset_id: String,
}

/// broadcast a message to a dataset
#[derive(Message)]
pub struct FrequencyRangeMessage {    
    /// frequency range    
    pub freq_range: (f64, f64),    
    /// dataset
    pub dataset_id: String,
}

/// New session is created
#[derive(Message)]
#[rtype(String)]
pub struct GetMolecules {    
    pub dataset_id: String,
}

/// `SessionServer` manages sending messages from the FITSWebQL host server to WebSocket clients
pub struct SessionServer {
    sessions: HashMap<Uuid, Recipient<Message>>,
    datasets: Arc<RwLock<HashMap<String, HashSet<Uuid>>>>,
    molecules: Arc<RwLock<HashMap<String, String>>>,
    //splatalogue db
    conn_res: rusqlite::Result<rusqlite::Connection>,
    timer: timer::Timer,
    //guard: timer::Guard,
}

impl Default for SessionServer {
    fn default() -> SessionServer {
        //let mut datasets = HashMap::new();
        //datasets.insert("all".to_owned(), HashSet::new());//for now group all connections together; in the future will be grouped by dataset_id

        let timer = timer::Timer::new();
        //let guard = timer.schedule_with_delay(chrono::Duration::seconds(0), move || { });

        SessionServer {
            sessions: HashMap::new(),
            datasets: Arc::new(RwLock::new(HashMap::new())),
            molecules: Arc::new(RwLock::new(HashMap::new())),
            conn_res: rusqlite::Connection::open(path::Path::new("splatalogue_v3.db")),
            timer: timer,
            //guard: guard,
        }
    }
}

/// Make actor from `SessionServer`
impl Actor for SessionServer {    
    type Context = Context<Self>;
}

/// Handler for Connect message.
/// Register new session and assign unique id to this session
impl Handler<Connect> for SessionServer {
    type Result = String;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {        
        // register a new session
        let id = msg.id;        
        self.sessions.insert(id, msg.addr);

        println!("[SessionServer]: registering a new session {}/{}", msg.dataset_id, id); 

        self.datasets.write().entry(msg.dataset_id).or_insert(HashSet::new()).insert(id);

        // return the session id
        id.to_string()
    }
}

/// remove (unregister) a given session
impl Handler<Disconnect> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        //let id = Uuid::parse_str(&msg.id).unwrap();
        let id = msg.id;

        if self.sessions.remove(&id).is_some() {
            println!("[SessionServer]: removing a session {}/{}", &msg.dataset_id, id);
          
            let remove_entry = {
                match self.datasets.write().get_mut(&msg.dataset_id) {
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

                self.datasets.write().remove(&msg.dataset_id);

                //do not remove the molecules here, schedule a garbage collection call instead

                let datasets = self.datasets.clone();
                let molecules = self.molecules.clone();

                self.timer.schedule_with_delay(chrono::Duration::seconds(GARBAGE_COLLECTION_TIMEOUT), move || {                    
                    // This closure is executed on the scheduler thread
                    println!("executing garbage collection for {}", &msg.dataset_id);

                    //check if there are no new active sessions
                    match datasets.read().get(&msg.dataset_id) {                        
                        Some(_) => {                            
                            println!("[garbage collection]: an active session has been found for {}, doing nothing", &msg.dataset_id);
                        },
                        None => {
                            println!("[garbage collection]: no active sessions found, {} will be expunged from memory", &msg.dataset_id);

                            molecules.write().remove(&msg.dataset_id);
                            DATASETS.write().remove(&msg.dataset_id);
                        }
                    };
                }).ignore();
            }
        }     
    }
}

/// try to remove a given dataset
impl Handler<Remove> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: Remove, _: &mut Context<Self>) {        
        println!("[SessionServer]: received a Remove request for '{}'", &msg.dataset_id);        
    }
}

/// Handler for WsMessage message.
impl Handler<WsMessage> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, _: &mut Context<Self>) {
        //println!("[SessionServer]: received a WsMessage '{}' bound for '{}'", &msg.msg, &msg.dataset_id);

        match self.datasets.read().get(&msg.dataset_id) {
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

/// Handler for GetMolecules message.
impl Handler<GetMolecules> for SessionServer {
    type Result = String;

    fn handle(&mut self, msg: GetMolecules, _: &mut Context<Self>) -> Self::Result {        
        match self.molecules.read().get(&msg.dataset_id) {            
            Some(contents) => contents.clone(),
            None => String::from("")
        }
    }
}

/// Handler for FrequencyRange message.
impl Handler<FrequencyRangeMessage> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: FrequencyRangeMessage, _: &mut Context<Self>) {
        println!("[SessionServer]: received a frequency range {:?} GHz for '{}'", &msg.freq_range, &msg.dataset_id);        

        let (freq_start, freq_end) = msg.freq_range;

        if freq_start == 0.0 || freq_end == 0.0 {
            //insert an empty JSON array into self.molecules
            self.molecules.write().insert(msg.dataset_id, String::from("[]"));
            return;
        }

        let mut molecules : Vec<serde_json::Value> = Vec::new();

        match self.conn_res {
            Ok(ref splat_db) => {
                println!("[SessionServer] splatalogue sqlite connection Ok");                

                match splat_db.prepare(&format!("SELECT * FROM lines WHERE frequency>={} AND frequency<={};", freq_start, freq_end)) {
                    Ok(mut stmt) => {
                        let molecule_iter = stmt.query_map(&[], |row| {
                            Molecule::from_sqlite_row(row)                            
                        }).unwrap();

                        for molecule in molecule_iter {
                            //println!("molecule {:?}", molecule.unwrap());
                            let mol = molecule.unwrap();                            
                            molecules.push(mol.to_json());
                        }
                    },
                    Err(err) => println!("sqlite prepare error: {}", err)
                } 
            },
            Err(ref err) => println!("[SessionServer] error connecting to splatalogue sqlite: {}", err)
        }

        let mut contents = String::from("[");

        for entry in &molecules {
            contents.push_str(&entry.to_string()) ;
            contents.push(',');
        };

        if !molecules.is_empty() {
            contents.pop() ;
        }   

        contents.push(']');

        //println!("{}", contents);
        self.molecules.write().insert(msg.dataset_id, contents);
    }
}