#![recursion_limit = "1024"]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[macro_use]
extern crate ispc;

// Functions exported from ispc will be callable under spmd::*
ispc_module!(spmd);

#[macro_use]
extern crate scan_fmt;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use bincode::serialize;
use chrono::Local;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;
use std::{env, mem, ptr};

use ::actix::*;
use actix_web::http::header::HeaderValue;
use actix_web::http::ContentEncoding;
use actix_web::middleware::Logger;
use actix_web::server::HttpServer;
use actix_web::*;
use futures::future::{result, Future};
use percent_encoding::percent_decode;
use tar::{Builder, Header};
use uuid::Uuid;

use bytes::Bytes;
use futures::{Async, Poll, Stream};
use std::sync::mpsc;

use rayon::prelude::*;
use std::cmp::Ordering::Equal;

use log::info;
use time as precise_time;
//use rav1e::*;

#[cfg(feature = "server")]
use postgres::{Connection, TlsMode};

use vpx_sys::*;

#[cfg(feature = "opencl")]
use ocl::core;

use std::collections::{HashMap, HashSet};

//use std::sync::RwLock;
use parking_lot::RwLock;
//use parking_lot::Mutex;

mod fits;
mod kalman;
mod molecule;
mod server;

use crate::kalman::KalmanFilter;
use crate::molecule::Molecule;

#[derive(Serialize, Debug)]
pub struct WsSpectrum {
    pub ts: f32,
    pub seq_id: u32,
    pub msg_type: u32,
    pub elapsed: f32,
    pub spectrum: Vec<f32>,
}

#[derive(Serialize, Debug)]
pub struct WsSpectra {
    pub ts: f32,
    pub seq_id: u32,
    pub msg_type: u32,
    pub mean_spectrum: Vec<f32>,
    pub integrated_spectrum: Vec<f32>,
}

#[derive(Serialize, Debug)]
pub struct WsHistogram {
    pub ts: f32,
    pub seq_id: u32,
    pub msg_type: u32,
    pub pmin: f32,
    pub pmax: f32,
    pub black: f32,
    pub white: f32,
    pub median: f32,
    pub sensitivity: f32,
    pub ratio_sensitivity: f32,
    pub hist: Vec<i32>,
}

#[derive(Serialize, Debug)]
pub struct WsFrame {
    pub ts: f32,
    pub seq_id: u32,
    pub msg_type: u32,
    pub elapsed: f32,
    pub frame: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct WsImage {
    pub ts: f32,
    pub seq_id: u32,
    pub msg_type: u32,
    pub identifier: String,
    pub width: u32,
    pub height: u32,
    pub image: Vec<u8>,
    pub alpha: Vec<u8>,
}

#[derive(Serialize, Debug)]
pub struct WsViewport {
    pub ts: f32,
    pub seq_id: u32,
    pub msg_type: u32,
    pub identifier: String,
    pub width: u32,
    pub height: u32,
    pub image: Vec<Vec<u8>>,
    pub alpha: Vec<u8>,
}

struct WsSessionState {
    addr: Addr<server::SessionServer>,
    home_dir: Option<std::path::PathBuf>,
}

pub struct UserParams {
    pmin: f32,
    pmax: f32,
    lmin: f32,
    lmax: f32,
    black: f32,
    white: f32,
    median: f32,
    sensitivity: f32,
    ratio_sensitivity: f32,
    flux: String,
    start: usize,
    end: usize,
    mask: Vec<u8>,
    pixels: Vec<f32>,
}

struct UserSession {
    dataset_id: Vec<String>,
    session_id: Uuid,
    user: Option<UserParams>,
    timestamp: std::time::Instant,
    log: std::io::Result<File>,
    wasm: bool,
    //hevc: std::io::Result<File>,
    cfg: vpx_codec_enc_cfg_t, //VP9 encoder config
    ctx: vpx_codec_ctx_t,     //VP9 encoder context
    param: *mut x265_param,   //HEVC param
    enc: *mut x265_encoder,   //HEVC context
    pic: *mut x265_picture,   //HEVC picture
    //config: EncoderConfig,
    width: u32,
    height: u32,
    streaming: bool,
    last_video_frame: i32,
    video_frame: f64,
    video_ref_freq: f64,
    video_fps: f64,
    video_seq_id: i32,
    video_timestamp: std::time::Instant,
    bitrate: i32,
    kf: KalmanFilter,
}

impl UserSession {
    pub fn new(id: &Vec<String>) -> UserSession {
        let uuid = Uuid::new_v4();

        #[cfg(not(feature = "server"))]
        let filename = format!("/dev/null");

        #[cfg(feature = "server")]
        let filename = format!("{}/{}_{}.log", LOG_DIRECTORY, id[0].replace("/", "_"), uuid);

        let log = File::create(filename);

        /*#[cfg(not(feature = "server"))]
        let filename = format!("/dev/null");

        #[cfg(feature = "server")]
        let filename = format!(
            "{}/{}_{}.hevc",
            LOG_DIRECTORY,
            id[0].replace("/", "_"),
            uuid
        );

        let hevc = File::create(filename);*/

        let session = UserSession {
            dataset_id: id.clone(),
            session_id: uuid,
            user: None,
            timestamp: std::time::Instant::now(), //SpawnHandle::default(),
            log: log,
            wasm: false,
            //hevc: hevc,
            cfg: vpx_codec_enc_cfg::default(),
            ctx: vpx_codec_ctx_t {
                name: ptr::null(),
                iface: ptr::null_mut(),
                err: VPX_CODEC_OK,
                err_detail: ptr::null(),
                init_flags: 0,
                config: vpx_codec_ctx__bindgen_ty_1 { enc: ptr::null() },
                priv_: ptr::null_mut(),
            },
            param: ptr::null_mut(),
            enc: ptr::null_mut(),
            pic: ptr::null_mut(),
            //config: EncoderConfig::default(),
            width: 0,
            height: 0,
            streaming: false,
            last_video_frame: -1,
            video_frame: 0.0,
            video_ref_freq: 0.0,
            video_fps: 10.0,
            video_seq_id: 0,
            video_timestamp: std::time::Instant::now(),
            bitrate: 1000,
            kf: KalmanFilter::default(),
        };

        println!("allocating a new websocket session for {}", id[0]);

        session
    }
}

impl Drop for UserSession {
    fn drop(&mut self) {
        println!("dropping a websocket session for {}", self.dataset_id[0]);

        unsafe { vpx_codec_destroy(&mut self.ctx) };

        unsafe {
            if !self.param.is_null() {
                x265_param_free(self.param);
            }

            if !self.enc.is_null() {
                x265_encoder_close(self.enc);
            }

            if !self.pic.is_null() {
                x265_picture_free(self.pic);
            }
        }
    }
}

impl Actor for UserSession {
    type Context = ws::WebsocketContext<Self, WsSessionState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("websocket connection started for {}", self.dataset_id[0]);

        let addr = ctx.address();

        ctx.state().addr.do_send(server::Connect {
            addr: addr.recipient(),
            dataset_id: self.dataset_id[0].clone(),
            id: self.session_id,
        });

        ctx.run_interval(std::time::Duration::new(10, 0), |act, ctx| {
            if std::time::Instant::now().duration_since(act.timestamp)
                > std::time::Duration::new(WEBSOCKET_TIMEOUT, 0)
            {
                println!("websocket inactivity timeout for {}", act.dataset_id[0]);

                ctx.text("[close]");
                ctx.stop();
            }
        });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        println!(
            "stopping a websocket connection for {}/{}",
            self.dataset_id[0], self.session_id
        );

