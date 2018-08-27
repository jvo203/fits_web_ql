#![recursion_limit="1024"]

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

extern crate actix;
extern crate actix_web;
extern crate percent_encoding;
extern crate curl;
extern crate byteorder;
extern crate chrono;
extern crate half;
extern crate uuid;
extern crate futures;
extern crate rayon;
extern crate rusqlite;
extern crate time as precise_time;
extern crate num_integer;
extern crate num;
extern crate num_cpus;
extern crate timer;
extern crate vpx_sys;
extern crate num_rational;
extern crate positioned_io;
extern crate atomic;
extern crate resize;

//extern crate rav1e;
//use rav1e::*;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate bincode;

use bincode::serialize;

use std::sync::Arc;
use std::thread;
use std::{env,ptr};
use std::time::SystemTime;
use std::collections::BTreeMap;

use actix::*;
use actix_web::*;
use actix_web::server::HttpServer;
use futures::future::{Future,result};
use percent_encoding::percent_decode;
use uuid::Uuid;

use resize::Resizer;
use resize::Pixel::Gray8;
use resize::Type::Triangle;

use vpx_sys::*;

#[macro_use]
extern crate scan_fmt;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_json;

extern crate parking_lot;

use std::collections::HashMap;
//use std::sync::RwLock;
use parking_lot::RwLock;

mod molecule;
mod fits;
mod server;
//mod encoder;

use molecule::Molecule;

#[derive(Serialize, Deserialize, Debug)]
pub struct WsSpectrum {
    pub ts: f32,
    pub seq_id: u32,
    pub msg_type: u32,
    pub elapsed: f32,
    pub spectrum: Vec<f32>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WsFrame {
    pub ts: f32,
    pub seq_id: u32,
    pub msg_type: u32,
    pub elapsed: f32,
    pub frame: Vec<u8>
}

struct WsSessionState {
    addr: Addr<server::SessionServer>,
}

struct UserSession {    
    dataset_id: String,
    session_id: Uuid,
    cfg: vpx_codec_enc_cfg_t,//VP9 encoder config
    ctx: vpx_codec_ctx_t,//VP9 encoder context
    downscaling: bool,
    width: u32,
    height: u32,
    resizer: Resizer<Gray8>,
}

impl UserSession {
    pub fn new(id: &String) -> UserSession {
        let session = UserSession {
            dataset_id: id.clone(),            
            session_id: Uuid::new_v4(),            
            cfg: vpx_codec_enc_cfg::default(),       
            ctx: vpx_codec_ctx_t {                
                name: ptr::null(),
                iface: ptr::null_mut(),
                err: VPX_CODEC_OK,
                err_detail: ptr::null(),
                init_flags: 0,
                config: vpx_codec_ctx__bindgen_ty_1 {                    
                    enc: ptr::null(),                    
                },
                priv_: ptr::null_mut(),
            },
            downscaling: false,
            width: 0,
            height: 0,
            resizer: resize::new(0,0,0,0,Gray8,Triangle),
        } ;

        println!("allocating a new websocket session for {}", id);

        session
    }
}

impl Drop for UserSession {
    fn drop(&mut self) {
        println!("dropping a websocket session for {}", self.dataset_id);

        unsafe { vpx_codec_destroy(&mut self.ctx) };
    }
}

impl Actor for UserSession {
    type Context = ws::WebsocketContext<Self, WsSessionState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("websocket connection started for {}", self.dataset_id);

        let addr = ctx.address();

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
                if (&text).contains("[debug]") {
                    println!("{}", text);
                }

                if (&text).contains("[heartbeat]") {
                    ctx.text(&text);       
                }

