extern crate actix;
extern crate actix_web;
extern crate percent_encoding;

extern crate byteorder;
extern crate chrono;
extern crate half;
extern crate uuid;
extern crate lz4_compress;
extern crate futures;

use std::sync::Arc;
use std::{thread,time};
use std::str::FromStr;
use std::env;
use std::time::{SystemTime};
use std::collections::BTreeMap;

use actix::*;
use actix_web::*;
use actix_web::server::HttpServer;
use futures::future::{Future,result};
use percent_encoding::percent_decode;
use uuid::Uuid;

#[macro_use]
extern crate scan_fmt;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_json;

use std::collections::HashMap;
use std::sync::RwLock;

static JVO_FITS_SERVER: &'static str = "jvox.vo.nao.ac.jp";

mod fits;
mod server;

struct WsSessionState {
    addr: Addr<Syn, server::SessionServer>,
}

#[derive(Debug)]
struct UserSession {    
    dataset_id: String,
    session_id: Uuid,
}

impl UserSession {
    pub fn new(id: &String) -> UserSession {
        let session = UserSession {
            dataset_id: id.clone(),
            //session_id: String::from(""),
            session_id: Uuid::new_v4(),
        } ;

        println!("allocating a new websocket session for {}", id);

        session
    }
}

impl Drop for UserSession {
    fn drop(&mut self) {
        println!("dropping a websocket session for {}", self.dataset_id);
    }
}

impl Actor for UserSession {
    type Context = ws::WebsocketContext<Self, WsSessionState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("websocket connection started for {}", self.dataset_id);

        let addr: Addr<Syn, _> = ctx.address();

        ctx.state()
            .addr
            .do_send(server::Connect {
                addr: addr.recipient(),
                dataset_id: self.dataset_id.clone(),
                id: self.session_id,
            });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        println!("stopping a websocket connection for {}/{}", self.dataset_id, self.session_id);

        ctx.state().addr.do_send(server::Disconnect {
            dataset_id: self.dataset_id.clone(),
            id: self.session_id.clone()
            });

        Running::Stop
    }     
}

/// forward progress messages from FITS loading to the websocket
impl Handler<server::Message> for UserSession {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        //println!("websocket sending '{}'", msg.0);
        ctx.text(msg.0);
    }
}

// Handler for ws::Message messages
impl StreamHandler<ws::Message, ws::ProtocolError> for UserSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        //println!("WEBSOCKET MESSAGE: {:?}", msg);

        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Text(text) => {                               
                if text.contains("[heartbeat]") {
                    ctx.text(text);
                }
            },                
            ws::Message::Binary(_) => println!("ignoring an incoming binary websocket message"),
            _ => ctx.stop(),
        }
    }
}

/*lazy_static! {
    static ref DATASETS: RwLock<HashMap<String, fits::FITS>> = {
        RwLock::new(HashMap::new())
    };
}*/

lazy_static! {
    static ref DATASETS: RwLock<HashMap<String, Arc<RwLock<Box<fits::FITS>>>>> = {
        RwLock::new(HashMap::new())
    };
}

#[cfg(not(feature = "server"))]
static SERVER_STRING: &'static str = "FITSWebQL v1.2.0";
const SERVER_PORT: i32 = 8080;

static VERSION_STRING: &'static str = "SV2018-06-22.1";

#[cfg(not(feature = "server"))]
static SERVER_MODE: &'static str = "LOCAL";

#[cfg(feature = "server")]
static SERVER_MODE: &'static str = "SERVER";

fn remove_symlinks() {
    let cache = std::path::Path::new(fits::FITSCACHE);

    for entry in cache.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            //remove a file if it's a symbolic link
            if let Ok(metadata) = entry.metadata() {
                let file_type = metadata.file_type();

                if file_type.is_symlink() {
                    println!("removing a symbolic link to {:?}", entry.file_name());
                    let _ = std::fs::remove_file(entry.path());
                }
            }     
        }
    }
}

fn get_home_directory() -> HttpResponse {
    match env::home_dir() {
        Some(path_buf) => get_directory(path_buf),
        None => HttpResponse::NotFound()
                    .content_type("text/html")
                    .body(format!("<p><b>Critical Error</b>: home directory not found</p>"))
    }
}