        ctx.state().addr.do_send(server::Disconnect {
            dataset_id: self.dataset_id[0].clone(),
            id: self.session_id.clone(),
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

                //check if WebAssembly is supported
                if (&text).contains("WebAssembly is supported") {
                    self.wasm = true;
                }

                if (&text).contains("[heartbeat]") {
                    ctx.text(&text);
                } else {
                    self.timestamp = std::time::Instant::now();

                    match self.log {
                        Ok(ref mut file) => {
                            let timestamp = Local::now();
                            let log_entry =
                                format!("{}\t{}\n", timestamp.format("%Y-%m-%d %H:%M:%S"), text);
                            let _ = file.write_all(log_entry.as_bytes());
                        }
                        Err(_) => {}
                    };
                }

                if (&text).contains("[init_video]") {
                    //println!("{}", text.replace("&", " "));
                    let (frame, view, ref_freq, fps, seq_id, target_bitrate, timestamp) = scan_fmt!(
                        &text.replace("&", " "),
                        "[init_video] frame={} view={} ref_freq={} fps={} seq_id={} bitrate={} timestamp={}",
                        String,
                        String,
                        String,
                        String,
                        i32,
                        i32,
                        String
                    );

                    let frame = match frame {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let is_composite = match view {
                        Some(s) => {
                            if s.contains("composite") {
                                true
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };

                    let ref_freq = match ref_freq {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    //use 10 frames per second by default
                    let fps = match fps {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 10.0,
                        },
                        _ => 10.0,
                    };

                    let seq_id = match seq_id {
                        Some(x) => x,
                        _ => 0,
                    };

                    let target_bitrate = match target_bitrate {
                        Some(x) => num::clamp(x, 100, 10000),
                        _ => 1000,
                    };

                    let timestamp = match timestamp {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    println!(
                        "[init_video] frame:{} is_composite:{} ref_freq:{} fps:{} seq_id:{} target_bitrate:{} timestamp:{}",
                        frame, is_composite, ref_freq, fps, seq_id, target_bitrate, timestamp
                    );

                    self.kf = KalmanFilter::new(frame);

                    self.video_frame = frame;
                    self.video_ref_freq = ref_freq;
                    self.video_fps = fps;
                    self.video_seq_id = seq_id;
                    self.video_timestamp = std::time::Instant::now(); //timestamp;
                    self.bitrate = target_bitrate;

                    //get a read lock to the dataset
                    let datasets = DATASETS.read();

                    let fits = match datasets.get(&self.dataset_id[0]).unwrap().try_read() {
                        Some(x) => x,
                        None => {
                            println!("[WS] cannot find {}", self.dataset_id[0]);
                            return;
                        }
                    };

                    {
                        *fits.timestamp.write() = SystemTime::now();
                    }

                    self.last_video_frame = -1;

                    //alloc HEVC params
                    if self.param.is_null() {
                        self.param = unsafe { x265_param_alloc() };
                        unsafe {
                            //x265_param_default_preset(self.param, CString::new("ultrafast").unwrap().as_ptr(), CString::new("fastdecode").unwrap().as_ptr());

                            if self.dataset_id.len() == 1 || !is_composite {
                                x265_param_default_preset(
                                    self.param,
                                    CString::new("superfast").unwrap().as_ptr(),
                                    CString::new("zerolatency").unwrap().as_ptr(),
                                );
                            } else {
                                x265_param_default_preset(
                                    self.param,
                                    CString::new("ultrafast").unwrap().as_ptr(),
                                    CString::new("zerolatency").unwrap().as_ptr(),
                                );
                            }

                            (*self.param).fpsNum = fps as u32;
                            (*self.param).fpsDenom = 1;
                        };
                    }

                    let mut ret = unsafe {
                        vpx_codec_enc_config_default(vpx_codec_vp9_cx(), &mut self.cfg, 0)
                    };

                    if ret != VPX_CODEC_OK || self.param.is_null() {
                        println!("video codec default configuration failed");
                    } else {
                        let mut w = fits.width as u32;
                        let mut h = fits.height as u32;
                        let pixel_count = (w as u64) * (h as u64);

                        if pixel_count > fits::VIDEO_PIXEL_COUNT_LIMIT {
                            let ratio: f32 = ((pixel_count as f32)
                                / (fits::VIDEO_PIXEL_COUNT_LIMIT as f32))
                                .sqrt();

                            if ratio > 4.5 {
                                //default scaling, no optimisations
                                w = ((w as f32) / ratio) as u32;
                                h = ((h as f32) / ratio) as u32;

                                println!(
                                    "downscaling the video from {}x{} to {}x{}, default ratio: {}",
                                    fits.width, fits.height, w, h, ratio
                                );
                            } else if ratio > 3.0 {
                                // 1/4
                                w = w / 4;
                                h = h / 4;

                                println!(
                                    "downscaling the video from {}x{} to {}x{} (1/4)",
                                    fits.width, fits.height, w, h
                                );
                            } else if ratio > 2.25 {
                                // 3/8
                                w = 3 * w / 8;
                                h = (h * 3 + 7) / 8;

                                println!(
                                    "downscaling the video from {}x{} to {}x{} (3/8)",
                                    fits.width, fits.height, w, h
                                );
                            } else if ratio > 1.5 {
                                // 1/2
                                w = w / 2;
                                h = h / 2;

                                println!(
                                    "downscaling the video from {}x{} to {}x{} (1/2)",
                                    fits.width, fits.height, w, h
                                );
                            } else if ratio > 1.0 {
                                // 3/4
                                w = 3 * w / 4;
                                h = 3 * h / 4;

                                println!(
                                    "downscaling the video from {}x{} to {}x{} (3/4)",
                                    fits.width, fits.height, w, h
                                );
                            }
                        } else {
                            //should we upscale the image to get around a limitation of HEVC x265?
                            if w < 32 || h < 32 {
                                let ratio = (32_f32 / w as f32).max(32_f32 / h as f32);

                                w = ((w as f32) * ratio) as u32;
                                h = ((h as f32) * ratio) as u32;

                                println!(
                                    "upscaling the video from {}x{} to {}x{} (x{})",
                                    fits.width, fits.height, w, h, ratio
                                );
                            }
                        }

                        //get the alpha channel
                        let alpha_frame = {
                            let start = precise_time::precise_time_ns();

                            //invert/downscale the mask (alpha channel) without interpolation
                            let mut alpha = vec![0; (w * h) as usize];

                            fits.resize_and_invert(
                                &fits.mask,
                                &mut alpha,
                                w,
                                h,
                                libyuv_FilterMode_kFilterNone,
                            );

                            let compressed_alpha = lz4_compress::compress(&alpha);

                            let stop = precise_time::precise_time_ns();

                            println!("alpha original length {}, lz4-compressed {} bytes, elapsed time {} [ms]", alpha.len(), compressed_alpha.len(), (stop-start)/1000000);

                            compressed_alpha
                        };

                        //send the video size + alpha channel as JSON
                        let resolution = json!({
                            "type" : "init_video",
                            "width" : w,
                            "height" : h,
                            "alpha" : alpha_frame,
                        });

                        ctx.text(resolution.to_string());

                        //HEVC config
                        unsafe {
                            (*self.param).bRepeatHeaders = 1;

                            if self.dataset_id.len() > 1 && is_composite {
                                (*self.param).internalCsp = X265_CSP_I444 as i32;
                            } else {
                                (*self.param).internalCsp = X265_CSP_I400 as i32;
                            }

                            (*self.param).internalBitDepth = 8;
                            (*self.param).sourceWidth = w as i32;
                            (*self.param).sourceHeight = h as i32;

                            //constant bitrate
                            (*self.param).rc.rateControlMode = X265_RC_METHODS_X265_RC_CRF as i32;
                            (*self.param).rc.bitrate = target_bitrate; //1000;
                        };

                        if self.pic.is_null() {
                            self.pic = unsafe { x265_picture_alloc() };
                        }

                        if self.enc.is_null() {
                            self.enc = unsafe { x265_encoder_open(self.param) }; //x265_encoder_open_160 for x265 2.8
                            unsafe { x265_picture_init(self.param, self.pic) };
                        }

                        self.width = w;
                        self.height = h;
                        self.cfg.g_w = w;
                        self.cfg.g_h = h;
                        self.cfg.g_timebase.num = 1;
                        self.cfg.g_timebase.den = fps as i32;

                        self.cfg.rc_min_quantizer = 10;
                        self.cfg.rc_max_quantizer = 42;

                        #[cfg(not(feature = "server"))]
                        {
                            self.cfg.rc_target_bitrate = target_bitrate as u32; //4000;
                        } // [kilobits per second]

                        #[cfg(feature = "server")]
                        {
                            self.cfg.rc_target_bitrate = target_bitrate as u32; //1000;
                        } // [kilobits per second]

                        #[cfg(feature = "server")]
                        {
                            self.cfg.rc_end_usage = vpx_rc_mode::VPX_CBR;
                        }

                        //internal frame downsampling
                        /*self.cfg.rc_resize_allowed = 1;
                        self.cfg.rc_scaled_width = self.cfg.g_w >> 2;
                        self.cfg.rc_scaled_height = self.cfg.g_h >> 2;
                        self.cfg.rc_resize_down_thresh = 30;*/

                        self.cfg.g_lag_in_frames = 0;
                        self.cfg.g_pass = vpx_enc_pass::VPX_RC_ONE_PASS;
                        self.cfg.g_threads = num_cpus::get().min(4) as u32; //set the upper limit on the number of threads to 4

                        //self.cfg.g_profile = 0 ;

                        //initialise the encoder itself
                        ret = unsafe {
                            vpx_codec_enc_init_ver(
                                &mut self.ctx,
                                vpx_codec_vp9_cx(),
                                &mut self.cfg,
                                0,
                                VPX_ENCODER_ABI_VERSION as i32,
                            )
                        };

                        if ret != VPX_CODEC_OK {
                            println!("VP9: codec init failed {:?}", ret);
                        }

                        //VP9: -8 - slower, 8 - faster
                        ret = unsafe {
                            vpx_codec_control_(
                                &mut self.ctx,
                                vp8e_enc_control_id::VP8E_SET_CPUUSED as i32,
                                6,
                            )
                        };

                        if ret != VPX_CODEC_OK {
                            println!("VP9: error setting VP8E_SET_CPUUSED {:?}", ret);
                        }

                        //start a video creation event loop
                        self.streaming = true;

                        ctx.run_later(std::time::Duration::new(0, 0), |act, _ctx| {
                            println!(
                                "video frame creation @ frame:{} ref_freq:{} seq_id:{} timestamp:{:?}; fps:{} streaming:{}",
                                act.video_frame, act.video_ref_freq, act.video_seq_id, act.video_timestamp, act.video_fps, act.streaming
                            );

                            //if act.streaming schedule the next execution
                        });
                    };
                }

                if (&text).contains("[end_video]") {
                    println!("{}", text);

                    self.streaming = false;

                    unsafe { vpx_codec_destroy(&mut self.ctx) };

                    unsafe {
                        if !self.param.is_null() {
                            x265_param_free(self.param);
                            self.param = ptr::null_mut();
                        }

                        if !self.enc.is_null() {
                            x265_encoder_close(self.enc);
                            self.enc = ptr::null_mut();
                        }

                        if !self.pic.is_null() {
                            x265_picture_free(self.pic);
                            self.pic = ptr::null_mut();
                        }
                    }
                }

                if (&text).contains("[spectrum]") {
                    //println!("{}", text.replace("&", " "));
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
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let frame_end = match frame_end {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let ref_freq = match ref_freq {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
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
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    println!("[spectrum] x1:{} y1:{} x2:{} y2:{} image:{} beam:{:?} intensity:{:?} frame_start:{} frame_end:{} ref_freq:{} seq_id:{} timestamp:{}", x1, y1, x2, y2, image, beam, intensity, frame_start, frame_end, ref_freq, seq_id, timestamp);

                    //get a read lock to the dataset
                    let datasets = DATASETS.read();

                    let fits = match datasets.get(&self.dataset_id[0]).unwrap().try_read() {
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

                    {
                        *fits.timestamp.write() = SystemTime::now();
                    }

                    if fits.has_data {
                        let start = precise_time::precise_time_ns();
                        match fits.get_spectrum(
                            x1,
                            y1,
                            x2,
                            y2,
                            beam,
                            intensity,
                            frame_start,
                            frame_end,
                            ref_freq,
                        ) {
                            Some(spectrum) => {
                                let stop = precise_time::precise_time_ns();
                                let elapsed = (stop - start) / 1000000;
                                //send a binary response message (serialize a structure to a binary stream)
                                let ws_spectrum = WsSpectrum {
                                    ts: timestamp as f32,
                                    seq_id: seq_id as u32,
                                    msg_type: 0,
                                    elapsed: elapsed as f32,
                                    spectrum: spectrum,
                                };

                                match serialize(&ws_spectrum) {
                                    Ok(bin) => {
                                        println!("binary length: {}", bin.len());
                                        //println!("{}", bin);
                                        ctx.binary(bin);
                                    }
                                    Err(err) => println!(
                                        "error serializing a WebSocket spectrum response: {}",
                                        err
                                    ),
                                }
                            }
                            None => {}
                        }

                        if image {
                            match fits.get_viewport(x1, y1, x2, y2, &self.user, self.wasm) {
                                Some((width, height, frame, alpha, identifier)) => {
                                    //send a binary response message (serialize a structure to a binary stream)
                                    let ws_viewport = WsViewport {
                                        ts: timestamp as f32,
                                        seq_id: seq_id as u32,
                                        msg_type: 1,
                                        identifier: identifier,
                                        width: width,
                                        height: height,
                                        image: frame,
                                        alpha: alpha,
                                    };

                                    match serialize(&ws_viewport) {
                                        Ok(bin) => {
                                            println!("binary length: {}", bin.len());
                                            //println!("{}", bin);
                                            ctx.binary(bin);
                                        }
                                        Err(err) => println!(
                                            "error serializing a WebSocket viewport response: {}",
                                            err
                                        ),
                                    }
                                }
                                None => {}
                            }
                        }
                    };
                }

                if (&text).contains("[image]") {
                    //println!("{}", text.replace("&", " "));
                    let (black, white, median, noise, flux, frame_start, frame_end, ref_freq, hist, timestamp) = scan_fmt!(&text.replace("&"," "), "[image] black={} white={} median={} noise={} flux={} frame_start={} frame_end={} ref_freq={} hist={} timestamp={}", String, String, String, String, String, String, String, String, bool, String);

                    let black = match black {
                        Some(s) => match s.parse::<f32>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let white = match white {
                        Some(s) => match s.parse::<f32>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let median = match median {
                        Some(s) => match s.parse::<f32>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let noise = match noise {
                        Some(s) => match s.replace("x", "").parse::<f32>() {
                            Ok(x) => x,
                            Err(_) => 1.0,
                        },
                        _ => 1.0,
                    };

                    let flux = match flux {
                        Some(s) => s,
                        _ => String::from("logistic"),
                    };

                    let frame_start = match frame_start {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let frame_end = match frame_end {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let ref_freq = match ref_freq {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let mut refresh_image = match hist {
                        Some(x) => x,
                        _ => false,
                    };

                    let timestamp = match timestamp {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    println!("[image] black:{} white:{} median:{} noise:{} flux:{} frame_start:{} frame_end:{} ref_freq:{} hist:{} timestamp:{}", black, white, median, noise, flux, frame_start, frame_end, ref_freq, refresh_image, timestamp);

                    let datasets = DATASETS.read();

                    let fits = match datasets.get(&self.dataset_id[0]).unwrap().try_read() {
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

                    {
                        *fits.timestamp.write() = SystemTime::now();
                    }

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

                        let (start, end) =
                            match fits.get_spectrum_range(frame_start, frame_end, ref_freq) {
                                Some(frame) => frame,
                                None => {
                                    println!("error: an invalid spectrum range");
                                    return;
                                }
                            };

                        //check if a user param structure exists
                        match self.user {
                            Some(ref mut user) => {
                                if start != user.start || end != user.end {
                                    refresh_image = true;
                                }

                                //update the user session
                                user.black = black;
                                user.white = white;
                                user.median = median;
                                user.sensitivity = noise * fits.sensitivity;
                                user.ratio_sensitivity = noise * fits.ratio_sensitivity;
                                user.flux = flux.clone();
                                user.start = start;
                                user.end = end;

                                if flux == "legacy" {
                                    //recalculate lmin, lmax; change pmin, pmax to black, white in a call to pixels_to_luminance
                                    let xmin = 0.01f32;
                                    let xmax = 100.0f32;
                                    let pmin = 0.001f32;
                                    let pmax = 0.5f32;

                                    let p = pmin + (pmax - pmin) * (noise - xmin) / (xmax - xmin);

                                    user.lmin = p.ln();
                                    user.lmax = (p + 1.0).ln();

                                    user.pmin = black;
                                    user.pmax = white;
                                };
                            }
                            None => {
                                if start != 0 || end != fits.depth - 1 {
                                    refresh_image = true;
                                }

                                let (pmin, pmax, lmin, lmax) = if flux == "legacy" {
                                    //recalculate lmin, lmax; change pmin, pmax to black, white in a call to pixels_to_luminance
                                    let xmin = 0.01f32;
                                    let xmax = 100.0f32;
                                    let pmin = 0.001f32;
                                    let pmax = 0.5f32;

                                    let p = pmin + (pmax - pmin) * (noise - xmin) / (xmax - xmin);

                                    let lmin = p.ln();
                                    let lmax = (p + 1.0).ln();

                                    let pmin = black;
                                    let pmax = white;

                                    (pmin, pmax, lmin, lmax)
                                } else {
                                    (fits.pmin, fits.pmax, fits.lmin, fits.lmax)
                                };

                                //create a new user parameter set
                                self.user = Some(UserParams {
                                    pmin: pmin,
                                    pmax: pmax,
                                    lmin: lmin,
                                    lmax: lmax,
                                    black: black,
                                    white: white,
                                    median: median,
                                    sensitivity: noise * fits.sensitivity,
                                    ratio_sensitivity: noise * fits.ratio_sensitivity,
                                    flux: flux.clone(),
                                    start: start,
                                    end: end,
                                    mask: fits.mask.clone(),
                                    pixels: fits.pixels.clone(),
                                });
                            }
                        }

                        println!("[image] refresh_histogram: {}", refresh_image);

                        match self.user {
                            Some(ref mut user) => {
                                if refresh_image {
                                    //regenerate pixels and mask
                                    match fits.make_image_spectrum(start, end) {
                                        Some((
                                            pixels,
                                            mask,
                                            mean_spectrum,
                                            integrated_spectrum,
                                        )) => {
                                            //get ord_pixels
                                            //apply std::f32::NAN to masked pixels
                                            let mut ord_pixels: Vec<f32> = pixels
                                                .par_iter()
                                                .zip(mask.par_iter())
                                                .map(
                                                    |(x, m)| {
                                                        if *m > 0 {
                                                            *x
                                                        } else {
                                                            std::f32::NAN
                                                        }
                                                    },
                                                )
                                                .collect();

                                            //let mut ord_pixels = pixels.clone();

                                            /*ord_pixels.par_sort_unstable_by(|a, b| {
                                                a.partial_cmp(b).unwrap_or(Equal)
                                            });*/
                                            ord_pixels.par_sort_unstable_by(|a, b| {
                                                if a.is_finite() && b.is_finite() {
                                                    a.partial_cmp(b).unwrap_or(Equal)
                                                } else {
                                                    if a.is_finite() {
                                                        std::cmp::Ordering::Less
                                                    } else {
                                                        if b.is_finite() {
                                                            std::cmp::Ordering::Greater
                                                        } else {
                                                            std::cmp::Ordering::Equal
                                                        }
                                                    }
                                                }
                                            });

                                            match fits.get_image_histogram(
                                                &ord_pixels,
                                                &pixels,
                                                &mask,
                                            ) {
                                                Some((
                                                    hist,
                                                    pmin,
                                                    pmax,
                                                    black,
                                                    white,
                                                    median,
                                                    sensitivity,
                                                    ratio_sensitivity,
                                                )) => {
                                                    user.pmin = pmin;
                                                    user.pmax = pmax;
                                                    user.black = black;
                                                    user.white = white;
                                                    user.median = median;
                                                    user.sensitivity = sensitivity;
                                                    user.ratio_sensitivity = ratio_sensitivity;
                                                    user.pixels = pixels;
                                                    user.mask = mask;

                                                    //and then

                                                    //send a spectra refresh
                                                    //send a binary response message (serialize a structure to a binary stream)
                                                    let ws_spectra = WsSpectra {
                                                        ts: timestamp as f32,
                                                        seq_id: 0,
                                                        msg_type: 3,
                                                        mean_spectrum: mean_spectrum,
                                                        integrated_spectrum: integrated_spectrum,
                                                    };

                                                    match serialize(&ws_spectra) {
                                                        Ok(bin) => {
                                                            println!(
                                                                "binary length: {}",
                                                                bin.len()
                                                            );
                                                            //println!("{}", bin);
                                                            ctx.binary(bin);
                                                        }
                                                        Err(err) => println!(
                                        "error serializing a WebSocket spectra response: {}",
                                        err
                                    ),
                                                    }

                                                    //send a histogram refresh
                                                    //send a binary response message (serialize a structure to a binary stream)
                                                    let ws_histogram = WsHistogram {
                                                        ts: timestamp as f32,
                                                        seq_id: 0,
                                                        msg_type: 4,
                                                        pmin: pmin,
                                                        pmax: pmax,
                                                        black: black,
                                                        white: white,
                                                        median: median,
                                                        sensitivity: sensitivity,
                                                        ratio_sensitivity: ratio_sensitivity,
                                                        hist: hist,
                                                    };

                                                    match serialize(&ws_histogram) {
                                                        Ok(bin) => {
                                                            println!(
                                                                "binary length: {}",
                                                                bin.len()
                                                            );
                                                            //println!("{}", bin);
                                                            ctx.binary(bin);
                                                        }
                                                        Err(err) => println!(
                                        "error serializing a WebSocket histogram response: {}",
                                        err
                                    ),
                                                    }
                                                }
                                                None => {}
                                            }
                                        }
                                        None => {
                                            println!("[image] make_image_spectrum returned None")
                                        }
                                    }
                                }

                                //get a VP9 keyframe
                                let mut image_frame: Vec<u8> = Vec::new();

                                let mut w = fits.width as u32;
                                let mut h = fits.height as u32;
                                let pixel_count = (w as u64) * (h as u64);

                                if pixel_count > fits::IMAGE_PIXEL_COUNT_LIMIT {
                                    let ratio: f32 = ((pixel_count as f32)
                                        / (fits::IMAGE_PIXEL_COUNT_LIMIT as f32))
                                        .sqrt();

                                    if ratio > 4.5 {
                                        //default scaling, no optimisations
                                        w = ((w as f32) / ratio) as u32;
                                        h = ((h as f32) / ratio) as u32;

                                        println!(
                    "downscaling the image from {}x{} to {}x{}, default ratio: {}",
                    fits.width, fits.height, w, h, ratio
                );
                                    } else if ratio > 3.0 {
                                        // 1/4
                                        w = w / 4;
                                        h = h / 4;

                                        println!(
                                            "downscaling the image from {}x{} to {}x{} (1/4)",
                                            fits.width, fits.height, w, h
                                        );
                                    } else if ratio > 2.25 {
                                        // 3/8
                                        w = 3 * w / 8;
                                        h = (h * 3 + 7) / 8;

                                        println!(
                                            "downscaling the image from {}x{} to {}x{} (3/8)",
                                            fits.width, fits.height, w, h
                                        );
                                    } else if ratio > 1.5 {
                                        // 1/2
                                        w = w / 2;
                                        h = h / 2;

                                        println!(
                                            "downscaling the image from {}x{} to {}x{} (1/2)",
                                            fits.width, fits.height, w, h
                                        );
                                    } else if ratio > 1.0 {
                                        // 3/4
                                        w = 3 * w / 4;
                                        h = 3 * h / 4;

                                        println!(
                                            "downscaling the image from {}x{} to {}x{} (3/4)",
                                            fits.width, fits.height, w, h
                                        );
                                    }
                                }

                                let mut raw: vpx_image = vpx_image::default();
                                let mut vpx_ctx = vpx_codec_ctx_t {
                                    name: ptr::null(),
                                    iface: ptr::null_mut(),
                                    err: VPX_CODEC_ERROR,
                                    err_detail: ptr::null(),
                                    init_flags: 0,
                                    config: vpx_codec_ctx__bindgen_ty_1 { enc: ptr::null() },
                                    priv_: ptr::null_mut(),
                                };

                                let align = 1;

                                //a workaround around a bug in libvpx triggered when h > w
                                let ret = if w > h {
                                    unsafe {
                                        vpx_img_alloc(
                                            &mut raw,
                                            vpx_img_fmt::VPX_IMG_FMT_I420,
                                            w,
                                            h,
                                            align,
                                        )
                                    } //I420 or I444
                                } else {
                                    unsafe {
                                        vpx_img_alloc(
                                            &mut raw,
                                            vpx_img_fmt::VPX_IMG_FMT_I420,
                                            h,
                                            w,
                                            align,
                                        )
                                    } //I420 or I444
                                };

                                if ret.is_null() {
                                    println!("VP9 image frame error: image allocation failed");
                                    return;
                                }
                                mem::forget(ret); // img and ret are the same
                                print!("{:#?}", raw);

                                //redo the image based on new user parameters, pixels and mask
                                let mut y = fits.pixels_to_luminance(
                                    &user.pixels,
                                    &user.mask,
                                    user.pmin,
                                    user.pmax,
                                    user.lmin,
                                    user.lmax,
                                    user.black,
                                    user.white,
                                    user.median,
                                    user.sensitivity,
                                    user.ratio_sensitivity,
                                    &user.flux,
                                );

                                {
                                    let start = precise_time::precise_time_ns();

                                    let mut dst = vec![0; (w as usize) * (h as usize)];
                                    fits.resize_and_invert(
                                        &y,
                                        &mut dst,
                                        w,
                                        h,
                                        libyuv_FilterMode_kFilterBox,
                                    );
                                    y = dst;

                                    let stop = precise_time::precise_time_ns();

                                    println!(
                                        "VP9 image frame inverting/downscaling time: {} [ms]",
                                        (stop - start) / 1000000
                                    );
                                }

                                let alpha_frame = {
                                    let start = precise_time::precise_time_ns();

                                    //invert/downscale the mask (alpha channel) without interpolation
                                    let mut alpha = vec![0; (w as usize) * (h as usize)];

                                    fits.resize_and_invert(
                                        &user.mask,
                                        &mut alpha,
                                        w,
                                        h,
                                        libyuv_FilterMode_kFilterNone,
                                    );

                                    let compressed_alpha = lz4_compress::compress(&alpha);

                                    let stop = precise_time::precise_time_ns();

                                    println!(
                "alpha original length {}, lz4-compressed {} bytes, elapsed time {} [ms]",
                alpha.len(),
                compressed_alpha.len(),
                (stop - start) / 1000000
            );

                                    compressed_alpha
                                };

                                //I420
                                let stride_u = raw.stride[1];
                                let stride_v = raw.stride[2];
                                let count = stride_u * stride_v;

                                let u: &[u8] = &vec![128; count as usize];
                                let v: &[u8] = &vec![128; count as usize];

                                raw.planes[0] = unsafe { mem::transmute(y.as_ptr()) };
                                raw.planes[1] = unsafe { mem::transmute(u.as_ptr()) };
                                raw.planes[2] = unsafe { mem::transmute(v.as_ptr()) };

                                //a workaround around a bug in libvpx triggered when h > w
                                raw.stride[0] = if w > h { w as i32 } else { h as i32 };

                                let mut cfg = vpx_codec_enc_cfg::default();
                                let mut ret = unsafe {
                                    vpx_codec_enc_config_default(vpx_codec_vp9_cx(), &mut cfg, 0)
                                };

                                if ret != VPX_CODEC_OK {
                                    println!("VP9 image frame error: default Configuration failed");

                                    //release the image
                                    unsafe { vpx_img_free(&mut raw) };

                                    return;
                                }

                                //a workaround around a bug in libvpx triggered when h > w
                                if w > h {
                                    cfg.g_w = w;
                                    cfg.g_h = h;
                                } else {
                                    cfg.g_w = h;
                                    cfg.g_h = w;
                                }

                                cfg.rc_min_quantizer = 10;
                                cfg.rc_max_quantizer = 42;
                                cfg.rc_target_bitrate = 4096; // [kilobits per second]
                                cfg.g_pass = vpx_enc_pass::VPX_RC_ONE_PASS;
                                cfg.g_threads = num_cpus::get().min(4) as u32; //set the upper limit on the number of threads to 4

                                ret = unsafe {
                                    vpx_codec_enc_init_ver(
                                        &mut vpx_ctx,
                                        vpx_codec_vp9_cx(),
                                        &mut cfg,
                                        0,
                                        VPX_ENCODER_ABI_VERSION as i32,
                                    )
                                };

                                if ret != VPX_CODEC_OK {
                                    println!("VP9 image frame error: codec init failed {:?}", ret);

                                    unsafe { vpx_img_free(&mut raw) };

                                    return;
                                }

                                ret = unsafe {
                                    vpx_codec_control_(
                                        &mut vpx_ctx,
                                        vp8e_enc_control_id::VP8E_SET_CPUUSED as i32,
                                        8,
                                    )
                                };

                                if ret != VPX_CODEC_OK {
                                    println!("VP9: error setting VP8E_SET_CPUUSED {:?}", ret);
                                }

                                let mut flags = 0;
                                flags |= VPX_EFLAG_FORCE_KF;

                                //call encode_frame with a valid image
                                match fits::encode_frame(
                                    vpx_ctx,
                                    raw,
                                    0,
                                    flags as i64,
                                    VPX_DL_BEST_QUALITY as u64,
                                ) {
                                    Ok(res) => match res {
                                        Some(res) => image_frame = res,
                                        _ => {}
                                    },
                                    Err(err) => {
                                        println!("codec error: {:?}", err);

                                        unsafe { vpx_img_free(&mut raw) };
                                        unsafe { vpx_codec_destroy(&mut vpx_ctx) };

                                        return;
                                    }
                                };

                                //flush the encoder to signal the end
                                match fits::flush_frame(vpx_ctx, VPX_DL_BEST_QUALITY as u64) {
                                    Ok(res) => match res {
                                        Some(res) => image_frame = res,
                                        _ => {}
                                    },
                                    Err(err) => {
                                        println!("codec error: {:?}", err);

                                        unsafe { vpx_img_free(&mut raw) };
                                        unsafe { vpx_codec_destroy(&mut vpx_ctx) };

                                        return;
                                    }
                                };

                                if image_frame.is_empty() {
                                    println!("VP9 image frame error: no image packet produced");

                                    unsafe { vpx_img_free(&mut raw) };

                                    unsafe { vpx_codec_destroy(&mut vpx_ctx) };

                                    return;
                                }

                                unsafe { vpx_img_free(&mut raw) };
                                unsafe { vpx_codec_destroy(&mut vpx_ctx) };

                                //(...) we have a VP9 frame, send to as WsImage
                                //send a binary response message (serialize a structure to a binary stream)
                                let ws_image = WsImage {
                                    ts: timestamp as f32,
                                    seq_id: 0,
                                    msg_type: 2,
                                    identifier: String::from("VP9"),
                                    width: w,
                                    height: h,
                                    image: image_frame,
                                    alpha: alpha_frame,
                                };

                                match serialize(&ws_image) {
                                    Ok(bin) => {
                                        println!("binary length: {}", bin.len());
                                        //println!("{}", bin);
                                        ctx.binary(bin);
                                    }
                                    Err(err) => println!(
                                        "error serializing a WebSocket image response: {}",
                                        err
                                    ),
                                }
                            }
                            None => {}
                        }
                    };
                }

                if (&text).contains("[video]") {
                    //println!("{}", text.replace("&"," "));
                    let (frame, key, view, ref_freq, fps, seq_id, target_bitrate, timestamp) = scan_fmt!(
                        &text.replace("&", " "),
                        "[video] frame={} key={} view={} ref_freq={} fps={} seq_id={} bitrate={} timestamp={}",
                        String,
                        bool,
                        String,
                        String,
                        String,
                        i32,
                        i32,
                        String
                    );

                    let frame = match frame {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let is_composite = match view {
                        Some(s) => {
                            if s.contains("composite") {
                                true
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };

                    let ref_freq = match ref_freq {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    let keyframe = match key {
                        Some(x) => x,
                        _ => false,
                    };

                    //use 10 frames per second by default
                    let fps = match fps {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 10.0,
                        },
                        _ => 10.0,
                    };

                    let seq_id = match seq_id {
                        Some(x) => x,
                        _ => 0,
                    };

                    let target_bitrate = match target_bitrate {
                        Some(x) => num::clamp(x, 100, 10000),
                        _ => 1000,
                    };

                    let timestamp = match timestamp {
                        Some(s) => match s.parse::<f64>() {
                            Ok(x) => x,
                            Err(_) => 0.0,
                        },
                        _ => 0.0,
                    };

                    println!(
                        "[video] frame:{} keyframe:{} is_composite:{} ref_freq:{} fps:{} seq_id:{} target_bitrate:{} timestamp:{}",
                        frame, keyframe, is_composite, ref_freq, fps, seq_id, target_bitrate, timestamp
                    );

                    //let deltat = timestamp - self.video_timestamp;
                    let deltat = std::time::Instant::now().duration_since(self.video_timestamp);
                    let deltat = deltat.as_secs() as f64 + deltat.subsec_nanos() as f64 * 1e-9;
                    self.kf.update(frame, deltat); //need to convert ms to s?

                    if keyframe {
                        self.kf.reset(frame);
                    }

                    let predicted_frame = self.kf.predict(frame, deltat);

                    println!(
                        "Kalman Filter: {:?}; current = {}, predicted = {}",
                        self.kf, frame, predicted_frame
                    );

                    self.video_frame = frame;
                    self.video_ref_freq = ref_freq;
                    self.video_fps = fps;
                    self.video_seq_id = seq_id;
                    self.video_timestamp = std::time::Instant::now(); //timestamp;
                    self.bitrate = target_bitrate;

                    let frame = predicted_frame;

                    if self.dataset_id.len() == 1 || !is_composite {
                        let datasets = DATASETS.read();

                        let fits = match datasets.get(&self.dataset_id[0]).unwrap().try_read() {
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

                        {
                            *fits.timestamp.write() = SystemTime::now();
                        }

                        if fits.has_data && !self.pic.is_null() {
                            let frame_index = match fits.get_spectrum_range(frame, frame, ref_freq)
                            {
                                Some((frame, _)) => frame,
                                None => {
                                    println!("error: an invalid spectrum range");
                                    return;
                                }
                            };

                            if self.last_video_frame == (frame_index as i32) && !keyframe {
                                println!("skipping a video frame");
                                return;
                            }

                            self.last_video_frame = frame_index as i32;

                            let start = precise_time::precise_time_ns();

                            let flux = match self.user {
                                Some(ref user) => user.flux.clone(),
                                None => fits.flux.clone(),
                            };

                            //HEVC (x265)
                            #[cfg(feature = "hevc")]
                            match fits.get_video_frame(frame_index, self.width, self.height, &flux)
                            {
                                Some(mut y) => {
                                    unsafe {
                                        (*self.pic).stride[0] = self.width as i32;
                                        (*self.pic).planes[0] =
                                            y.as_mut_ptr() as *mut std::os::raw::c_void;

                                        //adaptive bitrate
                                        (*self.param).rc.bitrate = target_bitrate;
                                    }

                                    let ret =
                                        unsafe { x265_encoder_reconfig(self.enc, self.param) };

                                    if ret < 0 {
                                        println!("x265: error changing the bitrate");
                                    }

                                    let mut nal_count: u32 = 0;
                                    let mut p_nal: *mut x265_nal = ptr::null_mut();

                                    //encode
                                    let ret = unsafe {
                                        x265_encoder_encode(
                                            self.enc,
                                            &mut p_nal,
                                            &mut nal_count,
                                            self.pic,
                                            ptr::null_mut(),
                                        )
                                    };

                                    let stop = precise_time::precise_time_ns();

                                    println!("x265 hevc video frame prepare/encode time: {} [ms], speed {} frames per second, ret = {}, nal_count = {}", (stop-start)/1000000, 1000000000/(stop-start), ret, nal_count);

                                    //y falls out of scope
                                    unsafe {
                                        (*self.pic).stride[0] = 0 as i32;
                                        (*self.pic).planes[0] = ptr::null_mut();
                                    }

                                    let elapsed = (stop - start) / 1000000;

                                    //process all NAL units one by one
                                    if nal_count > 0 {
                                        let nal_units = unsafe {
                                            std::slice::from_raw_parts(p_nal, nal_count as usize)
                                        };

                                        for unit in nal_units {
                                            println!(
                                                "NAL unit type: {}, size: {}",
                                                unit.type_, unit.sizeBytes
                                            );

                                            let payload = unsafe {
                                                std::slice::from_raw_parts(
                                                    unit.payload,
                                                    unit.sizeBytes as usize,
                                                )
                                            };

                                            let ws_frame = WsFrame {
                                                ts: timestamp as f32,
                                                seq_id: seq_id as u32,
                                                msg_type: 5, //an hevc video frame
                                                //length: video_frame.len() as u32,
                                                elapsed: elapsed as f32,
                                                frame: payload.to_vec(),
                                            };

                                            match serialize(&ws_frame) {
                                                Ok(bin) => {
                                                    println!("WsFrame binary length: {}", bin.len());
                                                    //println!("{}", bin);
                                                    ctx.binary(bin);
                                                },
                                                Err(err) => println!("error serializing a WebSocket video frame response: {}", err)
                                            }

                                            /*match self.hevc {
                                                Ok(ref mut file) => {
                                                    let _ = file.write_all(payload);
                                                }
                                                Err(_) => {}
                                            }*/
                                        }
                                    }

                                    /*if keyframe {
                                    //flush the encoder to signal the end
                                    loop {
                                        let ret = unsafe {
                                            x265_encoder_encode(
                                                self.enc,
                                                &mut p_nal,
                                                &mut nal_count,
                                                ptr::null_mut(),
                                                ptr::null_mut(),
                                            )
                                        };

                                        if ret > 0 {
                                            println!(
                                                "flushing the encoder, residual nal_count = {}",
                                                nal_count
                                            );

                                            let nal_units = unsafe {
                                                std::slice::from_raw_parts(
                                                    p_nal,
                                                    nal_count as usize,
                                                )
                                            };

                                            for unit in nal_units {
                                                println!(
                                                    "NAL unit type: {}, size: {}",
                                                    unit.type_, unit.sizeBytes
                                                );

                                                let payload = unsafe {
                                                    std::slice::from_raw_parts(
                                                        unit.payload,
                                                        unit.sizeBytes as usize,
                                                    )
                                                };

                                                let ws_frame = WsFrame {
                                                    ts: timestamp as f32,
                                                    seq_id: seq_id as u32,
                                                    msg_type: 5, //an hevc video frame
                                                    //length: video_frame.len() as u32,
                                                    elapsed: elapsed as f32,
                                                    frame: payload.to_vec(),
                                                };

                                                match serialize(&ws_frame) {
                                                    Ok(bin) => {
                                                    println!("WsFrame binary length: {}", bin.len());
                                                    //println!("{}", bin);
                                                    ctx.binary(bin);
                                                    },
                                                    Err(err) => println!("error serializing a WebSocket video frame response: {}", err)
                                                }

                                                /*match self.hevc {
                                                    Ok(ref mut file) => {
                                                        let _ = file.write_all(payload);
                                                    }
                                                    Err(_) => {}
                                                }*/
                                    }
                                    } else {
                                    break;
                                    }
                                    }
                                    }*/
                                }
                                None => {}
                            }

                            //VP9 (libvpx)
                            #[cfg(feature = "vp9")]
                            match fits.get_vpx_frame(
                                frame,
                                ref_freq,
                                self.width,
                                self.height,
                                &flux,
                            ) {
                                Some(mut image) => {
                                    //serialize a video response with seq_id, timestamp
                                    //send a binary response
                                    //print!("{:#?}", image);

                                    //let start = precise_time::precise_time_ns();

                                    //variable rate control
                                    //disabled due to bugs in libvpx, needs to be tested again and again
                                    self.cfg.rc_target_bitrate = target_bitrate as u32;

                                    let ret = unsafe {
                                        vpx_codec_enc_config_set(&mut self.ctx, &mut self.cfg)
                                    };

                                    if ret != VPX_CODEC_OK {
                                        println!("VP9: vpx_codec_enc_config_set error {:?}", ret);
                                    }

                                    let mut flags = 0;
                                    if keyframe {
                                        flags |= VPX_EFLAG_FORCE_KF;
                                    };

                                    //call encode_frame with a valid frame image
                                    let mut video_frame: Vec<u8> = Vec::new();

                                    match fits::encode_frame(
                                        self.ctx,
                                        image,
                                        0,
                                        flags as i64,
                                        VPX_DL_REALTIME as u64,
                                    ) {
                                        Ok(res) => match res {
                                            Some(res) => video_frame = res,
                                            _ => {}
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
                                                _ => {}
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
                                            msg_type: 5, //a VP9 video frame
                                            //length: video_frame.len() as u32,
                                            elapsed: ((stop - start) / 1000000) as f32,
                                            frame: video_frame,
                                        };

                                        match serialize(&ws_frame) {
                                            Ok(bin) => {
                                                println!("WsFrame binary length: {}", bin.len());
                                                //println!("{}", bin);
                                                ctx.binary(bin);
                                            },
                                            Err(err) => println!("error serializing a WebSocket video frame response: {}", err)
                                        }
                                    }
                                }
                                None => {}
                            };
                        }
                    } else {
                        let start = precise_time::precise_time_ns();
                        let width = self.width;
                        let height = self.height;

                        let flux = match self.user {
                            Some(ref user) => Some(user.flux.clone()),
                            None => None,
                        };

                        let mut planes: Vec<_> = self
                            .dataset_id
                            .clone()
                            .par_iter()
                            .map(|ref dataset_id| {
                                let datasets = DATASETS.read();

                                let fits = match datasets.get(*dataset_id).unwrap().try_read() {
                                    Some(x) => x,
                                    None => {
                                        println!(
                                            "[video] error getting {} from DATASETS; aborting",
                                            dataset_id
                                        );
                                        return vec![0; (width * height) as usize];
                                    }
                                };

                                {
                                    *fits.timestamp.write() = SystemTime::now();
                                }

                                if fits.has_data {
                                    let flux = match flux {
                                        Some(ref flux) => flux.clone(),
                                        None => fits.flux.clone(),
                                    };

                                    match fits.get_spectrum_range(frame, frame, ref_freq) {
                                        Some((frame_index, _)) => match fits.get_video_frame(
                                            frame_index,
                                            width,
                                            height,
                                            &flux,
                                        ) {
                                            Some(y) => y,
                                            None => vec![0; (width * height) as usize],
                                        },
                                        None => {
                                            println!("error: an invalid spectrum range");
                                            vec![0; (width * height) as usize]
                                        }
                                    }
                                } else {
                                    vec![0; (width * height) as usize]
                                }
                            })
                            .collect();

                        //HEVC (x265)
                        #[cfg(feature = "hevc")]
                        {
                            if !self.pic.is_null() {
                                //convert planes:RGB to planes:YUV (TODO!)
                                /*let total_size = width * height ;
                                unsafe {
                                    ispc_rgb_to_yuv(planes[0].as_mut_ptr(), planes[1].as_mut_ptr(), planes[2].as_mut_ptr(), total_size);
                                }*/

                                let mut dummy = vec![0; (width * height) as usize];

                                for i in 0..3 {
                                    unsafe {
                                        (*self.pic).stride[i] = width as i32;
                                        (*self.pic).planes[i] =
                                            dummy.as_mut_ptr() as *mut std::os::raw::c_void;
                                    }
                                }

                                //setup the I444 picture (max 3 channels)
                                for i in 0..planes.len().min(3) {
                                    unsafe {
                                        (*self.pic).stride[i] = width as i32;
                                        (*self.pic).planes[i] =
                                            planes[i].as_mut_ptr() as *mut std::os::raw::c_void;
                                    }
                                }

                                unsafe {
                                    //adaptive bitrate
                                    (*self.param).rc.bitrate = target_bitrate;
                                }

                                let ret = unsafe { x265_encoder_reconfig(self.enc, self.param) };

                                if ret < 0 {
                                    println!("x265: error changing the bitrate");
                                }

                                let mut nal_count: u32 = 0;
                                let mut p_nal: *mut x265_nal = ptr::null_mut();

                                //encode
                                let ret = unsafe {
                                    x265_encoder_encode(
                                        self.enc,
                                        &mut p_nal,
                                        &mut nal_count,
                                        self.pic,
                                        ptr::null_mut(),
                                    )
                                };

                                let stop = precise_time::precise_time_ns();

                                println!("x265 hevc video frame prepare/encode time: {} [ms], speed {} frames per second, ret = {}, nal_count = {}", (stop-start)/1000000, 1000000000/(stop-start), ret, nal_count);

                                //yuv planes fall out of scope
                                for i in 0..3 {
                                    unsafe {
                                        (*self.pic).stride[i] = 0 as i32;
                                        (*self.pic).planes[i] = ptr::null_mut();
                                    }
                                }

                                let elapsed = (stop - start) / 1000000;

                                //process all NAL units one by one
                                if nal_count > 0 {
                                    let nal_units = unsafe {
                                        std::slice::from_raw_parts(p_nal, nal_count as usize)
                                    };

                                    for unit in nal_units {
                                        println!(
                                            "NAL unit type: {}, size: {}",
                                            unit.type_, unit.sizeBytes
                                        );

                                        let payload = unsafe {
                                            std::slice::from_raw_parts(
                                                unit.payload,
                                                unit.sizeBytes as usize,
                                            )
                                        };

                                        let ws_frame = WsFrame {
                                            ts: timestamp as f32,
                                            seq_id: seq_id as u32,
                                            msg_type: 5, //an hevc video frame
                                            //length: video_frame.len() as u32,
                                            elapsed: elapsed as f32,
                                            frame: payload.to_vec(),
                                        };

                                        match serialize(&ws_frame) {
                                            Ok(bin) => {
                                                println!("WsFrame binary length: {}", bin.len());
                                                //println!("{}", bin);
                                                ctx.binary(bin);
                                            },
                                            Err(err) => println!("error serializing a WebSocket video frame response: {}", err)
                                        }

                                        /*match self.hevc {
                                            Ok(ref mut file) => {
                                                let _ = file.write_all(payload);
                                            }
                                            Err(_) => {}
                                        }*/
                                    }
                                }

                                /*if keyframe {
                                //flush the encoder to signal the end
                                loop {
                                    let ret = unsafe {
                                        x265_encoder_encode(
                                            self.enc,
                                            &mut p_nal,
                                            &mut nal_count,
                                            ptr::null_mut(),
                                            ptr::null_mut(),
                                        )
                                    };

                                    if ret > 0 {
                                        println!(
                                            "flushing the encoder, residual nal_count = {}",
                                            nal_count
                                        );

                                        let nal_units = unsafe {
                                            std::slice::from_raw_parts(
                                                p_nal,
                                                nal_count as usize,
                                            )
                                        };

                                        for unit in nal_units {
                                            println!(
                                                "NAL unit type: {}, size: {}",
                                                unit.type_, unit.sizeBytes
                                            );

                                            let payload = unsafe {
                                                std::slice::from_raw_parts(
                                                    unit.payload,
                                                    unit.sizeBytes as usize,
                                                )
                                            };

                                            let ws_frame = WsFrame {
                                                ts: timestamp as f32,
                                                seq_id: seq_id as u32,
                                                msg_type: 5, //an hevc video frame
                                                //length: video_frame.len() as u32,
                                                elapsed: elapsed as f32,
                                                frame: payload.to_vec(),
                                            };

                                            match serialize(&ws_frame) {
                                                Ok(bin) => {
                                                println!("WsFrame binary length: {}", bin.len());
                                                //println!("{}", bin);
                                                ctx.binary(bin);
                                                },
                                                Err(err) => println!("error serializing a WebSocket video frame response: {}", err)
                                            }

                                            /*match self.hevc {
                                                Ok(ref mut file) => {
                                                    let _ = file.write_all(payload);
                                                }
                                                Err(_) => {}
                                            }*/
                                }
                                } else {
                                break;
                                }
                                }
                                }*/
                            }
                        }
                    }
                }
            }
            ws::Message::Binary(_) => println!("ignoring an incoming binary websocket message"),
            _ => ctx.stop(),
        }
    }
}

//FITS datasets
lazy_static! {
    static ref DATASETS: Arc<RwLock<HashMap<String, Arc<RwLock<Box<fits::FITS>>>>>> =
        { Arc::new(RwLock::new(HashMap::new())) };
}

#[cfg(feature = "server")]
static LOG_DIRECTORY: &'static str = "LOGS";

static SERVER_STRING: &'static str = "FITSWebQL v4.1.5";
static VERSION_STRING: &'static str = "SV2019-02-09.1";
static WASM_STRING: &'static str = "WASM2019-02-08.1";

#[cfg(not(feature = "server"))]
static SERVER_MODE: &'static str = "LOCAL";

#[cfg(feature = "server")]
static SERVER_MODE: &'static str = "SERVER";

#[cfg(not(feature = "server"))]
const SERVER_ADDRESS: &'static str = "localhost";

#[cfg(feature = "server")]
const SERVER_ADDRESS: &'static str = "0.0.0.0";

#[cfg(feature = "server")]
const JVO_USER: &'static str = "jvo";

#[cfg(feature = "server")]
const JVO_HOST: &'static str = "localhost";

const SERVER_PORT: i32 = 8080;
const SERVER_PATH: &'static str = "fitswebql";

const WEBSOCKET_TIMEOUT: u64 = 60 * 60; //[s]; a websocket inactivity timeout

//const LONG_POLL_TIMEOUT: u64 = 100;//[ms]; keep it short, long intervals will block the actix event loop

fn stream_molecules(freq_start: f64, freq_end: f64) -> Option<mpsc::Receiver<Molecule>> {
    //splatalogue sqlite db integration

    let (stream_tx, stream_rx): (mpsc::Sender<Molecule>, mpsc::Receiver<Molecule>) =
        mpsc::channel();

    thread::spawn(move || {
        let splat_path = std::path::Path::new("splatalogue_v3.db");

        match rusqlite::Connection::open(splat_path) {
            Ok(splat_db) => {
                println!("[stream_molecules] connected to splatalogue sqlite");

                match splat_db.prepare(&format!(
                    "SELECT * FROM lines WHERE frequency>={} AND frequency<={};",
                    freq_start, freq_end
                )) {
                    Ok(mut stmt) => {
                        let molecule_iter = stmt
                            .query_map(rusqlite::NO_PARAMS, |row| Molecule::from_sqlite_row(row))
                            .unwrap();

                        for molecule in molecule_iter {
                            //println!("molecule {:?}", molecule.unwrap());
                            let mol = molecule.unwrap();

                            match stream_tx.send(mol) {
                                Ok(()) => {}
                                Err(err) => {
                                    println!("CRITICAL ERROR sending a molecule: {}", err);
                                    return;
                                }
                            }
                        }
                    }
                    Err(err) => println!("sqlite prepare error: {}", err),
                }
            }
            Err(err) => {
                println!("error connecting to splatalogue sqlite: {}", err);
            }
        };
    });

    Some(stream_rx)
}

fn _fetch_molecules(freq_start: f64, freq_end: f64) -> String {
    //splatalogue sqlite db integration
    let mut molecules: Vec<serde_json::Value> = Vec::new();

    let splat_path = std::path::Path::new("splatalogue_v3.db");

    match rusqlite::Connection::open(splat_path) {
        Ok(splat_db) => {
            println!("[fetch_molecules] connected to splatalogue sqlite");

            match splat_db.prepare(&format!(
                "SELECT * FROM lines WHERE frequency>={} AND frequency<={};",
                freq_start, freq_end
            )) {
                Ok(mut stmt) => {
                    let molecule_iter = stmt
                        .query_map(rusqlite::NO_PARAMS, |row| Molecule::from_sqlite_row(row))
                        .unwrap();

                    for molecule in molecule_iter {
                        //println!("molecule {:?}", molecule.unwrap());
                        let mol = molecule.unwrap();
                        molecules.push(mol.to_json());
                    }
                }
                Err(err) => println!("sqlite prepare error: {}", err),
            }
        }
        Err(err) => {
            println!("error connecting to splatalogue sqlite: {}", err);
        }
    };

    let mut contents = String::from("[");

    for entry in &molecules {
        contents.push_str(&entry.to_string());
        contents.push(',');
    }

    if !molecules.is_empty() {
        contents.pop();
    }

    contents.push(']');

    contents
}

fn create_server_path(server_path: &String) {
    let linkname = format!("htdocs/{}", server_path);
    let linkpath = std::path::Path::new(&linkname);

    if !linkpath.exists() {
        let filename = format!("fitswebql");
        let filepath = std::path::Path::new(&filename);

        match std::os::unix::fs::symlink(filepath, linkpath) {
            Ok(_) => {}
            Err(err) => println!("could not create a symbolic link to {}: {}", linkname, err),
        }
    }
}

fn remove_symlinks(server_path: Option<String>) {
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

    match server_path {
        Some(name) => {
            let filename = format!("htdocs/{}", name);
            let filepath = std::path::Path::new(&filename);

            if filepath.exists() {
                if let Ok(metadata) = filepath.symlink_metadata() {
                    let filetype = metadata.file_type();

                    if filetype.is_symlink() {
                        println!("removing a symbolic link to {:?}", filepath.file_name());
                        let _ = std::fs::remove_file(filepath);
                    }
                }
            }
        }
        None => {}
    }
}

fn get_home_directory(home_dir: &Option<std::path::PathBuf>) -> HttpResponse {
    match home_dir {
        Some(path_buf) => get_directory(path_buf.to_path_buf()),
        None => HttpResponse::NotFound()
            .content_type("text/html")
            .body(format!(
                "<p><b>Critical Error</b>: home directory not found</p>"
            )),
    }
}

fn get_directory(path: std::path::PathBuf) -> HttpResponse {
    println!("scanning directory: {:?}", path);

    let mut ordered_entries = BTreeMap::new();

    for entry in path.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            let file_name_buf = entry.file_name();
            let file_name = file_name_buf.to_str().unwrap();

            if file_name.starts_with(".") {
                continue;
            }

            if let Ok(metadata) = entry.metadata() {
                //println!("{:?}:{:?} filesize: {}", entry.path(), metadata, metadata.len());

                if metadata.is_dir() {
                    let ts = match metadata.modified() {
                        Ok(x) => x,
                        Err(_) => std::time::UNIX_EPOCH,
                    };

                    let std_duration = ts.duration_since(std::time::UNIX_EPOCH).unwrap();
                    let chrono_duration = ::chrono::Duration::from_std(std_duration).unwrap();
                    let unix = chrono::NaiveDateTime::from_timestamp(0, 0);
                    let naive = unix + chrono_duration;

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
                    let path = entry.path();
                    let ext = path.extension().and_then(std::ffi::OsStr::to_str);

                    if ext == Some("fits")
                        || ext == Some("FITS")
                        || path.to_str().unwrap().ends_with(".fits.gz")
                        || path.to_str().unwrap().ends_with(".FITS.GZ")
                    {
                        let ts = match metadata.modified() {
                            Ok(x) => x,
                            Err(_) => std::time::UNIX_EPOCH,
                        };

                        let std_duration = ts.duration_since(std::time::UNIX_EPOCH).unwrap();
                        let chrono_duration = ::chrono::Duration::from_std(std_duration).unwrap();
                        let unix = chrono::NaiveDateTime::from_timestamp(0, 0);
                        let naive = unix + chrono_duration;

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
        contents.push_str(&entry.to_string());
        contents.push(',');
    }

    if !ordered_entries.is_empty() {
        //remove the last comma
        contents.pop();
    }

    contents.push(']');

    HttpResponse::Ok()
        .content_type("application/json")
        .body(format!(
            "{{\"location\": \"{}\", \"contents\": {} }}",
            path.display(),
            contents
        ))
}

fn directory_handler(
    req: &HttpRequest<WsSessionState>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    let query = req.query();

    result(Ok(match query.get("dir") {
        Some(x) => get_directory(std::path::PathBuf::from(x)),
        None => {
            let state = req.state();
            let home_dir = &state.home_dir;

            //default location
            get_home_directory(home_dir)
        }
    }))
    .responder()
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

fn websocket_entry(
    req: &HttpRequest<WsSessionState>,
) -> Result<Box<Future<Item = HttpResponse, Error = Error>>, Error> /*Result<HttpResponse>*/ {
    let dataset_id_orig: String = req.match_info().query("id").unwrap();

    //dataset_id needs to be URI-decoded
    let dataset_id = match percent_decode(dataset_id_orig.as_bytes()).decode_utf8() {
        Ok(x) => x.into_owned(),
        Err(_) => dataset_id_orig.clone(),
    };

    let empty_agent = HeaderValue::from_static("");

    let headers = req.headers();
    let user_agent = match headers.get("user-agent") {
        Some(agent) => agent,
        None => &empty_agent,
    };

    let id: Vec<String> = dataset_id.split(';').map(|s| s.to_string()).collect();

    println!(
        "new websocket request for {:?}, user agent: {:?}",
        id, user_agent
    );

    Ok(Box::new(result(ws::start(req, UserSession::new(&id)))))
}

fn fitswebql_entry(
    req: &HttpRequest<WsSessionState>,
) -> Box<Future<Item = HttpResponse, Error = Error>> /*HttpResponse*/ {
    let fitswebql_path: String = req.match_info().query("path").unwrap();

    let state = req.state();
    let server = &state.addr;

    let query = req.query();

    #[cfg(feature = "server")]
    let db = match query.get("db") {
        Some(x) => x,
        None => "alma", //default database
    };

    #[cfg(feature = "server")]
    let table = match query.get("table") {
        Some(x) => x,
        None => "cube", //default table
    };

    #[cfg(not(feature = "server"))]
    let dir = match query.get("dir") {
        Some(x) => x,
        None => ".", //by default use the current directory
    };

    #[cfg(not(feature = "server"))]
    let ext = match query.get("ext") {
        Some(x) => x,
        None => "fits", //a default FITS file extension
    };

    #[cfg(not(feature = "server"))]
    let dataset = "filename";

    #[cfg(feature = "server")]
    let dataset = "datasetId";

    let dataset_id = match query.get(dataset) {
        Some(x) => vec![x.as_str()],
        None => {
            //try to read multiple datasets or filename,
            //i.e. dataset1,dataset2,... or filename1,filename2,...
            let mut v: Vec<&str> = Vec::new();
            let mut count: u32 = 1;

            loop {
                let pattern = format!("{}{}", dataset, count);
                count = count + 1;

                match query.get(&pattern) {
                    Some(x) => {
                        v.push(x);
                    }
                    None => {
                        break;
                    }
                };
            }

            //the last resort
            if v.is_empty() {
                return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                    format!("<p><b>Critical Error</b>: no {} available</p>", dataset),
                )))
                .responder();
            };

            v
        }
    };

    //sane defaults
    let mut composite = false;
    let mut optical = false;
    let mut flux = "";

    #[cfg(feature = "server")]
    {
        if db.contains("hsc") {
            optical = true;
            flux = "ratio";
        };

        if table.contains("fugin") {
            flux = "logistic";
        }
    }

    match query.get("view") {
        Some(value) => {
            if value.contains("composite") {
                composite = true;
            }

            if value.contains("optical") {
                optical = true;
            }
        }
        None => {}
    };

    match query.get("flux") {
        Some(value) => {
            let mut valid_values: HashSet<String> = HashSet::new();
            valid_values.insert(String::from("linear"));
            valid_values.insert(String::from("logistic"));
            valid_values.insert(String::from("ratio"));
            valid_values.insert(String::from("square"));
            valid_values.insert(String::from("legacy"));

            if valid_values.contains(value) {
                flux = value;
            };
        }
        None => {}
    };

    #[cfg(feature = "server")]
    let resp = format!(
        "FITSWebQL path: {}, db: {}, table: {}, dataset_id: {:?}, composite: {}, optical: {}, flux: {}",
        fitswebql_path, db, table, dataset_id, composite, optical, flux
    );

    #[cfg(not(feature = "server"))]
    let resp = format!(
        "FITSWebQL path: {}, dir: {}, ext: {}, filename: {:?}, composite: {}, optical: {}, flux: {}",
        fitswebql_path, dir, ext, dataset_id, composite, optical, flux
    );

    println!("{}", resp);

    //server version
    #[cfg(feature = "server")]
    return result(Ok(execute_fits(
        &fitswebql_path,
        &db,
        &table,
        fits::FITSCACHE,
        "fits",
        &dataset_id,
        composite,
        optical,
        &flux,
        &server,
    )))
    .responder();

    //local (Personal Edition)
    #[cfg(not(feature = "server"))]
    return result(Ok(execute_fits(
        &fitswebql_path,
        "",
        "",
        &dir,
        &ext,
        &dataset_id,
        composite,
        optical,
        &flux,
        &server,
    )))
    .responder();
}

fn get_image(req: &HttpRequest<WsSessionState>) -> Box<Future<Item = HttpResponse, Error = Error>> {
    //println!("{:?}", req);

    let query = req.query();

    let dataset_id = match query.get("datasetId") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_spectrum/datasetId parameter not found</p>"),
            )))
            .responder();
        }
    };

    //println!("[get_image] http request for {}", dataset_id);

    //check the IMAGECACHE first
    let filename = format!("{}/{}.img", fits::IMAGECACHE, dataset_id.replace("/", "_"));
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
            let filename = format!("{}/{}.img", fits::IMAGECACHE, dataset_id.replace("/","_"));
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

fn get_spectrum(
    req: &HttpRequest<WsSessionState>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    //println!("{:?}", req);

    let query = req.query();

    let dataset_id = match query.get("datasetId") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_spectrum/datasetId parameter not found</p>"),
            )))
            .responder();
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

struct MoleculeStream {
    rx: mpsc::Receiver<Molecule>,
    first: bool,
    end_transmission: bool,
}

impl MoleculeStream {
    pub fn new(rx: mpsc::Receiver<Molecule>) -> MoleculeStream {
        MoleculeStream {
            rx: rx,
            first: true,
            end_transmission: false,
        }
    }
}

impl Stream for MoleculeStream {
    type Item = Bytes;
    type Error = actix_web::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.rx.recv() {
            Ok(molecule) => {
                //println!("{:?}", molecule);

                if self.first {
                    self.first = false;

                    //the first molecule
                    Ok(Async::Ready(Some(Bytes::from(format!(
                        "{{\"molecules\" : [{}",
                        molecule.to_json().to_string()
                    )))))
                } else {
                    //subsequent molecules
                    Ok(Async::Ready(Some(Bytes::from(format!(
                        ",{}",
                        molecule.to_json().to_string()
                    )))))
                }
            }
            Err(err) => {
                if self.end_transmission {
                    println!("MoleculeStream: {}, terminating transmission", err);
                    Ok(Async::Ready(None))
                } else {
                    self.end_transmission = true;

                    if self.first {
                        //no molecules received; send an empty JSON array
                        Ok(Async::Ready(Some(Bytes::from("{\"molecules\" : []}"))))
                    } else {
                        //end a JSON array
                        Ok(Async::Ready(Some(Bytes::from("]}"))))
                    }
                }
            }
        }
    }
}

fn get_molecules(
    req: &HttpRequest<WsSessionState>,
) -> Box<Future<Item = HttpResponse, Error = Error>> {
    //println!("{:?}", req);

    let query = req.query();

    let dataset_id = match query.get("datasetId") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!(
                    "<p><b>Critical Error</b>: get_molecules/datasetId parameter not found</p>"
                ),
            )))
            .responder();
        }
    };

    //freq_start
    let freq_start = match query.get("freq_start") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!(
                    "<p><b>Critical Error</b>: get_molecules/freq_start parameter not found</p>"
                ),
            )))
            .responder();
        }
    };

    let freq_start = match freq_start.parse::<f64>() {
        Ok(x) => x / 1e9, //[Hz -> GHz]
        Err(_) => 0.0,
    };

    //freq_end
    let freq_end = match query.get("freq_end") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_molecules/freq_end parameter not found</p>"),
            )))
            .responder();
        }
    };

    let freq_end = match freq_end.parse::<f64>() {
        Ok(x) => x / 1e9, //[Hz -> GHz]
        Err(_) => 0.0,
    };

    println!(
        "[get_molecules] http request for {}: freq_start={}, freq_end={} [GHz]",
        dataset_id, freq_start, freq_end
    );

    result(Ok({
        if freq_start == 0.0 || freq_end == 0.0 {
            //send a request to the SessionServers

            let state = req.state();
            let server = &state.addr;

            let resp = server
                .send(server::GetMolecules {
                    dataset_id: dataset_id.to_owned(),
                })
                .wait();

            match resp {
                Ok(content) => {
                    if content == "" {
                        HttpResponse::Accepted()
                            .content_type("text/html")
                            .body(format!(
                                "<p><b>spectral lines for {} not available yet</p>",
                                dataset_id
                            ))
                    } else {
                        #[derive(Serialize, Deserialize)]
                        struct FreqRange {
                            freq_start: f64,
                            freq_end: f64,
                        }

                        let res: std::result::Result<FreqRange, serde_json::Error> =
                            serde_json::from_str(&content);

                        match res {
                            Ok(range) => {
                                let freq_start = range.freq_start;
                                let freq_end = range.freq_end;

                                //stream molecules from sqlite
                                match stream_molecules(freq_start, freq_end) {
                                    Some(rx) => {
                                        let molecules_stream = MoleculeStream::new(rx);

                                        HttpResponse::Ok()
                                            .content_type("application/json")
                                            .streaming(molecules_stream)
                                    }
                                    None => HttpResponse::Ok()
                                        .content_type("application/json")
                                        .body(format!("{{\"molecules\" : []}}")),
                                }
                            }
                            Err(_) => {
                                HttpResponse::NotFound()
                                    .content_type("text/html")
                                    .body(format!(
                                        "<p><b>Critical Error</b>: spectral lines not found</p>"
                                    ))
                            }
                        }
                    }
                }
                Err(_) => HttpResponse::NotFound()
                    .content_type("text/html")
                    .body(format!(
                        "<p><b>Critical Error</b>: spectral lines not found</p>"
                    )),
            }
        } else {
            //stream molecules from sqlite without waiting for a FITS header
            match stream_molecules(freq_start, freq_end) {
                Some(rx) => {
                    let molecules_stream = MoleculeStream::new(rx);

                    HttpResponse::Ok()
                        .content_type("application/json")
                        .streaming(molecules_stream)
                }
                None => HttpResponse::Ok()
                    .content_type("application/json")
                    .body(format!("{{\"molecules\" : []}}")),
            }
        }
    }))
    .responder()
}