                if (&text).contains("[init_video]") {
                    println!("{}", text.replace("&"," "));
                    let fps = scan_fmt!(&text.replace("&"," "), "[init_video] fps={}", i32);

                    let fps = match fps {
                        Some(x) => x,
                        _ => 10,//use 10 frames per second by default
                    };

                    //get a read lock to the dataset
                    let datasets = DATASETS.read();

                    let fits = match datasets.get(&self.dataset_id).unwrap().try_read() {
                        Some(x) => x,
                        None => {
                            println!("[WS] cannot find {}", self.dataset_id);
                            return;
                        }
                    };

                    { *fits.timestamp.write() = SystemTime::now() ; }

                    let mut ret = unsafe { vpx_codec_enc_config_default(vpx_codec_vp9_cx(), &mut self.cfg, 0) };

                    if ret != VPX_CODEC_OK {
                        println!("VP9: default configuration failed");
                    }
                    else {
                        let mut w = fits.width as u32 ;
                        let mut h = fits.height as u32 ;
                        let pixel_count = (w as u64) * (h as u64) ;

                        if pixel_count > fits::PIXEL_COUNT_LIMIT {
                            let ratio: f32 = ( (pixel_count as f32) / (fits::PIXEL_COUNT_LIMIT as f32) ).sqrt();
                            w = ( (w as f32) / ratio.sqrt() ) as u32 ;
	                        h = ( (h as f32) / ratio.sqrt() ) as u32 ;

                            println!("downscaling the video from {}x{} to {}x{}", fits.width, fits.height, w, h);

                            self.downscaling = true;
                        }

                        self.width = w ;
                        self.height = h ;
                        self.resizer = resize::new(fits.width as usize, fits.height as usize, w as usize, h as usize, Gray8, Triangle);

                        self.cfg.g_w = w;
                        self.cfg.g_h = h;
                        /*self.cfg.g_timebase.num = 1;
                        self.cfg.g_timebase.den = fps;*/

                        self.cfg.rc_min_quantizer = 10 ;
                        self.cfg.rc_max_quantizer = 42 ;

                        #[cfg(not(feature = "server"))]
                        { self.cfg.rc_target_bitrate = 4096; }// [kilobits per second]

                        #[cfg(feature = "server")]
                        { self.cfg.rc_target_bitrate = 1024; }// [kilobits per second]

                        #[cfg(feature = "server")]
                        { self.cfg.rc_end_usage = vpx_rc_mode::VPX_CBR; }

                        //internal frame downsampling
                        self.cfg.rc_resize_allowed = 1;
                        self.cfg.rc_scaled_width = self.cfg.g_w >> 2;
                        self.cfg.rc_scaled_height = self.cfg.g_h >> 2;
                        self.cfg.rc_resize_down_thresh = 30;

                        self.cfg.g_lag_in_frames = 0;
                        self.cfg.g_pass = vpx_enc_pass::VPX_RC_ONE_PASS;
                        self.cfg.g_threads = num_cpus::get().min(4) as u32 ;//set the upper limit on the number of threads to 4

                        //self.cfg.g_profile = 0 ;

                        //initialise the encoder itself
                        ret = unsafe {                            
                            vpx_codec_enc_init_ver(
                            &mut self.ctx,
                            vpx_codec_vp9_cx(),
                            &mut self.cfg,
                            0,
                            (14+4+5) as i32,//23 for libvpx-1.7.0; VPX_ENCODER_ABI_VERSION does not get expanded correctly by bind-gen
                            )
                        };

                        if ret != VPX_CODEC_OK {            
                            println!("VP9: codec init failed {:?}", ret);
                        }

                        //VP9: -8 - slower, 8 - faster
                        ret = unsafe {vpx_codec_control_(&mut self.ctx, vp8e_enc_control_id::VP8E_SET_CPUUSED as i32, 6) };

                        if ret != VPX_CODEC_OK {            
                            println!("VP9: error setting VP8E_SET_CPUUSED {:?}", ret);
                        }
                    };                    
                }

                if (&text).contains("[end_video]") {
                    println!("{}", text);

                    unsafe { vpx_codec_destroy(&mut self.ctx) };
                }