fn get_directory(path: std::path::PathBuf) -> HttpResponse {
    println!("scanning directory: {:?}", path) ;       
    
    let mut ordered_entries = BTreeMap::new();

    for entry in path.read_dir().expect("read_dir call failed") {

        if let Ok(entry) = entry {
            let file_name_buf = entry.file_name();
            let file_name = file_name_buf.to_str().unwrap();

            if file_name.starts_with(".") {
                continue ;
            }

            if let Ok(metadata) = entry.metadata() {
                //println!("{:?}:{:?} filesize: {}", entry.path(), metadata, metadata.len());

                if metadata.is_dir() {                    

                    let ts = match metadata.modified() {
                        Ok(x) => x,
                        Err(_) => std::time::UNIX_EPOCH
                    } ;

                    let std_duration = ts.duration_since(std::time::UNIX_EPOCH).unwrap() ;
                    let chrono_duration = ::chrono::Duration::from_std(std_duration).unwrap() ;
                    let unix = chrono::NaiveDateTime::from_timestamp(0, 0) ;
                    let naive = unix + chrono_duration ;

                    let dir_entry = json!({
                        "type" : "dir",
                        "name" : format!("{}", entry.file_name().into_string().unwrap()),
                        "last_modified" : format!("{}", naive.format("%c"))
                    });

                    println!("{}", dir_entry.to_string());
                    ordered_entries.insert(entry.file_name(), dir_entry);                    
                }

                //filter by .fits .FITS
                if metadata.is_file() {

                    let path = entry.path() ;
                    let ext = path.extension().and_then(std::ffi::OsStr::to_str) ; 
                    
                    if ext == Some("fits") || ext == Some("FITS") {                        
                        let ts = match metadata.modified() {
                            Ok(x) => x,
                            Err(_) => std::time::UNIX_EPOCH
                        } ;

                        let std_duration = ts.duration_since(std::time::UNIX_EPOCH).unwrap() ;
                        let chrono_duration = ::chrono::Duration::from_std(std_duration).unwrap() ;
                        let unix = chrono::NaiveDateTime::from_timestamp(0, 0) ;
                        let naive = unix + chrono_duration ;

                        let file_entry = json!({
                            "type" : "file",
                            "name" : format!("{}", entry.file_name().into_string().unwrap()),
                            "size" : metadata.len(),
                            "last_modified" : format!("{}", naive.format("%c"))
                        });

                        println!("{}", file_entry.to_string());
                        ordered_entries.insert(entry.file_name(), file_entry);                        
                    }
                }
            }
        }
    }     

    //println!("{:?}", ordered_entries);

    let mut contents = String::from("["); 

    for (_, entry) in &ordered_entries {
        contents.push_str(&entry.to_string()) ;
        contents.push(',');
    };

    if !ordered_entries.is_empty() {
        //remove the last comma
        contents.pop() ;
    }   

    contents.push(']');

    HttpResponse::Ok()
        .content_type("application/json")
        .body(format!("{{\"location\": \"{}\", \"contents\": {} }}", path.display(), contents))
}

fn directory_handler(req: HttpRequest<WsSessionState>) -> HttpResponse {
    let query = req.query();

    match query.get("dir") {
        Some(x) => get_directory(std::path::PathBuf::from(x)),
        None => get_home_directory()//default database
    }
}

// do websocket handshake and start an actor
/*fn websocket_entry(req: HttpRequest<WsSessionState>) -> Result<Box<Future<Item=HttpResponse, Error=Error>>, Error> {
    let dataset_id_orig: String = req.match_info().query("id").unwrap();

    let dataset_id = match percent_decode(dataset_id_orig.as_bytes()).decode_utf8() {
        Ok(x) => x.into_owned(),
        Err(_) => dataset_id_orig.clone(),
    };

    //dataset_id needs to be URI-decoded

    let session = UserSession::new(&dataset_id);

    Ok(Box::new(result(ws::start(req, session))))
}*/

fn websocket_entry(req: HttpRequest<WsSessionState>) -> Result<HttpResponse> {
    let dataset_id_orig: String = req.match_info().query("id").unwrap();

    let dataset_id = match percent_decode(dataset_id_orig.as_bytes()).decode_utf8() {
        Ok(x) => x.into_owned(),
        Err(_) => dataset_id_orig.clone(),
    };

    //dataset_id needs to be URI-decoded

    let session = UserSession::new(&dataset_id);

    ws::start(req, session)
}