struct FITSDataStream {
    rx: mpsc::Receiver<Vec<u8>>,
}

impl FITSDataStream {
    pub fn new(rx: mpsc::Receiver<Vec<u8>>) -> FITSDataStream {
        FITSDataStream { rx: rx }
    }
}

impl Stream for FITSDataStream {
    type Item = Bytes;
    type Error = actix_web::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.rx.recv() {
            Ok(v) => {
                //print!("partial FITS chunk length: {}", v.len());
                Ok(Async::Ready(Some(Bytes::from(v))))
            }
            Err(err) => {
                println!("FITSDataStream: {}, terminating transmission", err);

                Ok(Async::Ready(None))
            }
        }
    }
}

fn get_fits(req: &HttpRequest<WsSessionState>) -> Box<Future<Item = HttpResponse, Error = Error>> {
    //println!("{:?}", req);

    let query = req.query();

    /*#[cfg(not(feature = "server"))]
    let dataset = "filename";

    #[cfg(feature = "server")]*/
    let dataset = "datasetId"; //JavaScript get_fits? always uses datasetId

    let dataset_id = match query.get(dataset) {
        Some(x) => vec![x.as_str()],
        None => {
            //try to read multiple datasets or filename,
            //i.e. dataset1,dataset2,... or filename1,filename2,...
            let mut v: Vec<&str> = Vec::new();
            let mut count: u32 = 1;

            loop {
                let pattern = format!("{}{}", dataset, count);
                count = count + 1;

                match query.get(&pattern) {
                    Some(x) => {
                        v.push(x);
                    }
                    None => {
                        break;
                    }
                };
            }

            //the last resort
            if v.is_empty() {
                return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                    format!(
                        "<p><b>Critical Error</b>: get_fits/{} parameter not found</p>",
                        dataset
                    ),
                )))
                .responder();
            };

            v
        }
    };

    //x1
    let x1 = match query.get("x1") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_fits/x1 parameter not found</p>"),
            )))
            .responder();
        }
    };

    let x1 = match x1.parse::<i32>() {
        Ok(x) => x,
        Err(_) => 0,
    };

    //x2
    let x2 = match query.get("x2") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_fits/x2 parameter not found</p>"),
            )))
            .responder();
        }
    };

    let x2 = match x2.parse::<i32>() {
        Ok(x) => x,
        Err(_) => 0,
    };

    //y1
    let y1 = match query.get("y1") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_fits/y1 parameter not found</p>"),
            )))
            .responder();
        }
    };

    let y1 = match y1.parse::<i32>() {
        Ok(x) => x,
        Err(_) => 0,
    };

    //y2
    let y2 = match query.get("y2") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_fits/y2 parameter not found</p>"),
            )))
            .responder();
        }
    };

    let y2 = match y2.parse::<i32>() {
        Ok(x) => x,
        Err(_) => 0,
    };

    //frame_start
    let frame_start = match query.get("frame_start") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_fits/frame_start parameter not found</p>"),
            )))
            .responder();
        }
    };

    let frame_start = match frame_start.parse::<f64>() {
        Ok(x) => x,
        Err(_) => 0.0,
    };

    //frame_end
    let frame_end = match query.get("frame_end") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_fits/frame_end parameter not found</p>"),
            )))
            .responder();
        }
    };

    let frame_end = match frame_end.parse::<f64>() {
        Ok(x) => x,
        Err(_) => 0.0,
    };

    //ref_freq
    let ref_freq = match query.get("ref_freq") {
        Some(x) => x,
        None => {
            return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!("<p><b>Critical Error</b>: get_fits/ref_freq parameter not found</p>"),
            )))
            .responder();
        }
    };

    let ref_freq = match ref_freq.parse::<f64>() {
        Ok(x) => x,
        Err(_) => 0.0,
    };

    println!(
        "[get_fits] http request for {:?}: x1={}, y1={}, x2={}, y2={}, frame_start={}, frame_end={}, ref_freq={}",
        dataset_id, x1, y1, x2, y2, frame_start, frame_end, ref_freq
    );

    if dataset_id.len() > 1 {
        let mut ar = Builder::new(Vec::new());

        //dataset_id.iter().for_each(|entry| {//not used for now; problems accessing inner data of Arc<RwLock<Builder>> later on

        //for each dataset append it to the archives
        for entry in dataset_id {
            let datasets = DATASETS.read();

            let fits = match datasets.get(entry).unwrap().try_read() {
                Some(x) => x,
                None => {
                    println!("[get_fits] error getting {} from DATASETS; aborting", entry);

                    return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                        format!(
                            "<p><b>Critical Error</b>: get_fits/{} not found in DATASETS</p>",
                            entry
                        ),
                    )))
                    .responder();
                }
            };

            {
                *fits.timestamp.write() = SystemTime::now();
            }

            if fits.has_data {
                match fits.get_cutout_data(x1, y1, x2, y2, frame_start, frame_end, ref_freq) {
                    Some(region) => {
                        let mut header = Header::new_gnu();
                        if let Err(err) =
                            header.set_path(format!("{}-subregion.fits", entry.replace("/", "_")))
                        {
                            println!("Critical Error: get_fits/tar/set_path error: {}", err);

                            return result(Ok(HttpResponse::NotFound()
                                .content_type("text/html")
                                .body(format!(
                                    "<p><b>Critical Error</b>: get_fits/tar/set_path error: {}</p>",
                                    err
                                ))))
                            .responder();
                        }

                        header.set_mode(420);

                        match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
                            Ok(n) => header.set_mtime(n.as_secs()),
                            Err(_) => println!("SystemTime before UNIX EPOCH!"),
                        };

                        header.set_size(region.len() as u64);
                        header.set_cksum();

                        if let Err(err) = ar.append(&header, region.as_slice()) {
                            println!("Critical Error: get_fits/tar/append error: {}", err);
                            return result(Ok(HttpResponse::NotFound()
                                .content_type("text/html")
                                .body(format!(
                                    "<p><b>Critical Error</b>: get_fits/tar/append error: {}</p>",
                                    err
                                ))))
                            .responder();
                        }
                    }
                    None => println!(
                        "partial FITS cut-out for {} did not produce any data",
                        entry
                    ),
                }
            }
        }

        match ar.into_inner() {
            Ok(tarball) => {
                let timestamp = Local::now();
                let disposition_filename = format!(
                    "attachment; filename=fits_web_ql_{}.tar",
                    timestamp.format("%Y-%m-%d_%H:%M:%S")
                );

                result(Ok(HttpResponse::Ok()
                    .header("Cache-Control", "no-cache, no-store, must-revalidate")
                    .header("Pragma", "no-cache")
                    .header("Expires", "0")
                    .content_type("application/force-download")
                    .content_encoding(ContentEncoding::Identity) // disable compression
                    .header("Content-Disposition", disposition_filename)
                    .header("Content-Transfer-Encoding", "binary")
                    .header("Accept-Ranges", "bytes")
                    .body(tarball)))
                .responder()
            }
            Err(err) => result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!(
                    "<p><b>Critical Error</b>: get_fits tarball creation error: {}</p>",
                    err
                ),
            )))
            .responder(),
        }
    } else {
        //only one dataset, no need to use tarball, stream the data instead
        let entry = dataset_id[0];

        let datasets = DATASETS.read();

        let fits = match datasets.get(entry).unwrap().try_read() {
            Some(x) => x,
            None => {
                println!("[get_fits] error getting {} from DATASETS; aborting", entry);

                return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                    format!(
                        "<p><b>Critical Error</b>: get_fits/{} not found in DATASETS</p>",
                        entry
                    ),
                )))
                .responder();
            }
        };

        {
            *fits.timestamp.write() = SystemTime::now();
        }

        if fits.has_data {
            //streaming version (an immediate response, low memory footprint)
            match fits.get_cutout_stream(x1, y1, x2, y2, frame_start, frame_end, ref_freq) {
                Some(rx) => {
                    let fits_stream = FITSDataStream::new(rx);

                    let disposition_filename = format!(
                        "attachment; filename={}-subregion.fits",
                        entry.replace("/", "_")
                    );

                    return result(Ok(HttpResponse::Ok()
                        .header("Cache-Control", "no-cache, no-store, must-revalidate")
                        .header("Pragma", "no-cache")
                        .header("Expires", "0")
                        .content_type("application/force-download")
                        .content_encoding(ContentEncoding::Identity) // disable compression
                        .header("Content-Disposition", disposition_filename)
                        .header("Content-Transfer-Encoding", "binary")
                        .header("Accept-Ranges", "bytes")
                        .streaming(fits_stream)))
                    .responder();
                }
                None => {
                    return result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                        format!(
                            "<p><b>Critical Error</b>: get_fits: {} contains no data</p>",
                            entry
                        ),
                    )))
                    .responder();
                }
            }

        //non-streaming version (stores the whole FITS cutout in memory, delayed response)
        /*match fits.get_cutout_data(x1, y1, x2, y2, frame_start, frame_end, ref_freq) {
            Some(region) => {
                let disposition_filename = format!(
                    "attachment; filename={}-subregion.fits",
                    entry.replace("/", "_")
                );

                result(Ok(HttpResponse::Ok()
                    .header("Cache-Control", "no-cache, no-store, must-revalidate")
                    .header("Pragma", "no-cache")
                    .header("Expires", "0")
                    .content_type("application/force-download")
                    .content_encoding(ContentEncoding::Identity)// disable compression
                    .header("Content-Disposition", disposition_filename)
                    .header("Content-Transfer-Encoding", "binary")
                    .header("Accept-Ranges", "bytes")
                    .body(region))).responder()
            }
            None => {
                println!(
                    "partial FITS cut-out for {} did not produce any data",
                    entry
                );
                result(Ok(HttpResponse::NotFound().content_type("text/html").body(
            format!(
                "<p><b>Critical Error</b>: partial FITS cut-out for {} did not produce any data</p>",
                entry
            ),
        ))).responder()
            }
        }*/
        } else {
            result(Ok(HttpResponse::NotFound().content_type("text/html").body(
                format!(
                    "<p><b>Critical Error</b>: get_fits: {} contains no data</p>",
                    entry
                ),
            )))
            .responder()
        }
    }
}