                if (&text).contains("[spectrum]") {
                    //println!("{}", text.replace("&"," "));
                    let (x1, y1, x2, y2, image, beam, intensity, frame_start, frame_end, ref_freq, seq_id, timestamp) = scan_fmt!(&text.replace("&"," "), "[spectrum] x1={} y1={} x2={} y2={} image={} beam={} intensity={} frame_start={} frame_end={} ref_freq={} seq_id={} timestamp={}", i32, i32, i32, i32, bool, String, String, String, String, String, i32, String);

                    let x1 = match x1 {
                        Some(x) => x,
                        _ => 0,
                    };

                    let y1 = match y1 {
                        Some(y) => y,
                        _ => 0,
                    };

                    let x2 = match x2 {
                        Some(x) => x,
                        _ => 0,
                    };

                    let y2 = match y2 {
                        Some(y) => y,
                        _ => 0,
                    };

                    let image = match image {
                        Some(x) => x,
                        _ => false,
                    };

                    let beam = match beam {
                        Some(s) => match s.as_ref() {
                            "square" => fits::Beam::Square,
                            _ => fits::Beam::Circle,                            
                        },
                        _ => fits::Beam::Circle,
                    };

                    let intensity = match intensity {
                        Some(s) => match s.as_ref() {
                            "mean" => fits::Intensity::Mean,
                            _ => fits::Intensity::Integrated,                            
                        },
                        _ => fits::Intensity::Integrated,
                    };

                    let frame_start = match frame_start {
                        Some(s) => match s.parse::<f64>() {                            
                            Ok(x) => x,
                            Err(_) => 0.0
                        },
                        _ => 0.0,
                    };

                    let frame_end = match frame_end {
                        Some(s) => match s.parse::<f64>() {                            
                            Ok(x) => x,
                            Err(_) => 0.0
                        },
                        _ => 0.0,
                    };

                    let ref_freq = match ref_freq {
                        Some(s) => match s.parse::<f64>() {                            
                            Ok(x) => x,
                            Err(_) => 0.0
                        },
                        _ => 0.0,
                    };

                    let seq_id = match seq_id {
                        Some(x) => x,
                        _ => 0,
                    };

                    let timestamp = match timestamp {
                        Some(s) => match s.parse::<f64>() { 
                            Ok(x) => x,
                            Err(_) => 0.0
                        },
                        _ => 0.0,
                    };

                    println!("x1:{} y1:{} x2:{} y2:{} image:{} beam:{:?} intensity:{:?} frame_start:{} frame_end:{} ref_freq:{} seq_id:{} timestamp:{}", x1, y1, x2, y2, image, beam, intensity, frame_start, frame_end, ref_freq, seq_id, timestamp);

                    //get a read lock to the dataset
                    let datasets = DATASETS.read();

                    let fits = match datasets.get(&self.dataset_id).unwrap().try_read() {
                        Some(x) => x,
                        None => {
                            let msg = json!({
                                "type" : "spectrum",
                                "message" : "unavailable",                  
                            });

                            ctx.text(msg.to_string());
                            return;
                        }
                    };

                    { *fits.timestamp.write() = SystemTime::now() ; }

                    if fits.has_data {
                        let start = precise_time::precise_time_ns(); 
                        match fits.get_spectrum(x1, y1, x2, y2, beam, intensity, frame_start, frame_end, ref_freq) {
                            Some(spectrum) => {
                                let stop = precise_time::precise_time_ns(); 
                                let elapsed = (stop-start)/1000000 ;
                                //send a binary response message (serialize a structure to a binary stream)
                                let ws_spectrum = WsSpectrum {
                                    ts: timestamp as f32,
                                    seq_id: seq_id as u32,
                                    msg_type: 0,
                                    //length: spectrum.len() as u32,
                                    elapsed: elapsed as f32,
                                    spectrum: spectrum
                                };

                                match serialize(&ws_spectrum) {
                                    Ok(bin) => {                      
                                        println!("binary length: {}", bin.len());
                                        //println!("{}", bin);
                                        ctx.binary(bin);
                                    },
                                    Err(err) => println!("error serializing a WebSocket spectrum response: {}", err)
                                }            
                            },
                            None => {},
                        };                                        
                    };
                }

                if (&text).contains("[image]") {
                    let datasets = DATASETS.read();

                    let fits = match datasets.get(&self.dataset_id).unwrap().try_read() {
                        Some(x) => x,
                        None => {
                            let msg = json!({
                                "type" : "image",
                                "message" : "unavailable",                  
                            });

                            ctx.text(msg.to_string());
                            return;
                        }
                    };

                    { *fits.timestamp.write() = SystemTime::now() ; }

                    if fits.is_dummy {
                        let msg = json!({
                            "type" : "image",
                            "message" : "unavailable",                  
                        });

                        ctx.text(msg.to_string());
                        return;
                    };

                    if fits.has_data {
                        //fits.make_vpx_image()
                        //send a binary response
                    };
                }