fn fitswebql_entry(req: HttpRequest<WsSessionState>) -> HttpResponse {
    let fitswebql_path: String = req.match_info().query("path").unwrap();
    
    let state = req.state();
    let server = &state.addr;    

    let query = req.query();
    
    #[cfg(feature = "server")]
    let db = match query.get("db") {
        Some(x) => {x},
        None => {"alma"}//default database
    };

    #[cfg(feature = "server")]
    let table = match query.get("table") {
        Some(x) => {x},
        None => {"cube"}//default table
    };

    #[cfg(not(feature = "server"))]
    let dir = match query.get("dir") {
        Some(x) => {x},
        None => {"."}//by default use the current directory
    };

    #[cfg(not(feature = "server"))]
    let ext = match query.get("ext") {
        Some(x) => {x},
        None => {"fits"}//a default FITS file extension
    };

    #[cfg(not(feature = "server"))]
    let dataset = "filename" ;

    #[cfg(feature = "server")]
    let dataset = "datasetId" ;

    let dataset_id = match query.get(dataset) {
        Some(x) => {vec![x]},
        None => {
            //try to read multiple datasets or filename,
            //i.e. dataset1,dataset2,... or filename1,filename2,...
            let mut v: Vec<&str> = Vec::new();            
            let mut count: u32 = 1;

            loop {
                let pattern = format!("{}{}", dataset, count);
                count = count + 1;

                match query.get(&pattern) {
                    Some(x) => {v.push(x);},
                    None => {break;}
                } ;
            } ;

            //the last resort
            if v.is_empty() {            
                return HttpResponse::NotFound()
                    .content_type("text/html")
                    .body(format!("<p><b>Critical Error</b>: no {} available</p>", dataset))                    
                };
            
            v
        }
    };

    let composite = match query.get("composite") {
        Some(x) => { match bool::from_str(x) {
                        Ok(b) => {b},
                        Err(_) => {false}
                        }
                },
        None => {false}
    };

    #[cfg(feature = "server")]
    let resp = format!("FITSWebQL path: {}, db: {}, table: {}, dataset_id: {:?}, composite: {}", fitswebql_path, db, table, dataset_id, composite);

    #[cfg(not(feature = "server"))]
    let resp = format!("FITSWebQL path: {}, dir: {}, ext: {}, filename: {:?}, composite: {}", fitswebql_path, dir, ext, dataset_id, composite);

    println!("{}", resp);

    //server
    //execute_fits(&fitswebql_path, &db, &table, &dataset_id, composite)

    #[cfg(not(feature = "server"))]
    execute_fits(&fitswebql_path, &dir, &ext, &dataset_id, composite, &server)
}