#[cfg(feature = "server")]
fn get_jvo_path(dataset_id: &String, db: &str, table: &str) -> (Option<std::path::PathBuf>, bool) {
    let connection_url = format!("postgresql://{}@{}/{}", JVO_USER, JVO_HOST, db);

    println!("PostgreSQL connection URL: {}", connection_url);

    match Connection::connect(connection_url, TlsMode::None) {
        Ok(conn) => {
            println!("connected to PostgreSQL");

            //data_id: if db is alma append _00_00_00
            let data_id = match db {
                "alma" => format!("{}_00_00_00", dataset_id),
                _ => dataset_id.clone(),
            };

            let sql = format!("SELECT path FROM {} WHERE data_id = '{}';", table, data_id);
            let res = conn.query(&sql, &[]);

            match res {
                Ok(rows) => {
                    for row in &rows {
                        let path: String = row.get(0);

                        let filename = match table.find('.') {
                            Some(index) => {
                                let table = &table[0..index];
                                format!(
                                    "{}/{}/{}/{}",
                                    fits::FITSHOME,
                                    db,
                                    table.to_string().to_ascii_uppercase(),
                                    path
                                )
                            }
                            None => format!("{}/{}/{}", fits::FITSHOME, db, path),
                        };

                        let filepath = std::path::PathBuf::from(&filename);
                        let ext = filepath.extension().and_then(std::ffi::OsStr::to_str);
                        println!("filepath: {:?}", filepath);

                        if filepath.exists() {
                            let is_compressed = if ext == Some("gz") || ext == Some("GZ") {
                                true
                            } else {
                                false
                            };
                            return (Some(filepath), is_compressed);
                        }
                    }
                }
                Err(err) => println!("error executing a SQL query {}: {}", sql, err),
            };
        }
        Err(err) => println!("error connecting to PostgreSQL: {}", err),
    }

    return (None, false);
}

