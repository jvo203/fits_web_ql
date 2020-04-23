use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono;
use std;
use std::time::Duration;
use std::time::SystemTime;
use timer;

use actix::*;
#[cfg(target_os = "linux")]
use thread_priority::*;

use uuid::Uuid;

use crate::DATASETS;

#[cfg(feature = "cluster")]
use crate::CLUSTER_JOBS;

#[cfg(feature = "jvo")]
const GARBAGE_COLLECTION_TIMEOUT: i64 = 60 * 60; //[s]; a dataset inactivity timeout//was 60

#[cfg(not(feature = "jvo"))]
const GARBAGE_COLLECTION_TIMEOUT: i64 = 10; //[s]; a dataset inactivity timeout

const ORPHAN_GARBAGE_COLLECTION_TIMEOUT: i64 = 60 * 60; //[s]; a dataset inactivity timeout//was 60

const DUMMY_DATASET_TIMEOUT: u64 = 24 * 60 * 60; //[s]; 24 hours, plenty of time for a local jvox download to complete (or fail)

/// the server sends these messages to session
/// New session is created
#[derive(Message)]
#[rtype(String)]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub dataset_id: String,
    pub id: Uuid,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(String)]
pub struct Disconnect {
    pub dataset_id: String,
    pub id: Uuid,
}

//remove a dataset
#[derive(Message)]
#[rtype(String)]
pub struct Remove {
    pub dataset_id: String,
}

/// broadcast a message to a dataset
#[derive(Message)]
#[rtype(String)]
pub struct WsMessage {
    /// a WebSocket text message
    pub notification: String,
    pub total: i32,
    pub running: i32,
    pub elapsed: std::time::Duration,
    /// dataset
    pub dataset_id: String,
}

/// `SessionServer` manages sending messages from the FITSWebQL host server to WebSocket clients
pub struct SessionServer {
    sessions: HashMap<Uuid, Recipient<WsMessage>>,
    datasets: Arc<RwLock<HashMap<String, HashSet<Uuid>>>>,
    timer: timer::Timer,
    _guard: timer::Guard,
}

impl Default for SessionServer {
    fn default() -> SessionServer {
        //let mut datasets = HashMap::new();
        //datasets.insert("all".to_owned(), HashSet::new());//for now group all connections together; in the future will be grouped by dataset_id

        let datasets = Arc::new(RwLock::new(HashMap::new()));
        let datasets_copy = datasets.clone();

        let timer = timer::Timer::new();
        let guard = timer.schedule_repeating(chrono::Duration::seconds(ORPHAN_GARBAGE_COLLECTION_TIMEOUT), move || {
            //println!("cleaning orphaned datasets");

            let orphans: Vec<_> = {
                let tmp = DATASETS.read();

                tmp.iter().map(|(key, value)| {
                    let dataset = value.read();

                    let now = SystemTime::now();
                    let elapsed = now.duration_since(*dataset.timestamp.read());

                    let timeout = if dataset.is_dummy {
                        Duration::new(DUMMY_DATASET_TIMEOUT, 0)
                    } else {
                        Duration::new(GARBAGE_COLLECTION_TIMEOUT as u64, 0)
                    };

                    match elapsed {
                        Ok(elapsed) => {
                            println!("[orphaned dataset cleanup]: key: {}, elapsed time: {:?}", key, elapsed);

                            if elapsed > timeout {
                                println!("{} marked as a candidate for deletion", key);

                                //check if there are no new active sessions
                                match datasets_copy.read().get(key) {
                                    Some(_) => {
                                        println!("[orphaned dataset cleanup]: an active session has been found for {}, doing nothing", key);
                                        None
                                    },
                                    None => {
                                        println!("[orphaned dataset cleanup]: no active sessions found, {} will be expunged from memory", key);
                                        Some(key.clone())
                                    }
                                }
                            }
                            else {
                                None
                            }
                        },
                        Err(err) => {
                            println!("SystemTime::duration_since failed: {}", err);
                            None
                        }
                    }
                }).collect()
            };

            //println!("orphans: {:?}", orphans);

            for key in orphans {
                match key {
                    Some(key) => {
                        //println!("[orphaned dataset cleanup]: no active sessions found, {} will be expunged from memory", key);                    
                        #[cfg(feature = "cluster")]
                        CLUSTER_JOBS.write().remove(&key);

                        let entry = DATASETS.write().remove(&key);
                        match entry {
                            Some(value) => {
                                std::thread::spawn(move || {
                                    #[cfg(target_os = "linux")]
                                    {
                                        match set_current_thread_priority(ThreadPriority::Min) {
                                            Ok(_) => println!("successfully lowered priority for the dataset drop thread"),
                                            Err(err) => println!("error changing the thread priority: {:?}", err),
                                        }
                                    };

                                    let fits = value.read();
                                    println!("non-blocking drop for {}", fits.dataset_id);
                                    fits.drop_to_cache();
                                });

                                println!("resuming the actix server thread");
                            },
                            None => println!("{} not found in the DATASETS", &key),
                        };

                        println!("[orphaned dataset cleanup]: {} has been expunged from memory", key);
                    },
                    None => {},
                }
            }
        });

        SessionServer {
            sessions: HashMap::new(),
            datasets: datasets,
            timer: timer,
            _guard: guard,
        }
    }
}