                if (&text).contains("[video]") {
                    //println!("{}", text.replace("&"," "));
                    let (frame, key, ref_freq, seq_id, timestamp) = scan_fmt!(&text.replace("&"," "), "[video] frame={} key={} ref_freq={} seq_id={} timestamp={}", String, bool,String, i32, String);

                    let frame = match frame {
                        Some(s) => match s.parse::<f64>() {                            
                            Ok(x) => x,
                            Err(_) => 0.0
                        },
                        _ => 0.0,
                    };

                    let ref_freq = match ref_freq {
                        Some(s) => match s.parse::<f64>() {                            
                            Ok(x) => x,
                            Err(_) => 0.0
                        },
                        _ => 0.0,
                    };

                    let keyframe = match key {
                        Some(x) => x,
                        _ => false,
                    };

                    let seq_id = match seq_id {
                        Some(x) => x,
                        _ => 0,
                    };

                    let timestamp = match timestamp {
                        Some(s) => match s.parse::<f64>() { 
                            Ok(x) => x,
                            Err(_) => 0.0
                        },
                        _ => 0.0,
                    };

                    println!("frame:{} keyframe:{} ref_freq:{} seq_id:{} timestamp:{}", frame, keyframe, ref_freq, seq_id, timestamp);

                    let datasets = DATASETS.read();

                    let fits = match datasets.get(&self.dataset_id).unwrap().try_read() {
                        Some(x) => x,
                        None => {
                            let msg = json!({
                                "type" : "video",
                                "message" : "unavailable",                  
                            });

                            ctx.text(msg.to_string());
                            return;
                        }
                    };

                    { *fits.timestamp.write() = SystemTime::now() ; }

                    if fits.has_data {
                        let start = precise_time::precise_time_ns();
                        match fits.get_video_frame(frame, ref_freq, self.width, self.height, &mut self.resizer, self.downscaling) {                            
                            Some(mut image) => {
                                //serialize a video response with seq_id, timestamp
                                //send a binary response
                                //print!("{:#?}", image);

                                //let start = precise_time::precise_time_ns();

                                let mut flags = 0;
                                if keyframe {
                                    flags |= VPX_EFLAG_FORCE_KF;
                                };                                                                

                                //call encode_frame with a valid frame image
                                let mut video_frame: Vec<u8> = Vec::new();

                                match fits::encode_frame(self.ctx, image, 0, flags as i64, VPX_DL_REALTIME as u64) {
                                    Ok(res) => match res {                                        
                                        Some(res) => video_frame = res,
                                    _ => {},
                                    },
                                    Err(err) => {
                                        println!("codec error: {:?}", err);
                                    }
                                };

                                unsafe { vpx_img_free(&mut image) };

                                if keyframe {
                                    //flush the encoder to signal the end    
                                    match fits::flush_frame(self.ctx, VPX_DL_REALTIME as u64) {
                                        Ok(res) => match res {
                                            Some(res) => video_frame = res,
                                            _ => {},
                                        },
                                        Err(err) => {
                                            println!("codec error: {:?}", err);
                                        }
                                    }
                                }

                                let stop = precise_time::precise_time_ns();

                                println!("VP9 video frame prepare/encode time: {} [ms], speed {} frames per second, frame length: {} bytes", (stop-start)/1000000, 1000000000/(stop-start), video_frame.len());

                                if !video_frame.is_empty() {                                    
                                    //println!("VP9 video frame length: {} bytes", video_frame.len());
                                    //send a binary response message (serialize a structure to a binary stream)
                                    let ws_frame = WsFrame {                                        
                                        ts: timestamp as f32,
                                        seq_id: seq_id as u32,
                                        msg_type: 5,//a video frame
                                        //length: video_frame.len() as u32,
                                        elapsed: ( (stop-start)/1000000 ) as f32,
                                        frame: video_frame
                                    };

                                    match serialize(&ws_frame) {                                        
                                        Ok(bin) => {                      
                                            println!("binary length: {}", bin.len());
                                            //println!("{}", bin);
                                            ctx.binary(bin);
                                        },
                                        Err(err) => println!("error serializing a WebSocket video frame response: {}", err)
                                    }
                                }                            
                            },
                            None => {},
                        };
                    };
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
    static ref DATASETS: Arc<RwLock<HashMap<String, Arc<RwLock<Box<fits::FITS>>>>>> = {
        Arc::new(RwLock::new(HashMap::new()))
    };
}

#[cfg(not(feature = "server"))]
static SERVER_STRING: &'static str = "FITSWebQL v1.2.0";
#[cfg(feature = "server")]
static SERVER_STRING: &'static str = "FITSWebQL v3.2.0";

static VERSION_STRING: &'static str = "SV2018-08-27.2";

#[cfg(not(feature = "server"))]
static SERVER_MODE: &'static str = "LOCAL";

#[cfg(feature = "server")]
static SERVER_MODE: &'static str = "SERVER";

#[cfg(not(feature = "server"))]
const SERVER_ADDRESS: &'static str = "localhost";

#[cfg(feature = "server")]
const SERVER_ADDRESS: &'static str = "0.0.0.0";

const SERVER_PORT: i32 = 8080;

//const LONG_POLL_TIMEOUT: u64 = 100;//[ms]; keep it short, long intervals will block the actix event loop

fn fetch_molecules(freq_start: f32, freq_end: f32) -> String {
    //splatalogue sqlite db integration    
    let mut molecules : Vec<serde_json::Value> = Vec::new();

    let splat_path = std::path::Path::new("splatalogue_v3.db");

    match rusqlite::Connection::open(splat_path) {
        Ok(splat_db) => {
            println!("[fetch_molecules] connected to splatalogue sqlite");

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
        Err(err) => {
            println!("error connecting to splatalogue sqlite: {}", err);
        }
    };

    let mut contents = String::from("[");

    for entry in &molecules {
        contents.push_str(&entry.to_string()) ;
        contents.push(',');
    };

    if !molecules.is_empty() {
        contents.pop() ;
    }   

    contents.push(']');

    contents
}

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

fn directory_handler(req: &HttpRequest<WsSessionState>) -> HttpResponse {
    let query = req.query();

    match query.get("dir") {
        Some(x) => get_directory(std::path::PathBuf::from(x)),
        None => get_home_directory()//default database
    }
}

// do websocket handshake and start an actor
/*fn websocket_entry(req: HttpRequest<WsSessionState>) -> Result<Box<Future<Item=HttpResponse, Error=Error>>, Error> {
    let dataset_id_orig: String = req.match_info().query("id").unwrap();

    //dataset_id needs to be URI-decoded
    let dataset_id = match percent_decode(dataset_id_orig.as_bytes()).decode_utf8() {
        Ok(x) => x.into_owned(),
        Err(_) => dataset_id_orig.clone(),
    };

    let session = UserSession::new(&dataset_id);

    Ok(Box::new(result(ws::start(req, session))))
}*/

fn websocket_entry(req: &HttpRequest<WsSessionState>) -> Result<HttpResponse> {
    let dataset_id_orig: String = req.match_info().query("id").unwrap();

    //dataset_id needs to be URI-decoded
    let dataset_id = match percent_decode(dataset_id_orig.as_bytes()).decode_utf8() {
        Ok(x) => x.into_owned(),
        Err(_) => dataset_id_orig.clone(),
    };

    let session = UserSession::new(&dataset_id);

    ws::start(req, session)
}

fn fitswebql_entry(req: &HttpRequest<WsSessionState>) -> HttpResponse {
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
        Some(x) => {vec![x.as_str()]},
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
                    .body(format!("<p><b>Critical Error</b>: no {} available</p>", dataset));                    
                };
            
            v
        }
    };