fn execute_fits(
    fitswebql_path: &String,
    _db: &str,
    _table: &str,
    dir: &str,
    ext: &str,
    dataset_id: &Vec<&str>,
    composite: bool,
    is_optical: bool,
    flux: &str,
    server: &Addr<server::SessionServer>,
) -> HttpResponse {
    //get fits location

    //launch FITS threads
    let mut has_fits: bool = true;

    //for each dataset_id
    for i in 0..dataset_id.len() {
        let data_id = dataset_id[i];

        //does the entry exist in the datasets hash map?
        let has_entry = {
            let datasets = DATASETS.read();
            datasets.contains_key(data_id)
        };

        //if it does not exist set has_fits to false and load the FITS data
        if !has_entry {
            has_fits = false;

            #[cfg(feature = "server")]
            let my_db = _db.to_string();

            #[cfg(feature = "server")]
            let my_table = _table.to_string();

            let my_dir = dir.to_string();
            let my_data_id = data_id.to_string();
            let my_ext = ext.to_string();
            let my_server = server.clone();
            let my_flux = flux.to_string();

            DATASETS.write().insert(
                my_data_id.clone(),
                Arc::new(RwLock::new(Box::new(fits::FITS::new(
                    &my_data_id,
                    &my_flux,
                )))),
            );

            //load FITS data in a new thread
            thread::spawn(move || {
                #[cfg(not(feature = "server"))]
                let (filepath, is_compressed) = (
                    std::path::PathBuf::from(&format!("{}/{}.{}", my_dir, my_data_id, my_ext)),
                    if my_ext == "gz" || my_ext == "GZ" {
                        true
                    } else {
                        false
                    },
                );

                #[cfg(feature = "server")]
                let (filepath, is_compressed) = {
                    //try to read a directory from the PostgreSQL database
                    let (buf, is_compressed) =
                        get_jvo_path(&my_data_id.to_string(), &my_db, &my_table);

                    match buf {
                        Some(buf) => (buf, is_compressed),
                        None => (
                            std::path::PathBuf::from(&format!(
                                "{}/{}.{}",
                                my_dir, my_data_id, my_ext
                            )),
                            false,
                        ),
                    }
                };

                println!("loading FITS data from {:?}", filepath);

                let fits = fits::FITS::from_path(
                    &my_data_id.clone(),
                    &my_flux.clone(),
                    is_optical,
                    filepath.as_path(),
                    is_compressed,
                    &my_server,
                ); //from_path or from_path_mmap

                let fits = Arc::new(RwLock::new(Box::new(fits)));

                DATASETS.write().insert(my_data_id.clone(), fits.clone());

                if fits.read().has_data {
                    thread::spawn(move || {
                        fits.read().make_data_histogram();
                    });
                };
            });
        } else {
            //update the timestamp
            let datasets = DATASETS.read();
            let dataset = datasets.get(data_id).unwrap().read();

            has_fits = has_fits && dataset.has_data;
            *dataset.timestamp.write() = SystemTime::now();
        };
    }

    http_fits_response(
        &fitswebql_path,
        &dataset_id,
        composite,
        is_optical,
        has_fits,
    )
}