/// Make actor from `SessionServer`
impl Actor for SessionServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(1 << 31);
    }
}

/// Handler for Connect message.
/// Register new session and assign unique id to this session
impl Handler<Connect> for SessionServer {
    type Result = String;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // register a new session
        let id = msg.id;
        self.sessions.insert(id, msg.addr);

        println!(
            "[SessionServer]: registering a new session {}/{}",
            msg.dataset_id, id
        );

        self.datasets
            .write()
            .entry(msg.dataset_id)
            .or_insert(HashSet::new())
            .insert(id);

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
            println!(
                "[SessionServer]: removing a session {}/{}",
                &msg.dataset_id, id
            );

            let remove_entry = {
                match self.datasets.write().get_mut(&msg.dataset_id) {
                    Some(dataset) => {
                        dataset.remove(&id);
                        dataset.is_empty()
                    }
                    None => {
                        println!("[SessionServer]: {} not found", &msg.dataset_id);
                        false
                    }
                }
            };

            if remove_entry {
                println!("[SessionServer]: unlinking a dataset {}", &msg.dataset_id);

                self.datasets.write().remove(&msg.dataset_id);
                let datasets = self.datasets.clone();

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

                            let is_dummy = {
                                let tmp = DATASETS.read();
                                let fits = tmp.get(&msg.dataset_id);

                                match fits {
                                    Some(lock) => {
                                        let fits = lock.read();
                                        fits.is_dummy
                                    },
                                    None => {
                                        println!("[garbage collection]: (warning) {} not found in a HashMap", &msg.dataset_id);
                                        return;
                                    }
                                }
                            };

                            //do not remove dummy datasets (loading progress etc)
                            //they will be cleaned in a separate garbage collection thread
                            if !is_dummy {
                                #[cfg(feature = "cluster")]
                                CLUSTER_JOBS.write().remove(&msg.dataset_id);

                                let entry = DATASETS.write().remove(&msg.dataset_id) ;
                                match entry {
                                    Some(value) => {
                                        std::thread::spawn(move || {
                                            #[cfg(target_os = "linux")]
                                            {
                                                match set_current_thread_priority(ThreadPriority::Min) {
                                                    Ok(_) => println!("successfully lowered priority for the dataset drop thread"),
                                                    Err(err) => println!("error changing the thread priority: {:?}", err),
                                                }
                                            };

                                            let fits = value.read();
                                            println!("non-blocking drop for {}", fits.dataset_id);
                                            fits.drop_to_cache();
                                        });

                                        println!("resuming the actix server thread");
                                    },
                                    None => println!("{} not found in the DATASETS", &msg.dataset_id),
                                };
                            }
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
        println!(
            "[SessionServer]: received a Remove request for '{}'",
            &msg.dataset_id
        );
    }
}

/// Handler for WsMessage message.
impl Handler<WsMessage> for SessionServer {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, _: &mut Context<Self>) {
        //println!("[SessionServer]: received a WsMessage '{}' bound for '{}'", &msg.msg, &msg.dataset_id);

        match self.datasets.read().get(&msg.dataset_id) {
            Some(dataset) => {
                //progress interval checking has been moved to the websocket actor
                //simply pass through all progress notifications
                for id in dataset {
                    if let Some(addr) = self.sessions.get(id) {
                        let _ = addr.do_send(WsMessage {
                            notification: msg.notification.clone(),
                            total: msg.total,
                            running: msg.running,
                            elapsed: msg.elapsed,
                            dataset_id: msg.dataset_id.clone(),
                        });
                    }
                }
            }
            None => {
                //println!("[SessionServer]: {} not found", &msg.dataset_id);
            }
        }
    }
}