fn get_spectrum(req: HttpRequest<WsSessionState>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    //println!("{:?}", req);

    let dataset_id = match req.query().get("datasetId") {
        Some(x) => {x},
        None => {            
            return result(Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: get_spectrum/datasetId parameter not found</p>"))))
                .responder()
        }
    };

    println!("[get_spectrum] http request for {}", dataset_id);

    result(Ok({
        let datasets = DATASETS.read().unwrap();

        println!("[get_spectrum] obtained read access to <DATASETS>, trying to get read access to {}", dataset_id);

        let fits = match datasets.get(dataset_id).unwrap().read() {
            Ok(x) => x,
            Err(err) => {
                println!("[get_spectrum] {}: cannot obtain a read access to {}", err, dataset_id);

                return result(Ok(HttpResponse::NotFound()
                    .content_type("text/html")
                    .body(format!("<p><b>Critical Error</b>: {} not found</p>", dataset_id))))
                    .responder();
            }
        };

        println!("[get_spectrum] obtained read access to {}, has_data = {}", dataset_id, fits.has_data);

        if fits.has_data {
            let resp = json!({
                //"HEADERSIZE" : fits.compressed_header.len()
            });

            HttpResponse::Ok()
                .content_type("application/json")
                .body(format!("{}",resp.to_string()))
        }
        else {
            HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: spectrum not found</p>"))
        }
    }))
    .responder()
}

fn get_molecules(req: HttpRequest<WsSessionState>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    //println!("{:?}", req);

    let dataset_id = match req.query().get("datasetId") {
        Some(x) => {x},
        None => {            
            return result(Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: get_molecules/datasetId parameter not found</p>"))))
                .responder();
        }
    };

    //freq_start
    let freq_start = match req.query().get("freq_start") {
        Some(x) => {x},
        None => {            
            return result(Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: get_molecules/freq_start parameter not found</p>"))))
                .responder();
        }
    };

    let freq_start = match freq_start.parse::<f32>() {        
        Ok(x) => x,
        Err(_) => 0.0
    };

    //freq_end
    let freq_end = match req.query().get("freq_end") {
        Some(x) => {x},
        None => {            
            return result(Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: get_molecules/freq_end parameter not found</p>"))))
                .responder();
        }
    };

    let freq_end = match freq_end.parse::<f32>() {        
        Ok(x) => x,
        Err(_) => 0.0
    };

    println!("[get_molecules] http request for {}: freq_start={}, freq_end={}", dataset_id, freq_start, freq_end);

    result(Ok({
        let datasets = DATASETS.read().unwrap();

        println!("[get_molecules] obtained read access to <DATASETS>, trying to get read access to {}", dataset_id);

        let fits = match datasets.get(dataset_id).unwrap().read() {
            Ok(x) => x,
            Err(err) => {
                println!("[get_molecules] {}: cannot obtain a read access to {}", err, dataset_id);

                return result(Ok(HttpResponse::NotFound()
                    .content_type("text/html")
                    .body(format!("<p><b>Critical Error</b>: {} not found</p>", dataset_id))))
                    .responder();
            }
        };

        println!("[get_molecules] obtained read access to {}, has_header = {}", dataset_id, fits.has_header);

        HttpResponse::NotFound()
            .content_type("text/html")
            .body(format!("<p><b>Critical Error</b>: spectral lines not found</p>"))
    }))
    .responder()
}

/*#[cfg(not(feature = "server"))]
fn execute_fits_old(fitswebql_path: &String, dir: &str, ext: &str, dataset_id: &Vec<&str>, composite: bool, server: &Addr<Syn, server::SessionServer>) -> HttpResponse {

    //get fits location    

    //launch FITS threads
    let mut has_fits: bool = true ;

    //for each dataset_id
    for i in 0..dataset_id.len() {
        let data_id = dataset_id[i];

        //does the entry exist in the datasets hash map?
        let has_entry = {
            let datasets = DATASETS.read().unwrap();
            datasets.contains_key(data_id) 
        } ;

        //if it does not exist set has_fits to false and load the FITS data
        if !has_entry {
            has_fits = false ;            

            let my_dir = dir.to_string();
            let my_data_id = data_id.to_string();
            let my_ext = ext.to_string();
            let my_server = server.clone();
                                    
            DATASETS.write().unwrap().insert(my_data_id.clone(), fits::FITS::new(&my_data_id));            
               
            //load FITS data in a new thread
            thread::spawn(move || {
                let filename = format!("{}/{}.{}", my_dir, my_data_id, my_ext);
                println!("loading FITS data from {}", filename); 

                let filepath = std::path::Path::new(&filename);           
                let fits = fits::FITS::from_path(&my_data_id.clone(), filepath, &my_server);

                DATASETS.write().unwrap().insert(my_data_id.clone(), fits);                           
            });
        }
        else {
            //update the timestamp
            let datasets = DATASETS.read().unwrap();
            let dataset = datasets.get(data_id).unwrap() ;
            
            has_fits = has_fits && dataset.has_data ;
            *dataset.timestamp.write().unwrap() = SystemTime::now() ;
        } ;
    } ;

    http_fits_response(&fitswebql_path, &dataset_id, composite, has_fits)
}*/

#[cfg(not(feature = "server"))]
fn execute_fits(fitswebql_path: &String, dir: &str, ext: &str, dataset_id: &Vec<&str>, composite: bool, server: &Addr<Syn, server::SessionServer>) -> HttpResponse {

    //get fits location    

    //launch FITS threads
    let mut has_fits: bool = true ;

    //for each dataset_id
    for i in 0..dataset_id.len() {
        let data_id = dataset_id[i];        
        
        let mut datasets = DATASETS.write().unwrap();                

        //if it does not exist set has_fits to false and load the FITS data
        if !datasets.contains_key(data_id) {
            has_fits = false ;            

            let my_dir = dir.to_string();
            let my_data_id = data_id.to_string();
            let my_ext = ext.to_string();
            let my_server = server.clone();                                                           
               
            datasets.insert(my_data_id.clone(), Arc::new(RwLock::new(Box::new(fits::FITS::new(&my_data_id))))); 

            //load FITS data in a new thread
            thread::spawn(move || {
                let filename = format!("{}/{}.{}", my_dir, my_data_id, my_ext);
                println!("loading FITS data from {}", filename);                 
                
                let datasets = DATASETS.read().unwrap();

                let mut fits = match datasets.get(&my_data_id).unwrap().write() {                    
                    Ok(x) => x,                        
                    Err(err) => {                        
                        println!("{}: cannot obtain a mutable reference to {}", err, my_data_id);
                        return;
                    }                
                };                

                //println!("obtained a mutable reference to {}", my_data_id);

                let filepath = std::path::Path::new(&filename);
                fits.load_from_path(&my_data_id.clone(), filepath, &my_server);            
            });
        }
        else {
            //update the timestamp            
            let dataset = datasets.get(data_id).unwrap().read().unwrap() ;
            
            has_fits = has_fits && dataset.has_data ;
            *dataset.timestamp.write().unwrap() = SystemTime::now() ;

            println!("updated an access timestamp for {}", data_id);
        } ;
    } ;

    http_fits_response(&fitswebql_path, &dataset_id, composite, has_fits)
}

fn http_fits_response(fitswebql_path: &String, dataset_id: &Vec<&str>, composite: bool, has_fits: bool) -> HttpResponse {

    //let has_fits: bool = false ;//later on it should be changed to true; iterate over all datasets, setting it to false if not found    

    //build up an HTML response
    let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<link href=\"https://fonts.googleapis.com/css?family=Inconsolata\" rel=\"stylesheet\"/>\n");
    html.push_str("<link href=\"https://fonts.googleapis.com/css?family=Lato\" rel=\"stylesheet\"/>\n");

    html.push_str("<script src=\"https://d3js.org/d3.v4.min.js\"></script>\n");
    html.push_str("<script src=\"reconnecting-websocket.js\"></script>\n");
    html.push_str("<script src=\"//cdnjs.cloudflare.com/ajax/libs/numeral.js/2.0.6/numeral.min.js\"></script>\n");

    html.push_str("<script src=\"ra_dec_conversion.js\"></script>\n");
    html.push_str("<script src=\"sylvester.js\"></script>\n");
    html.push_str("<script src=\"shortcut.js\"></script>\n");
    html.push_str("<script src=\"colourmaps.js\"></script>\n");
    html.push_str("<script src=\"lz4.min.js\"></script>\n");
    html.push_str("<script src=\"marchingsquares-isocontours.min.js\"></script>\n");
    html.push_str("<script src=\"marchingsquares-isobands.min.js\"></script>\n");    

    //bootstrap
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1, user-scalable=no, minimum-scale=1, maximum-scale=1\">\n");
    html.push_str("<link rel=\"stylesheet\" href=\"https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/css/bootstrap.min.css\">\n");
    html.push_str("<script src=\"https://ajax.googleapis.com/ajax/libs/jquery/3.1.1/jquery.min.js\"></script>\n");
    html.push_str("<script src=\"https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/js/bootstrap.min.js\"></script>\n");
    
    //FITSWebQL main JavaScript
    html.push_str(&format!("<script src=\"fitswebql.js?{}\"></script>\n", VERSION_STRING));
    //custom css styles
    html.push_str("<link rel=\"stylesheet\" href=\"fitswebql.css\"/>\n");

    html.push_str("<title>FITSWebQL</title></head><body>\n");
    html.push_str(&format!("<div id='votable' style='width: 0; height: 0;' data-va_count='{}' ", dataset_id.len()));

    if dataset_id.len() == 1 {
        html.push_str(&format!("data-datasetId='{}' ", dataset_id[0]));
    }
    else {
        for i in 0..dataset_id.len() {
            html.push_str(&format!("data-datasetId{}='{}' ", i+1, dataset_id[i]));
        }

        if composite && dataset_id.len() <= 3 {
            html.push_str("data-composite='1' ");
        }
    }

    html.push_str(&format!("data-root-path='/{}/' data-server-version='{}' data-server-string='{}' data-server-mode='{}' data-has-fits='{}'></div>\n", fitswebql_path, VERSION_STRING, SERVER_STRING, SERVER_MODE, has_fits));

    //the page entry point
    html.push_str("<script>
        const golden_ratio = 1.6180339887;
        var ALMAWS = null ;
        var firstTime = true ;
        var has_image = false ;         
        var PROGRESS_VARIABLE = 0.0 ;
        var PROGRESS_INFO = \"\" ;      
        var RESTFRQ = 0.0 ;
        var USER_SELFRQ = 0.0 ;
        var USER_DELTAV = 0.0 ;
        var ROOT_PATH = \"/fitswebql/\" ;
        mainRenderer();
        var idleResize = -1;
        window.onresize = resizeMe;
    </script>\n");

    //Google Analytics
    #[cfg(feature = "development")]
    html.push_str("<script>
  (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){ 
  (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o), 
  m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m) 
  })(window,document,'script','//www.google-analytics.com/analytics.js','ga');
  ga('create', 'UA-72136224-1', 'auto');				
  ga('send', 'pageview');						  									
  </script>\n");

    #[cfg(feature = "test")]
    html.push_str("<script>
  (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){ 
  (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o), 
  m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m) 
  })(window,document,'script','//www.google-analytics.com/analytics.js','ga');
  ga('create', 'UA-72136224-2', 'auto');				
  ga('send', 'pageview');  									
  </script>\n");

    #[cfg(feature = "production")]
    html.push_str("<script>
  (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){
  (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o),
  m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m)
  })(window,document,'script','//www.google-analytics.com/analytics.js','ga');
  ga('create', 'UA-72136224-4', 'auto');
  ga('send', 'pageview');
  </script>\n");

    #[cfg(not(feature = "server"))]
    html.push_str("<script>
      (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){
      (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o),
      m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m)
      })(window,document,'script','//www.google-analytics.com/analytics.js','ga');
      ga('create', 'UA-72136224-5', 'auto');
      ga('send', 'pageview');
      </script>\n");

    html.push_str("</body></html>\n");

    HttpResponse::Ok()
        .content_type("text/html")
        .body(html)
}