fn http_fits_response(
    fitswebql_path: &String,
    dataset_id: &Vec<&str>,
    composite: bool,
    is_optical: bool,
    has_fits: bool,
) -> HttpResponse {
    println!("calling http_fits_response for {:?}", dataset_id);
    //let has_fits: bool = false ;//later on it should be changed to true; iterate over all datasets, setting it to false if not found

    //build up an HTML response
    let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");

    html.push_str(
        "<link href=\"https://fonts.googleapis.com/css?family=Inconsolata\" rel=\"stylesheet\"/>\n",
    );

    html.push_str("<script src=\"https://d3js.org/d3.v5.min.js\"></script>\n");

    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"reconnecting-websocket.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/reconnecting-websocket.js\"></script>\n");

    html.push_str("<script src=\"//cdnjs.cloudflare.com/ajax/libs/numeral.js/2.0.6/numeral.min.js\"></script>\n");

    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"ra_dec_conversion.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/ra_dec_conversion.js\"></script>\n");

    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"sylvester.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/sylvester.js\"></script>\n");

    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"shortcut.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/shortcut.js\"></script>\n");

    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"colourmaps.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/colourmaps.js\"></script>\n");

    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"lz4.min.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/lz4.min.js\"></script>\n");

    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"marchingsquares-isocontours.min.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/marchingsquares-isocontours.min.js\"></script>\n");

    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"marchingsquares-isobands.min.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/marchingsquares-isobands.min.js\"></script>\n");

    //VP9 decoder
    #[cfg(not(feature = "cdn"))]
    html.push_str("<script src=\"ogv-decoder-video-vp9.js\"></script>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/ogv-decoder-video-vp9.js\"></script>\n");

    //custom vpx wasm decoder
    #[cfg(feature = "vp9")]
    {
        #[cfg(not(feature = "cdn"))]
        html.push_str("<script src=\"vpx.js\"></script>\n");
        #[cfg(feature = "cdn")]
        html.push_str("<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/vpx.js\"></script>\n");

        html.push_str("<script>
        Module.onRuntimeInitialized = async _ => {
            api = {
                vpx_version: Module.cwrap('vpx_version', 'number', []),
                vpx_init: Module.cwrap('vpx_init', '', []),
                vpx_destroy: Module.cwrap('vpx_destroy', '', []),
                vpx_decode_frame: Module.cwrap('vpx_decode_frame', 'number', ['number', 'number', 'number', 'number', 'number', 'number', 'string']),
            };
            console.log('VP9 libvpx decoder version:', api.vpx_version());
            api.vpx_init();
        };
        </script>\n");
    }

    //custom hevc wasm decoder
    #[cfg(feature = "hevc")]
    {
        #[cfg(not(feature = "cdn"))]
        html.push_str(&format!(
            "<script src=\"hevc_{}.js\"></script>\n",
            WASM_STRING
        ));

        #[cfg(feature = "cdn")]
        html.push_str(&format!(
            "<script src=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/hevc_{}.js\"></script>\n",
            WASM_STRING
        ));

        html.push_str("<script>
        Module.onRuntimeInitialized = async _ => {
            api = {                
                hevc_init: Module.cwrap('hevc_init', '', ['number']), 
                hevc_destroy: Module.cwrap('hevc_destroy', '', ['number']),                
                hevc_decode_nal_unit: Module.cwrap('hevc_decode_nal_unit', 'number', ['number', 'number', 'number', 'number', 'number', 'number', 'number', 'number', 'string']),               
            };                   
        };
        </script>\n");
    }

    //bootstrap
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1, user-scalable=no, minimum-scale=1, maximum-scale=1\">\n");
    html.push_str("<link rel=\"stylesheet\" href=\"https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/css/bootstrap.min.css\">\n");
    html.push_str("<script src=\"https://ajax.googleapis.com/ajax/libs/jquery/3.1.1/jquery.min.js\"></script>\n");
    html.push_str("<script src=\"https://maxcdn.bootstrapcdn.com/bootstrap/3.3.7/js/bootstrap.min.js\"></script>\n");

    //FITSWebQL main JavaScript
    html.push_str(&format!(
        "<script src=\"fitswebql.js?{}\"></script>\n",
        VERSION_STRING
    ));
    //custom css styles
    #[cfg(not(feature = "cdn"))]
    html.push_str("<link rel=\"stylesheet\" href=\"fitswebql.css\"/>\n");
    #[cfg(feature = "cdn")]
    html.push_str("<link rel=\"stylesheet\" href=\"https://cdn.jsdelivr.net/gh/jvo203/fits_web_ql/htdocs/fitswebql/fitswebql.css\"/>\n");

    html.push_str("<title>FITSWebQL</title></head><body>\n");
    html.push_str(&format!(
        "<div id='votable' style='width: 0; height: 0;' data-va_count='{}' ",
        dataset_id.len()
    ));

    if dataset_id.len() == 1 {
        html.push_str(&format!("data-datasetId='{}' ", dataset_id[0]));
    } else {
        for i in 0..dataset_id.len() {
            html.push_str(&format!("data-datasetId{}='{}' ", i + 1, dataset_id[i]));
        }

        if composite && dataset_id.len() <= 3 {
            html.push_str("data-composite='1' ");
        }
    }

    html.push_str(&format!("data-root-path='/{}/' data-server-version='{}' data-server-string='{}' data-server-mode='{}' data-has-fits='{}' data-is-optical='{}'></div>\n", fitswebql_path, VERSION_STRING, SERVER_STRING, SERVER_MODE, has_fits, is_optical));

    #[cfg(not(feature = "server"))]
    {
        html.push_str(
            "<script>
        var WS_SOCKET = 'ws://';
        </script>\n",
        );
    }

    #[cfg(feature = "server")]
    {
        #[cfg(not(feature = "production"))]
        {
            html.push_str(
                "<script>
        var WS_SOCKET = 'ws://';
        </script>\n",
            );
        }

        #[cfg(feature = "production")]
        {
            html.push_str(
                "<script>
        var WS_SOCKET = 'wss://';
        </script>\n",
            );
        }
    }

    //the page entry point
    html.push_str(
        "<script>
        const golden_ratio = 1.6180339887;
        var ALMAWS = null ;
        var wsVideo = null ;
        var wsConn = null ;
        var firstTime = true ;
        var has_image = false ;         
        var PROGRESS_VARIABLE = 0.0 ;
        var PROGRESS_INFO = \"\" ;      
        var RESTFRQ = 0.0 ;
        var USER_SELFRQ = 0.0 ;
        var USER_DELTAV = 0.0 ;
        var ROOT_PATH = \"/fitswebql/\" ;        
        var idleResize = -1;
        window.onresize = resizeMe;
        window.onbeforeunload = function() {            
            if(wsConn != null)
            {
                for(let i=0;i<va_count;i++)
                    wsConn[i].close();
            }

            if(wsVideo != null)
                wsVideo.close();
        };
        mainRenderer();
    </script>\n",
    );

    //Google Analytics
    #[cfg(feature = "development")]
    html.push_str(
        "<script>
  (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){ 
  (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o), 
  m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m) 
  })(window,document,'script','//www.google-analytics.com/analytics.js','ga');
  ga('create', 'UA-72136224-1', 'auto');				
  ga('send', 'pageview');						  									
  </script>\n",
    );

    #[cfg(feature = "test")]
    html.push_str(
        "<script>
  (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){ 
  (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o), 
  m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m) 
  })(window,document,'script','//www.google-analytics.com/analytics.js','ga');
  ga('create', 'UA-72136224-2', 'auto');				
  ga('send', 'pageview');  									
  </script>\n",
    );

    #[cfg(feature = "production")]
    html.push_str(
        "<script>
  (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){
  (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o),
  m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m)
  })(window,document,'script','//www.google-analytics.com/analytics.js','ga');
  ga('create', 'UA-72136224-4', 'auto');
  ga('send', 'pageview');
  </script>\n",
    );

    #[cfg(not(feature = "server"))]
    html.push_str(
        "<script>
      (function(i,s,o,g,r,a,m){i['GoogleAnalyticsObject']=r;i[r]=i[r]||function(){
      (i[r].q=i[r].q||[]).push(arguments)},i[r].l=1*new Date();a=s.createElement(o),
      m=s.getElementsByTagName(o)[0];a.async=1;a.src=g;m.parentNode.insertBefore(a,m)
      })(window,document,'script','//www.google-analytics.com/analytics.js','ga');
      ga('create', 'UA-72136224-5', 'auto');
      ga('send', 'pageview');
      </script>\n",
    );

    html.push_str("</body></html>\n");

    HttpResponse::Ok().content_type("text/html").body(html)
}