    let composite = match query.get("view") {
        Some(x) => {
            match x.as_ref() {
                "composite" => true,
                _ => false
            }
        },
        None => false
    };

    let flux = match query.get("flux") {
        Some(x) => {x},
        None => {""}//nothing by default
    };

    #[cfg(feature = "server")]
    let resp = format!("FITSWebQL path: {}, db: {}, table: {}, dataset_id: {:?}, composite: {}, flux: {}", fitswebql_path, db, table, dataset_id, composite, flux);

    #[cfg(not(feature = "server"))]
    let resp = format!("FITSWebQL path: {}, dir: {}, ext: {}, filename: {:?}, composite: {}, flux: {}", fitswebql_path, dir, ext, dataset_id, composite, flux);

    println!("{}", resp);

    //server version
    //execute_fits(&fitswebql_path, &db, &table, &dataset_id, composite, &flux, &server)
    #[cfg(feature = "server")]    
    return execute_fits(&fitswebql_path, fits::FITSCACHE, "fits", &dataset_id, composite, &flux, &server);

    //local (Personal Edition)
    #[cfg(not(feature = "server"))]
    return execute_fits(&fitswebql_path, &dir, &ext, &dataset_id, composite, &flux, &server);
}


fn get_image(req: &HttpRequest<WsSessionState>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    //println!("{:?}", req);

    let query = req.query() ;

    let dataset_id = match query.get("datasetId") {
        Some(x) => {x},
        None => {            
            return result(Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: get_spectrum/datasetId parameter not found</p>"))))
                .responder()
        }
    };

    //println!("[get_image] http request for {}", dataset_id);    

    //check the IMAGECACHE first
    let filename = format!("{}/{}.vp9", fits::IMAGECACHE, dataset_id.replace("/","_"));
    let filepath = std::path::Path::new(&filename);

    if filepath.exists() {
        return result(fs::NamedFile::open(filepath).respond_to(&req)).responder();
    };

    result(Ok({
        let datasets = DATASETS.read();//.unwrap();

        //println!("[get_image] obtained read access to <DATASETS>, trying to get read access to {}", dataset_id);

        let fits = match datasets.get(dataset_id).unwrap().try_read()/*_for(time::Duration::from_millis(LONG_POLL_TIMEOUT))*/ {
            Some(x) => x,
            None => {
                //println!("[get_image]: RwLock timeout, cannot obtain a read access to {}", dataset_id);

                return result(Ok(HttpResponse::Accepted()
                    .content_type("text/html")
                    .body(format!("<p><b>RwLock timeout</b>: {} not available yet</p>", dataset_id))))
                    .responder();
            }
        };

        { *fits.timestamp.write() = SystemTime::now() ; }

        //println!("[get_image] obtained read access to {}, has_data = {}", dataset_id, fits.has_data);

        if fits.is_dummy {
            return result(Ok(HttpResponse::Accepted()
                    .content_type("text/html")
                    .body(format!("<p><b>RwLock timeout</b>: {} not available yet</p>", dataset_id))
                    )).responder();
        }

        if fits.has_data {
            //send the binary image data from IMAGECACHE                       
            let filename = format!("{}/{}.vp9", fits::IMAGECACHE, dataset_id.replace("/","_"));
            let filepath = std::path::Path::new(&filename);

            if filepath.exists() {
                return result(fs::NamedFile::open(filepath).respond_to(&req)).responder();
            }
            else {
                return result(Ok(HttpResponse::NotFound()
                    .content_type("text/html")
                    .body(format!("<p><b>Critical Error</b>: spectrum not found</p>"))
                )).responder();
            };
        }
        else {
            HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: spectrum not found</p>"))
        }
    }))
    .responder()
}

fn get_spectrum(req: &HttpRequest<WsSessionState>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    //println!("{:?}", req);

    let query = req.query() ;

    let dataset_id = match query.get("datasetId") {
        Some(x) => {x},
        None => {            
            return result(Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: get_spectrum/datasetId parameter not found</p>"))))
                .responder()
        }
    };

    //println!("[get_spectrum] http request for {}", dataset_id);

    result(Ok({
        let datasets = DATASETS.read();//.unwrap();

        //println!("[get_spectrum] obtained read access to <DATASETS>, trying to get read access to {}", dataset_id);

        let fits = match datasets.get(dataset_id).unwrap().try_read()/*_for(time::Duration::from_millis(LONG_POLL_TIMEOUT))*/ {
            Some(x) => x,
            None => {
                //println!("[get_spectrum]: RwLock timeout, cannot obtain a read access to {}", dataset_id);

                return result(Ok(HttpResponse::Accepted()
                    .content_type("text/html")
                    .body(format!("<p><b>RwLock timeout</b>: {} not available yet</p>", dataset_id))))
                    .responder();
            }
        };

        { *fits.timestamp.write() = SystemTime::now() ; }

        //println!("[get_spectrum] obtained read access to {}, has_data = {}", dataset_id, fits.has_data);

        if fits.is_dummy {
            return result(Ok(HttpResponse::Accepted()
                    .content_type("text/html")
                    .body(format!("<p><b>RwLock timeout</b>: {} not available yet</p>", dataset_id))))
                    .responder();
        }

        if fits.has_data {
            HttpResponse::Ok()
                .content_type("application/json")
                .body(format!("{}",fits.to_json()))
        }
        else {
            HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: spectrum not found</p>"))
        }
    }))
    .responder()
}

fn get_molecules(req: &HttpRequest<WsSessionState>) -> Box<Future<Item=HttpResponse, Error=Error>> {
    //println!("{:?}", req);    

    let query = req.query() ;

    let dataset_id = match query.get("datasetId") {
        Some(x) => {x},
        None => {            
            return result(Ok(HttpResponse::NotFound()
                .content_type("text/html")
                .body(format!("<p><b>Critical Error</b>: get_molecules/datasetId parameter not found</p>"))))
                .responder();
        }
    };

    //freq_start
    let freq_start = match query.get("freq_start") {
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
    let freq_end = match query.get("freq_end") {
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
        if freq_start == 0.0 || freq_end == 0.0 {
            //send a request to the SessionServers

            let state = req.state();
            let server = &state.addr;

            let resp = server.send(server::GetMolecules {                
                dataset_id: dataset_id.to_owned(),
            })
            .wait();            
            
            match resp {
                Ok(content) => {
                    if content == "" {
                        HttpResponse::Accepted()                        
                            .content_type("text/html")
                            .body(format!("<p><b>spectral lines for {} not available yet</p>", dataset_id))                            
                    }
                    else {
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .body(format!("{{\"molecules\" : {}}}", content))                            
                    }
                },
                Err(_) => {
                    HttpResponse::NotFound()
                        .content_type("text/html")
                        .body(format!("<p><b>Critical Error</b>: spectral lines not found</p>"))                        
                }
            }                         
        }
        else {                        
            //fetch molecules from sqlite without waiting for a FITS header            
            let content = fetch_molecules(freq_start, freq_end);                

            HttpResponse::Ok()
                .content_type("application/json")
                .body(format!("{{\"molecules\" : {}}}", content))
        }
    }))
    .responder()
}

//#[cfg(not(feature = "server"))]
fn execute_fits(fitswebql_path: &String, dir: &str, ext: &str, dataset_id: &Vec<&str>, composite: bool, flux: &str, server: &Addr<server::SessionServer>) -> HttpResponse {

    //get fits location    

    //launch FITS threads
    let mut has_fits: bool = true ;

    //for each dataset_id
    for i in 0..dataset_id.len() {
        let data_id = dataset_id[i];

        //does the entry exist in the datasets hash map?
        let has_entry = {
            let datasets = DATASETS.read();
            datasets.contains_key(data_id) 
        } ;

        //if it does not exist set has_fits to false and load the FITS data
        if !has_entry {
            has_fits = false ;            

            let my_dir = dir.to_string();
            let my_data_id = data_id.to_string();
            let my_ext = ext.to_string();
            let my_server = server.clone();
            let my_flux = flux.to_string();
                                    
            DATASETS.write().insert(my_data_id.clone(), Arc::new(RwLock::new(Box::new(fits::FITS::new(&my_data_id, &my_flux)))));
               
            //load FITS data in a new thread
            thread::spawn(move || {
                let filename = format!("{}/{}.{}", my_dir, my_data_id, my_ext);
                println!("loading FITS data from {}", filename); 

                let filepath = std::path::Path::new(&filename);           
                let fits = fits::FITS::from_path(&my_data_id.clone(), &my_flux.clone(), filepath, &my_server);//from_path or from_path_mmap

                let fits = Arc::new(RwLock::new(Box::new(fits)));
                
                DATASETS.write().insert(my_data_id.clone(), fits.clone());

                if fits.read().has_data {
                    thread::spawn(move || {
                        fits.read().make_data_histogram();                
                    });
                };
            });
        }
        else {
            //update the timestamp
            let datasets = DATASETS.read();
            let dataset = datasets.get(data_id).unwrap().read() ;
            
            has_fits = has_fits && dataset.has_data ;
            *dataset.timestamp.write() = SystemTime::now() ;
        } ;
    } ;

    http_fits_response(&fitswebql_path, &dataset_id, composite, has_fits)
}

fn http_fits_response(fitswebql_path: &String, dataset_id: &Vec<&str>, composite: bool, has_fits: bool) -> HttpResponse {
    println!("calling http_fits_response for {:?}", dataset_id);
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
    //html.push_str("<script src=\"lz4.min.js\"></script>\n");
    html.push_str("<script src=\"marchingsquares-isocontours.min.js\"></script>\n");
    html.push_str("<script src=\"marchingsquares-isobands.min.js\"></script>\n");    

    //VP9 decoder
    html.push_str("<script src=\"ogv-decoder-video-vp9.js\"></script>\n");
    //html.push_str("<script src=\"ogv-decoder-video-vp9-wasm.js\"></script>\n");
    //html.push_str("<script src=\"ogv.js\"></script>\n");

    //custom vpx wasm
    html.push_str("<script src=\"vpx.js\"></script>\n");
    html.push_str("<script>
        Module.onRuntimeInitialized = async _ => {
            api = {
                vpx_version: Module.cwrap('vpx_version', 'number', []),
                vpx_init: Module.cwrap('vpx_init', '', []),
                vpx_destroy: Module.cwrap('vpx_destroy', '', []),
                vpx_decode_frame: Module.cwrap('vpx_decode_frame', 'number', ['number', 'number', 'number', 'number', 'number', 'string']),
            };
            console.log('VP9 libvpx decoder version:', api.vpx_version());
            api.vpx_init();
        };
    </script>\n");

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
    //an experimental AV1 part
    //let config = EncoderConfig::default();
    //println!("{:?}", config);
    //end of AV1

    let mut server_port = SERVER_PORT ;

    let args: Vec<String> = env::args().collect();

    if args.len() > 2 {
        let key = &args[1];
        let value = &args[2];

        if key == "--port" {
            server_port = value.parse::<i32>().unwrap();
        }
    }

    remove_symlinks();

    //splatalogue sqlite db integration
    /*let splat_path = std::path::Path::new("splatalogue_v3.db");
    let splat_db = sqlite::open(splat_path).unwrap();*/

    #[cfg(not(feature = "server"))]
    let index_file = "fitswebql.html" ;

    #[cfg(feature = "server")]
    let index_file = "almawebql.html" ;    

    let sys = actix::System::new("fits_web_ql");

    // Start the WebSocket message server actor in a separate thread
    let server = Arbiter::start(|_| server::SessionServer::default());    
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
                .resource("/{path}/get_image", |r| {r.method(http::Method::GET).f(get_image)})
                .resource("/{path}/get_spectrum", |r| {r.method(http::Method::GET).f(get_spectrum)})
                .resource("/{path}/get_molecules", |r| {r.method(http::Method::GET).f(get_molecules)})
                .handler("/", fs::StaticFiles::new("htdocs").unwrap().index_file(index_file))
        })
        .bind(&format!("{}:{}", SERVER_ADDRESS, server_port)).expect(&format!("Cannot bind to localhost:{}, try setting a different HTTP port via a command-line option '--port XXXX'", server_port))        
        .start();

    println!("detected number of CPUs: {}", num_cpus::get());

    #[cfg(not(feature = "server"))]
    {
        println!("started a local FITSWebQL server; point your browser to http://localhost:{}", server_port);
        println!("press CTRL+C to exit");
    }    

    #[cfg(feature = "server")]
    {
        println!("started a fits_web_ql server process on port {}", server_port);
        println!("send SIGINT to shutdown, i.e. killall -s SIGINT fits_web_ql");
    }

    let _ = sys.run();

    DATASETS.write().clear();
    remove_symlinks();

    println!("FITSWebQL: clean shutdown completed.");
}