fn main() {
    remove_symlinks();

    #[cfg(not(feature = "server"))]
    let index_file = "fitswebql.html" ;

    #[cfg(feature = "server")]
    let index_file = "almawebql.html" ;    

    let sys = actix::System::new("fits_web_ql");

    // Start the WebSocket message server actor in a separate thread
    let server: Addr<Syn, _> = Arbiter::start(|_| server::SessionServer::default());    
    //let server: Addr<Syn, _> = SyncArbiter::start(32,|| server::SessionServer::default());//16 or 32 threads at most

    HttpServer::new(
        move || {            
            // WebSocket sessions state
            let state = WsSessionState {                
                addr: server.clone(),                
            };            
        
            App::with_state(state)            
                .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::GET).f(fitswebql_entry)})  
                .resource("/{path}/websocket/{id}", |r| {r.route().f(websocket_entry)})
                .resource("/get_directory", |r| {r.method(http::Method::GET).f(directory_handler)})
                .resource("/{path}/get_spectrum", |r| {r.method(http::Method::GET).f(get_spectrum)})
                .resource("/{path}/get_molecules", |r| {r.method(http::Method::GET).f(get_molecules)})
                .handler("/", fs::StaticFiles::new("htdocs").index_file(index_file))
        })
        .bind(&format!("localhost:{}", SERVER_PORT)).expect(&format!("Cannot bind to localhost:{}", SERVER_PORT))        
        .start();

    #[cfg(not(feature = "server"))]
    {
        println!("started a local FITSWebQL server; point your browser to http://localhost:{}", SERVER_PORT);
        println!("press CTRL+C to exit");
    }    

    #[cfg(feature = "server")]
    {
        println!("started a fits_web_ql server on port {}", SERVER_PORT);
        println!("send SIGINT to shutdown");
    }

    let _ = sys.run();

    DATASETS.write().unwrap().clear();
    remove_symlinks();

    println!("FITSWebQL: clean shutdown completed.");
}