fn main() {
    //std::env::set_var("RUST_LOG", "info");
    std::env::set_var("RUST_LOG", "actix_web=info");

    #[cfg(feature = "server")]
    flexi_logger::Logger::with_env_or_str("fits_web_ql=info")
        .log_to_file()
        .directory(LOG_DIRECTORY)
        //.format(flexi_logger::opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    info!("{} main()", SERVER_STRING);

    let mut server_port = SERVER_PORT;
    let mut server_path = String::from(SERVER_PATH);
    let mut server_address = String::from(SERVER_ADDRESS);
    let mut home_dir = dirs::home_dir();
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 {
        for i in 1..args.len() - 1 {
            let key = &args[i];
            let value = &args[i + 1];

            if key == "--port" {
                match value.parse::<i32>() {
                    Ok(port) => server_port = port,
                    Err(err) => println!(
                        "error parsing the port number: {}, defaulting to {}",
                        err, server_port
                    ),
                }
            }

            if key == "--path" {
                server_path = value.clone();

                create_server_path(value);
            }

            if key == "--interface" {
                server_address = value.clone();
            }

            if key == "--home" {
                let path = std::path::PathBuf::from(value);

                if path.exists() {
                    home_dir = Some(path);
                } else {
                    println!(
                        "the specified home directory {} cannot be found, using the default $HOME",
                        value
                    );
                }
            }
        }
    }

    println!(
        "server interface: {}, port: {}, path: {}",
        server_address, server_port, server_path
    );

    remove_symlinks(None);

    //splatalogue sqlite db integration
    /*let splat_path = std::path::Path::new("splatalogue_v3.db");
    let splat_db = sqlite::open(splat_path).unwrap();*/

    #[cfg(not(feature = "server"))]
    let index_file = "fitswebql.html";

    #[cfg(feature = "server")]
    let index_file = "almawebql.html";

    let sys = actix::System::new("fits_web_ql");

    // Start the WebSocket message server actor in a separate thread
    let server = Arbiter::start(|_| server::SessionServer::default());
    //let server: Addr<Syn, _> = SyncArbiter::start(32,|| server::SessionServer::default());//16 or 32 threads at most

    let actix_server_path = server_path.clone();

    HttpServer::new(
        move || {
            // WebSocket sessions state
            let state = WsSessionState {
                addr: server.clone(),
                home_dir: home_dir.clone(),
            };

            App::with_state(state)
                //.middleware(Logger::default())
                .middleware(Logger::new("%t %a %{User-Agent}i %r")
                    .exclude("/")
                    .exclude("/fitswebql/get_molecules")
                    .exclude("/fitswebql/get_image")
                    .exclude("/fitswebql/get_spectrum")
                    .exclude(format!("/{}/get_molecules", actix_server_path))
                    .exclude(format!("/{}/get_image", actix_server_path))
                    .exclude(format!("/{}/get_spectrum", actix_server_path))
                )
                .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::GET).f(fitswebql_entry)})  
                .resource("/{path}/websocket/{id}", |r| {r.route().f(websocket_entry)})                
                .resource("/get_directory", |r| {r.method(http::Method::GET).f(directory_handler)})
                .resource("/{path}/get_image", |r| {r.method(http::Method::GET).f(get_image)})
                .resource("/{path}/get_spectrum", |r| {r.method(http::Method::GET).f(get_spectrum)})
                .resource("/{path}/get_molecules", |r| {r.method(http::Method::GET).f(get_molecules)})
                .resource("/{path}/get_fits", |r| {r.method(http::Method::GET).f(get_fits)})
                .handler("/", fs::StaticFiles::new("htdocs").unwrap().index_file(index_file))
        })
        .bind(&format!("{}:{}", server_address, server_port)).expect(&format!("Cannot bind to {}:{}, try setting a different HTTP port via a command-line option '--port XXXX'", server_address, server_port))        
        .start();

    println!("detected number of CPUs: {}", num_cpus::get());

    #[cfg(feature = "opencl")]
    match core::default_platform() {
        Ok(platform_id) => match core::get_device_ids(&platform_id, None, None) {
            Ok(device_ids) => {
                println!("OpenCL device list: {:?}", device_ids);

                for dev in device_ids {
                    let dev_version =
                        core::get_device_info(&dev, core::DeviceInfo::Version).unwrap();
                    let dev_type = core::get_device_info(&dev, core::DeviceInfo::Type).unwrap();
                    println!("Device type: {}, OpenCL version: {}", dev_type, dev_version);
                }
            }
            Err(err) => println!("{}", err),
        },
        Err(err) => println!("{}", err),
    }

    #[cfg(not(feature = "server"))]
    {
        println!(
            "started a local FITSWebQL server; point your web browser to http://localhost:{}",
            server_port
        );
        println!("press CTRL+C to exit");
    }

    #[cfg(feature = "server")]
    {
        println!(
            "started a fits_web_ql server process on port {}",
            server_port
        );
        println!("send SIGINT to shut down, i.e. killall -s SIGINT fits_web_ql");
    }

    let _ = sys.run();

    DATASETS.write().clear();

    remove_symlinks(Some(server_path));

    println!("FITSWebQL: clean shutdown completed.");

    unsafe { x265_cleanup() };
}
