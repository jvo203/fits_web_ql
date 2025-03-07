use atomic;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use half::f16;
use num_cpus;
use parking_lot::RwLock;
use positioned_io::ReadAt;
use regex::Regex;
use std;
use std::cell::RefCell;
use std::ffi::CString;
use std::fs::File;
use std::io::BufWriter;
use std::io::Cursor;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::{Read, Write};
use std::rc::Rc;
use std::slice;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use std::time::SystemTime;
use std::{mem, ptr};

use bincode::{Encode, config, encode_to_vec};
use lz4_compress;
use uuid::Uuid;

use crate::UserParams;
use crate::server;
use ::actix::*;
use rayon;
use rayon::prelude::*;

use bzip2::read::BzDecoder;
use bzip2::write::BzDecoder as BzDecompressor;
use flate2::read::GzDecoder;
use flate2::write::GzDecoder as GzDecompressor;

#[cfg(feature = "ipp")]
macro_rules! ipp_assert {
    ($result:expr) => {
        assert!(unsafe { $result } == ipp_sys::ippStsNoErr as i32);
    };
}

#[cfg(feature = "ipp")]
const IPPI_INTER_LANCZOS: u32 = 16;

#[cfg(feature = "ipp")]
pub const HEIGHT_PER_THREAD: u32 = 512;

#[cfg(feature = "zfp")]
use zfp_sys::*;

#[cfg(feature = "zfp")]
use bincode::{Decode, decode_from_slice, encode_into_std_write};

#[cfg(feature = "zfp")]
use std::sync::atomic::AtomicBool;

#[cfg(feature = "zfp")]
#[derive(Encode, Decode, Debug)]
pub struct ZFPMaskedArray {
    pub array: Vec<u8>,
    pub mask: Vec<u8>,
    pub frame_min: f32,
    pub frame_max: f32,
    //pub precision: u32,
    pub rate: f64,
}

#[cfg(feature = "opencl")]
use ocl::ProQue;

#[cfg(feature = "opencl")]
use rand::distributions::{Distribution, StandardNormal, Uniform};

use curl::easy::Easy;

use num;
use num_integer::Integer;
use std::cmp::Ordering::Equal;

use std::sync::atomic::{AtomicIsize, Ordering};

//use openjpeg2_sys as ffi;
use vpx_sys::*;

use crate::*; //bindings to local C libraries

fn get_packets(mut ctx: vpx_codec_ctx_t) -> Option<Vec<u8>> {
    unsafe {
        let mut iter = mem::zeroed();

        loop {
            let pkt = vpx_codec_get_cx_data(&mut ctx, &mut iter);

            if pkt.is_null() {
                break;
            } else {
                println!("{:#?}", (*pkt).kind);

                if (*pkt).kind == vpx_codec_cx_pkt_kind::VPX_CODEC_CX_FRAME_PKT {
                    //println!("{:#?}",(*pkt).data.frame);
                    let f = (*pkt).data.frame;

                    println!("frame length: {} bytes", f.sz);

                    let mut image_frame: Vec<u8> = Vec::with_capacity(f.sz as usize);
                    ptr::copy_nonoverlapping(
                        mem::transmute(f.buf),
                        image_frame.as_mut_ptr(),
                        f.sz as usize,
                    );
                    image_frame.set_len(f.sz as usize);

                    return Some(image_frame);
                };
            }
        }
    };

    None
}

pub fn encode_frame(
    mut ctx: vpx_codec_ctx_t,
    mut img: vpx_image,
    frame: i64,
    flags: i64,
    deadline: u64,
) -> Result<Option<Vec<u8>>, vpx_codec_err_t> {
    let ret = unsafe { vpx_codec_encode(&mut ctx, &mut img, frame, 1, flags, deadline) };

    match ret {
        VPX_CODEC_OK => Ok(get_packets(ctx)),
        _ => Err(ret),
    }
}

pub fn flush_frame(
    mut ctx: vpx_codec_ctx_t,
    deadline: u64,
) -> Result<Option<Vec<u8>>, vpx_codec_err_t> {
    let ret = unsafe { vpx_codec_encode(&mut ctx, ptr::null_mut(), -1, 1, 0, deadline) };

    match ret {
        VPX_CODEC_OK => Ok(get_packets(ctx)),
        _ => Err(ret),
    }
}

#[cfg(feature = "zfp")]
fn zfp_decompress_float_array2d(
    mut buffer: Vec<u8>,
    nx: usize,
    ny: usize,
    //precision: u32,
    rate: f64,
) -> Result<Vec<f32>, String> {
    let _watch = Instant::now();
    let mut res = true;
    let mut array: Vec<f32> = vec![0.0; nx * ny];

    /* allocate meta data for the 2D array a[nx][ny] */
    let data_type = zfp_type_zfp_type_float;
    let field = unsafe {
        zfp_field_2d(
            array.as_mut_ptr() as *mut std::ffi::c_void,
            data_type,
            nx as usize,
            ny as usize,
        )
    };

    /* allocate meta data for a compressed stream */
    let zfp = unsafe { zfp_stream_open(std::ptr::null_mut() as *mut bitstream) };

    /* set compression mode and parameters */
    unsafe { zfp_stream_set_rate(zfp, rate, data_type, 2, 0) };
    /*let tolerance = 1.0e-3;
    unsafe { zfp_stream_set_accuracy(zfp, tolerance) };*/
    /*unsafe { zfp_stream_set_precision(zfp, precision) };*/

    #[cfg(feature = "cuda")]
    {
        let ret = unsafe { zfp_stream_set_execution(zfp, zfp_exec_policy_zfp_exec_cuda) };

        if ret == 0 {
            println!("failed to set the execution policy to zfp_exec_cuda");
        }
    }

    let bufsize = buffer.len();
    /* associate bit stream with a compressed buffer */
    let stream = unsafe { stream_open(buffer.as_mut_ptr() as *mut std::ffi::c_void, bufsize) };
    unsafe {
        zfp_stream_set_bit_stream(zfp, stream);
        zfp_stream_rewind(zfp);
    }

    let ret = unsafe { zfp_decompress(zfp, field) };
    if ret == 0 {
        res = false;
    }

    /*println!(
        "ret = {}, decompressed data sample: {:?}",
        ret,
        &array[0..10]
    );*/

    /* clean up */
    unsafe {
        zfp_field_free(field);
        zfp_stream_close(zfp);
        stream_close(stream);
    }

    if res {
        /*let time_s = (_watch.elapsed().as_nanos() as f32) / 1000000000.0;
        let total_size = nx * ny * std::mem::size_of::<f32>();
        let speed = (total_size as f32 / 1024.0 / 1024.0) / time_s;

        println!(
            "[zfp_decompress_float_array2d] elapsed time: {:?}, speed {}MB/s",
            _watch.elapsed(),
            speed
        );*/

        Ok(array)
    } else {
        Err("failed to decompress a zfp array".to_string())
    }
}

#[cfg(feature = "jvo")]
static JVO_FITS_SERVER: &'static str = "jvox.vo.nao.ac.jp";

#[cfg(feature = "jvo")]
static JVO_FITS_DB: &'static str = "alma";

#[cfg(feature = "jvo")]
pub static FITSHOME: &'static str = "/home";

pub static FITSCACHE: &'static str = "FITSCACHE";
pub static IMAGECACHE: &'static str = "IMAGECACHE";

#[cfg(feature = "raid")]
pub static RAID_PREFIX: &'static str = "/Volumes/SSD";

#[cfg(feature = "raid")]
pub const RAID_COUNT: usize = 8;

pub const IMAGE_PIXEL_COUNT_LIMIT: u64 = 1280 * 720;
pub const VIDEO_PIXEL_COUNT_LIMIT: u64 = 720 * 480;

const FITS_CHUNK_LENGTH: usize = 2880;
const FITS_LINE_LENGTH: usize = 80;

const NBINS: usize = 1024;
const NBINS2: usize = 16 * 1024;

#[derive(Debug)]
pub enum Codec {
    HEVC,
    VPX,
}

#[derive(Debug, Clone, Copy)]
pub enum Beam {
    Circle,
    Square,
}

#[derive(Debug, Clone, Copy)]
pub enum Intensity {
    Mean,
    Integrated,
}

#[derive(Debug)]
pub struct FITS {
    created: Instant,
    pub dataset_id: String,
    url: String,
    filesize: u64,
    //basic header/votable
    pub telescope: String,
    obj_name: String,
    obs_date: String,
    timesys: String,
    specsys: String,
    beam_unit: String,
    beam_type: String,
    bmaj: f64,
    bmin: f64,
    bpa: f64,
    restfrq: f64,
    line: String,
    filter: String,
    obsra: f64,
    obsdec: f64,
    datamin: f32,
    datamax: f32,
    //this is a FITS data part
    bitpix: i32,
    naxis: i32,
    naxes: [usize; 4],
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    polarisation: usize,
    data_u8: Vec<Vec<u8>>,
    data_i16: Vec<Vec<i16>>,
    data_i32: Vec<Vec<i32>>,
    data_f16: Vec<Vec<f16>>, //half-float (short)
    //data_f32: Vec<f32>,//float32 will always be converted to float16
    data_f64: Vec<Vec<f64>>,
    header: String,
    mean_spectrum: Vec<f32>,
    integrated_spectrum: Vec<f32>,
    pub mask: Vec<u8>,
    pub pixels: Vec<f32>,
    bscale: f32,
    bzero: f32,
    ignrval: f32,
    crval1: f64,
    cdelt1: f64,
    crpix1: f64,
    cunit1: String,
    ctype1: String,
    crval2: f64,
    cdelt2: f64,
    crpix2: f64,
    cunit2: String,
    ctype2: String,
    crval3: f64,
    cdelt3: f64,
    crpix3: f64,
    cunit3: String,
    ctype3: String,
    cd1_1: f64,
    cd1_2: f64,
    cd2_1: f64,
    cd2_2: f64,
    frame_min: Vec<f32>,
    frame_max: Vec<f32>,
    dmin: f32,
    dmax: f32,
    data_hist: RwLock<Vec<i64>>,
    data_median: RwLock<f32>,
    data_mad: RwLock<f32>,
    data_mad_p: RwLock<f32>,
    data_mad_n: RwLock<f32>,
    pub pmin: f32,
    pub pmax: f32,
    pub lmin: f32,
    pub lmax: f32,
    hist: Vec<i32>,
    median: f32,
    mad: f32,
    mad_p: f32,
    mad_n: f32,
    black: f32,
    white: f32,
    pub sensitivity: f32,
    pub ratio_sensitivity: f32,
    pub flux: String,
    has_frequency: bool,
    has_velocity: bool,
    frame_multiplier: f64,
    pub has_header: bool,
    pub has_data: bool,
    pub timestamp: RwLock<SystemTime>, //last access time
    pub is_optical: bool,
    pub is_xray: bool,
    pub is_dummy: bool,
    pub status_code: u16,
}

#[derive(Encode, Debug)]
struct FITSImage {
    identifier: String,
    width: u32,
    height: u32,
    image: Vec<u8>, //image/video codec-compressed luma
    alpha: Vec<u8>, //lz4-compressed alpha channel
}

impl FITS {
    pub fn new(id: &String, url: &String, flux: &String) -> FITS {
        let obj_name = match Uuid::parse_str(id) {
            Ok(_) => String::from(""),
            Err(_) => id.clone().replace(".fits", "").replace(".FITS", ""),
        };

        let fits = FITS {
            created: Instant::now(),
            dataset_id: id.clone(),
            url: url.clone(),
            filesize: 0,
            telescope: String::from(""),
            obj_name: obj_name,
            obs_date: String::from(""),
            timesys: String::from(""),
            specsys: String::from(""),
            beam_unit: String::from(""),
            beam_type: String::from(""),
            bmaj: 0.0,
            bmin: 0.0,
            bpa: 0.0,
            restfrq: 0.0,
            line: String::from(""),
            filter: String::from(""),
            obsra: 0.0,
            obsdec: 0.0,
            datamin: std::f32::MIN,
            datamax: std::f32::MAX,
            bitpix: 0,
            naxis: 0,
            naxes: [0; 4],
            width: 0,
            height: 0,
            depth: 1,
            polarisation: 1,
            data_u8: Vec::new(),
            data_i16: Vec::new(),
            data_i32: Vec::new(),
            data_f16: Vec::new(),
            //data_f32: Vec::new(),//float32 will always be converted to float16
            data_f64: Vec::new(),
            header: String::from(""),
            mean_spectrum: Vec::new(),
            integrated_spectrum: Vec::new(),
            mask: Vec::new(),
            pixels: Vec::new(),
            bscale: 1.0,
            bzero: 0.0,
            ignrval: std::f32::MIN,
            crval1: 0.0,
            cdelt1: std::f64::NAN,
            crpix1: 0.0,
            cunit1: String::from(""),
            ctype1: String::from(""),
            crval2: 0.0,
            cdelt2: std::f64::NAN,
            crpix2: 0.0,
            cunit2: String::from(""),
            ctype2: String::from(""),
            crval3: 0.0,
            cdelt3: 1.0, //std::f64::NAN,
            crpix3: 0.0,
            cunit3: String::from(""),
            ctype3: String::from(""),
            cd1_1: std::f64::NAN,
            cd1_2: std::f64::NAN,
            cd2_1: std::f64::NAN,
            cd2_2: std::f64::NAN,
            frame_min: Vec::new(),
            frame_max: Vec::new(),
            dmin: std::f32::MAX, //no mistake here
            dmax: std::f32::MIN, //no mistake here
            data_hist: RwLock::new(Vec::new()),
            data_median: RwLock::new(0.0),
            data_mad: RwLock::new(0.0),
            data_mad_p: RwLock::new(0.0),
            data_mad_n: RwLock::new(0.0),
            pmin: std::f32::MIN,
            pmax: std::f32::MAX,
            lmin: (0.5f32).ln(),
            lmax: (1.5f32).ln(),
            hist: Vec::new(),
            median: 0.0,
            mad: 0.0,
            mad_p: 0.0,
            mad_n: 0.0,
            black: 0.0,
            white: 0.0,
            sensitivity: 0.0,
            ratio_sensitivity: 0.0,
            flux: flux.clone(),
            has_frequency: false,
            has_velocity: false,
            frame_multiplier: 1.0,
            has_header: false,
            has_data: false,
            timestamp: RwLock::new(SystemTime::now()),
            is_optical: true,
            is_xray: false,
            is_dummy: true,
            status_code: 404,
        };

        fits
    }

    //a parallel multi-threaded read from the FITS file or half-float cache
    fn read_from_fits_or_cache_par(
        &mut self,
        filepath: &std::path::Path,
        header_offset: usize,
        frame_size: usize,
        is_cache: bool,
        cdelt3: f32,
        server: &Addr<server::SessionServer>,
    ) -> bool {
        //load data from filepath
        let f = match File::open(filepath) {
            Ok(x) => x,
            Err(err) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, err);
                return false;
            }
        };

        let total = self.depth;
        let frame_count: AtomicIsize = AtomicIsize::new(0);

        let watch = Instant::now();

        let num_threads = if is_cache {
            num_cpus::get_physical()
        } else {
            //reduce the number of threads for NFS
            num::clamp(num_cpus::get_physical(), 1, 8)
        };

        let pool = match rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
        {
            Ok(pool) => pool,
            Err(err) => {
                println!(
                    "{:?}, switching to a non-optimised function read_from_cache()",
                    err
                );
                return false;
            }
        };

        println!("custom thread pool: {:?}", pool);

        //set up thread-local vectors
        let mut thread_pixels: Vec<RwLock<Vec<f32>>> = Vec::with_capacity(num_threads);
        let mut thread_mask: Vec<RwLock<Vec<u8>>> = Vec::with_capacity(num_threads);

        let mut thread_min: Vec<atomic::Atomic<f32>> = Vec::with_capacity(num_threads);
        let mut thread_max: Vec<atomic::Atomic<f32>> = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            thread_pixels.push(RwLock::new(vec![0.0; self.pixels.len()]));
            thread_mask.push(RwLock::new(vec![0; self.mask.len()]));

            thread_min.push(atomic::Atomic::new(std::f32::MAX));
            thread_max.push(atomic::Atomic::new(std::f32::MIN));
        }

        let mut thread_mean_spectrum: Vec<atomic::Atomic<f32>> =
            Vec::with_capacity(self.depth as usize);
        let mut thread_integrated_spectrum: Vec<atomic::Atomic<f32>> =
            Vec::with_capacity(self.depth as usize);
        let mut thread_frame_min: Vec<atomic::Atomic<f32>> =
            Vec::with_capacity(self.depth as usize);
        let mut thread_frame_max: Vec<atomic::Atomic<f32>> =
            Vec::with_capacity(self.depth as usize);

        for _ in 0..self.depth {
            thread_mean_spectrum.push(atomic::Atomic::new(0.0));
            thread_integrated_spectrum.push(atomic::Atomic::new(0.0));

            thread_frame_min.push(atomic::Atomic::new(std::f32::MAX));
            thread_frame_max.push(atomic::Atomic::new(std::f32::MIN));
        }

        //at first fill-in the self.data_f16 vector in parallel
        let gather_f16: Vec<_> = pool.install(|| {
            (0..self.depth)
                .into_par_iter()
                .map(|frame| {
                    //frame is i32
                    let offset = header_offset + (frame as usize) * frame_size;
                    let mut data_u8: Vec<u8> = vec![0; frame_size];

                    //parallel read at offset
                    let bytes_read = match f.read_at(offset as u64, &mut data_u8) {
                        Ok(size) => size,
                        Err(_err) => {
                            // print!("read_at error: {}", err);
                            // 0

                            // resize the data_u8 vector to 0
                            data_u8.resize(0, 0);

                            // read data in 256KB chunks
                            let chunk_size: usize = 256 * 1024;
                            let mut bytes_read = 0;

                            for chunk_offset in (offset..offset + frame_size).step_by(chunk_size) {
                                let buf_size = chunk_size.min(offset + frame_size - chunk_offset);
                                let mut data_chunk: Vec<u8> = vec![0; buf_size];

                                bytes_read += match f.read_at(chunk_offset as u64, &mut data_chunk)
                                {
                                    Ok(size) => {
                                        data_u8.extend_from_slice(&data_chunk);
                                        size
                                    }
                                    Err(err) => {
                                        print!(
                                            "read_at error: {}, chunk_offset: {}, buf_size: {}",
                                            err, chunk_offset, buf_size
                                        );
                                        0
                                    }
                                };
                            }

                            if bytes_read != frame_size {
                                println!(
                                    "read_at error: {} bytes read, {} bytes expected",
                                    bytes_read, frame_size
                                );
                                0
                            } else {
                                bytes_read
                            }
                        }
                    };

                    let tid = match pool.current_thread_index() {
                        Some(tid) => tid,
                        None => 0,
                    };

                    //println!("tid: {}, frame: {}, offset: {}, bytes read: {}", tid, frame, offset, bytes_read);

                    if bytes_read != frame_size {
                        println!(
                            "CRITICAL ERROR {:?}: read {} bytes @ frame {}, requested {} bytes",
                            filepath, bytes_read, frame, frame_size
                        );
                    };

                    let len = data_u8.len();

                    let data_f16: Vec<f16> = if is_cache {
                        //need to mutate data_u8 into vec_f16
                        let ptr = data_u8.as_ptr() as *mut f16;
                        let capacity = data_u8.capacity();

                        //half-float occupies 2 bytes
                        unsafe { Vec::from_raw_parts(ptr, len / 2, capacity / 2) }
                    } else {
                        //float32 takes up 4 bytes
                        let no_bytes = (self.bitpix.abs() / 8) as usize;
                        vec![f16::from_f32(0.0); len / no_bytes]
                    };

                    //parallel data processing
                    let mut frame_min = std::f32::MAX;
                    let mut frame_max = std::f32::MIN;

                    let mut mean_spectrum = 0.0_f32;
                    let mut integrated_spectrum = 0.0_f32;

                    let mut references: [f32; 4] =
                        [frame_min, frame_max, mean_spectrum, integrated_spectrum];

                    let mut pixels = thread_pixels[tid].write();
                    let mask = thread_mask[tid].write();

                    let vec_ptr = data_f16.as_ptr() as *mut i16;
                    let vec_len = data_f16.len();

                    let mask_ptr = mask.as_ptr() as *mut u8;
                    let mask_len = mask.len();

                    let vec_raw = unsafe { slice::from_raw_parts_mut(vec_ptr, vec_len) };
                    let mask_raw = unsafe { slice::from_raw_parts_mut(mask_ptr, mask_len) };

                    if is_cache {
                        mem::forget(data_u8);

                        unsafe {
                            spmd::make_image_spectrumF16_minmax(
                                vec_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.ignrval,
                                self.datamin,
                                self.datamax,
                                cdelt3,
                                pixels.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                vec_len as u32,
                                references.as_mut_ptr(),
                            );
                        }
                    } else {
                        unsafe {
                            spmd::make_image_spectrumF32_minmax(
                                data_u8.as_ptr() as *mut i32,
                                vec_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.ignrval,
                                self.datamin,
                                self.datamax,
                                cdelt3,
                                pixels.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                vec_len as u32,
                                references.as_mut_ptr(),
                            );
                        }
                    }

                    frame_min = references[0];
                    frame_max = references[1];
                    mean_spectrum = references[2];
                    integrated_spectrum = references[3];

                    thread_mean_spectrum[frame as usize].store(mean_spectrum, Ordering::SeqCst);
                    thread_integrated_spectrum[frame as usize]
                        .store(integrated_spectrum, Ordering::SeqCst);

                    let current_frame_min = thread_frame_min[frame as usize].load(Ordering::SeqCst);
                    thread_frame_min[frame as usize]
                        .store(frame_min.min(current_frame_min), Ordering::SeqCst);

                    let current_frame_max = thread_frame_max[frame as usize].load(Ordering::SeqCst);
                    thread_frame_max[frame as usize]
                        .store(frame_max.max(current_frame_max), Ordering::SeqCst);

                    let current_min = thread_min[tid].load(Ordering::SeqCst);
                    thread_min[tid].store(frame_min.min(current_min), Ordering::SeqCst);

                    let current_max = thread_max[tid].load(Ordering::SeqCst);
                    thread_max[tid].store(frame_max.max(current_max), Ordering::SeqCst);
                    //end of parallel data processing

                    let previous_frame_count = frame_count.fetch_add(1, Ordering::SeqCst) as i32;
                    let current_frame_count = previous_frame_count + 1;
                    self.send_progress_notification(
                        &server,
                        &"loading FITS".to_owned(),
                        total as i32,
                        current_frame_count,
                    );

                    data_f16
                })
                .collect()
        });

        self.data_f16 = gather_f16;

        self.frame_min = thread_frame_min
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        self.frame_max = thread_frame_max
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        self.mean_spectrum = thread_mean_spectrum
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        self.integrated_spectrum = thread_integrated_spectrum
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        //then fuse self.pixels and self.mask with the local thread versions
        for tid in 0..num_threads {
            self.dmin = self.dmin.min(thread_min[tid].load(Ordering::SeqCst));
            self.dmax = self.dmax.max(thread_max[tid].load(Ordering::SeqCst));

            let mut pixels_tid = thread_pixels[tid].write();

            let mask_tid = thread_mask[tid].read();
            let mask_tid_ptr = mask_tid.as_ptr() as *mut u8;

            let mask_ptr = self.mask.as_ptr() as *mut u8;

            let total_size = self.pixels.len();

            unsafe {
                let mask_raw = slice::from_raw_parts_mut(mask_ptr, total_size);
                let mask_tid_raw = slice::from_raw_parts_mut(mask_tid_ptr, total_size);

                spmd::join_pixels_masks(
                    self.pixels.as_mut_ptr(),
                    pixels_tid.as_mut_ptr(),
                    mask_raw.as_mut_ptr(),
                    mask_tid_raw.as_mut_ptr(),
                    cdelt3,
                    total_size as u32,
                );
            }
        }

        println!("[read_from_cache_par] elapsed time: {:?}", watch.elapsed());

        true
    }

    //a parallel multi-threaded read from the zfp-compressed cache
    #[cfg(feature = "zfp")]
    fn zfp_decompress(
        &mut self,
        id: &String,
        cdelt3: f32,
        server: &Addr<server::SessionServer>,
    ) -> bool {
        use bincode::error::DecodeError;

        #[cfg(not(feature = "raid"))]
        {
            let filename = format!("{}/{}.zfp", FITSCACHE, id.replace("/", "_"));
            let zfp_dir = std::path::Path::new(&filename);
            let mut zfp_ok = std::path::PathBuf::from(zfp_dir);
            zfp_ok.push(".ok");

            if !zfp_ok.exists() {
                return false;
            }
        }

        #[cfg(feature = "raid")]
        {
            let mut zfp_exists = true;

            for raid_volume in 0..RAID_COUNT {
                let filename = format!(
                    "{}{}/{}/{}.zfp",
                    RAID_PREFIX,
                    raid_volume,
                    FITSCACHE,
                    self.dataset_id.replace("/", "_")
                );
                let zfp_dir = std::path::Path::new(&filename);

                let mut zfp_ok = std::path::PathBuf::from(zfp_dir);
                zfp_ok.push(".ok");

                zfp_exists = zfp_exists && zfp_ok.exists();
            }

            // cache files do not exist in RAID-0 volumes
            if !zfp_exists {
                return false;
            }
        }

        let total = self.depth;
        let frame_count: AtomicIsize = AtomicIsize::new(0);
        let success: AtomicBool = AtomicBool::new(true);

        let watch = Instant::now();

        let num_threads = num_cpus::get_physical();

        let pool = match rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
        {
            Ok(pool) => pool,
            Err(err) => {
                println!(
                    "{:?}, switching to a non-optimised function read_from_cache()",
                    err
                );
                return false;
            }
        };

        println!("custom thread pool: {:?}", pool);

        //set up thread-local vectors
        let mut thread_pixels: Vec<RwLock<Vec<f32>>> = Vec::with_capacity(num_threads);
        let mut thread_mask: Vec<RwLock<Vec<u8>>> = Vec::with_capacity(num_threads);

        let mut thread_min: Vec<atomic::Atomic<f32>> = Vec::with_capacity(num_threads);
        let mut thread_max: Vec<atomic::Atomic<f32>> = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            thread_pixels.push(RwLock::new(vec![0.0; self.pixels.len()]));
            thread_mask.push(RwLock::new(vec![0; self.mask.len()]));

            thread_min.push(atomic::Atomic::new(std::f32::MAX));
            thread_max.push(atomic::Atomic::new(std::f32::MIN));
        }

        let mut thread_mean_spectrum: Vec<atomic::Atomic<f32>> =
            Vec::with_capacity(self.depth as usize);
        let mut thread_integrated_spectrum: Vec<atomic::Atomic<f32>> =
            Vec::with_capacity(self.depth as usize);
        let mut thread_frame_min: Vec<atomic::Atomic<f32>> =
            Vec::with_capacity(self.depth as usize);
        let mut thread_frame_max: Vec<atomic::Atomic<f32>> =
            Vec::with_capacity(self.depth as usize);

        for _ in 0..self.depth {
            thread_mean_spectrum.push(atomic::Atomic::new(0.0));
            thread_integrated_spectrum.push(atomic::Atomic::new(0.0));

            thread_frame_min.push(atomic::Atomic::new(std::f32::MAX));
            thread_frame_max.push(atomic::Atomic::new(std::f32::MIN));
        }

        //at first fill-in the self.data_f16 vector in parallel
        let gather_f16: Vec<_> = pool.install(|| {
            (0..self.depth)
                .into_par_iter()
                .map(|frame| {
                    let data_f16: Vec<f16> = vec![f16::from_f32(0.0); self.width * self.height];

                    #[cfg(feature = "raid")]
                    let raid_volume = frame % RAID_COUNT;

                    #[cfg(feature = "raid")]
                    let filename = format!(
                        "{}{}/{}/{}.zfp",
                        RAID_PREFIX,
                        raid_volume,
                        FITSCACHE,
                        id.replace("/", "_")
                    );

                    #[cfg(not(feature = "raid"))]
                    let filename = format!("{}/{}.zfp", FITSCACHE, id.replace("/", "_"));

                    let zfp_dir = std::path::Path::new(&filename);

                    //zfp-decompress a frame
                    let mut cache_file = std::path::PathBuf::from(zfp_dir);
                    cache_file.push(format!("{}.bin", frame));

                    match File::open(cache_file) {
                        Ok(mut f) => {
                            //let mut buffer = std::io::BufReader::new(f);//BufReader is slower
                            let mut buffer = Vec::new();
                            match f.read_to_end(&mut buffer) {
                                Ok(_) => {
                                    let res: Result<(ZFPMaskedArray, _), DecodeError> =
                                        decode_from_slice(&mut buffer, config::legacy());
                                    match res {
                                        Ok((zfp_frame, _)) => {
                                            //decompress a ZFPMaskedArray object
                                            match zfp_decompress_float_array2d(
                                                zfp_frame.array,
                                                self.width,
                                                self.height,
                                                //zfp_frame.precision,
                                                zfp_frame.rate,
                                            ) {
                                                Ok(mut array) => {
                                                    match lz4_compress::decompress(&zfp_frame.mask)
                                                    {
                                                        Ok(mask) => {
                                                            //parallel data processing
                                                            let frame_min = zfp_frame.frame_min;
                                                            let frame_max = zfp_frame.frame_max;
                                                            let mut mean_spectrum = 0.0_f32;
                                                            let mut integrated_spectrum = 0.0_f32;

                                                            let tid =
                                                                match pool.current_thread_index() {
                                                                    Some(tid) => tid,
                                                                    None => 0,
                                                                };

                                                            /*println!(
                                                                "tid: {}, frame_min: {}, frame_max: {}, self.width: {}, self.height: {}",
                                                                tid, frame_min, frame_max, self.width, self.height
                                                            );

                                                            if frame == self.depth / 2 {
                                                                println!("array: {:?}", array);
                                                                //println!("mask: {:?}", mask);
                                                            }*/

                                                            //convert the (array,mask) into f16
                                                            let mut references: [f32; 2] = [
                                                                mean_spectrum,
                                                                integrated_spectrum,
                                                            ];

                                                            let vec_ptr =
                                                                data_f16.as_ptr() as *mut i16;
                                                            let vec_len = data_f16.len();

                                                            let mut pixels =
                                                                thread_pixels[tid].write();
                                                            let dst_mask = thread_mask[tid].write();
                                                            let src_mask_ptr =
                                                                mask.as_ptr() as *mut u8;
                                                            let dst_mask_ptr =
                                                                dst_mask.as_ptr() as *mut u8;

                                                            unsafe {
                                                                spmd::make_image_spectrumF32_2_F16(
                                                                    array.as_mut_ptr(),
                                                                    src_mask_ptr,
                                                                    frame_min,
                                                                    frame_max,
                                                                    vec_ptr,
                                                                    self.bzero,
                                                                    self.bscale,
                                                                    cdelt3,
                                                                    pixels.as_mut_ptr(),
                                                                    dst_mask_ptr,
                                                                    vec_len as u32,
                                                                    references.as_mut_ptr(),
                                                                );
                                                            }

                                                            mean_spectrum = references[0];
                                                            integrated_spectrum = references[1];

                                                            thread_mean_spectrum[frame as usize]
                                                                .store(
                                                                    mean_spectrum,
                                                                    Ordering::SeqCst,
                                                                );
                                                            thread_integrated_spectrum
                                                                [frame as usize]
                                                                .store(
                                                                    integrated_spectrum,
                                                                    Ordering::SeqCst,
                                                                );

                                                            let current_frame_min =
                                                                thread_frame_min[frame as usize]
                                                                    .load(Ordering::SeqCst);
                                                            thread_frame_min[frame as usize].store(
                                                                frame_min.min(current_frame_min),
                                                                Ordering::SeqCst,
                                                            );

                                                            let current_frame_max =
                                                                thread_frame_max[frame as usize]
                                                                    .load(Ordering::SeqCst);
                                                            thread_frame_max[frame as usize].store(
                                                                frame_max.max(current_frame_max),
                                                                Ordering::SeqCst,
                                                            );

                                                            let current_min = thread_min[tid]
                                                                .load(Ordering::SeqCst);
                                                            thread_min[tid].store(
                                                                frame_min.min(current_min),
                                                                Ordering::SeqCst,
                                                            );

                                                            let current_max = thread_max[tid]
                                                                .load(Ordering::SeqCst);
                                                            thread_max[tid].store(
                                                                frame_max.max(current_max),
                                                                Ordering::SeqCst,
                                                            );
                                                            //end of parallel data processing

                                                            let previous_frame_count = frame_count
                                                                .fetch_add(1, Ordering::SeqCst)
                                                                as i32;
                                                            let current_frame_count =
                                                                previous_frame_count + 1;
                                                            self.send_progress_notification(
                                                                &server,
                                                                &"loading FITS".to_owned(),
                                                                total as i32,
                                                                current_frame_count,
                                                            );
                                                        }
                                                        Err(err) => {
                                                            println!("{}", err);
                                                            success
                                                                .fetch_and(false, Ordering::SeqCst);
                                                        }
                                                    }
                                                }
                                                Err(err) => {
                                                    println!("{}", err);
                                                    success.fetch_and(false, Ordering::SeqCst);
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            println!("CRITICAL ERROR deserialize: {:?}", err);
                                            success.fetch_and(false, Ordering::SeqCst);
                                        }
                                    }
                                }
                                Err(err) => {
                                    println!("CRITICAL ERROR cannot read from file: {:?}", err);
                                    success.fetch_and(false, Ordering::SeqCst);
                                }
                            }
                        }
                        Err(err) => {
                            println!("CRITICAL ERROR: {:?}", err);
                            success.fetch_and(false, Ordering::SeqCst);
                        }
                    };

                    data_f16
                })
                .collect()
        });

        self.data_f16 = gather_f16;

        self.frame_min = thread_frame_min
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        self.frame_max = thread_frame_max
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        self.mean_spectrum = thread_mean_spectrum
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        self.integrated_spectrum = thread_integrated_spectrum
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        //then fuse self.pixels and self.mask with the local thread versions
        for tid in 0..num_threads {
            self.dmin = self.dmin.min(thread_min[tid].load(Ordering::SeqCst));
            self.dmax = self.dmax.max(thread_max[tid].load(Ordering::SeqCst));

            let mut pixels_tid = thread_pixels[tid].write();

            let mask_tid = thread_mask[tid].read();
            let mask_tid_ptr = mask_tid.as_ptr() as *mut u8;

            let mask_ptr = self.mask.as_ptr() as *mut u8;

            let total_size = self.pixels.len();

            unsafe {
                let mask_raw = slice::from_raw_parts_mut(mask_ptr, total_size);
                let mask_tid_raw = slice::from_raw_parts_mut(mask_tid_ptr, total_size);

                spmd::join_pixels_masks(
                    self.pixels.as_mut_ptr(),
                    pixels_tid.as_mut_ptr(),
                    mask_raw.as_mut_ptr(),
                    mask_tid_raw.as_mut_ptr(),
                    cdelt3,
                    total_size as u32,
                );
            }
        }

        println!("[zfp_decompress] elapsed time: {:?}", watch.elapsed());

        success.load(Ordering::SeqCst)
    }

    fn read_from_cache(
        &mut self,
        filepath: &std::path::Path,
        frame_size: usize,
        cdelt3: f32,
        server: &Addr<server::SessionServer>,
    ) -> bool {
        //load data from filepath
        let mut f = match File::open(filepath) {
            Ok(x) => x,
            Err(err) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, err);
                return false;
            }
        };

        let total = self.depth;

        let watch = Instant::now();

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let mut frame: usize = 0;

            while frame < total {
                //println!("requesting a cube frame {}/{}", frame, fits.depth);
                let mut data: Vec<u8> = vec![0; frame_size];

                //read a FITS cube frame (half-float only)
                match f.read_exact(&mut data) {
                    Ok(()) => {
                        //println!("sending a cube frame for processing {}/{}", frame+1, total);
                        //send data for processing -> tx
                        match tx.send(data) {
                            Ok(()) => {}
                            Err(err) => {
                                println!("file reading thread: {}", err);
                                return;
                            }
                        };

                        frame = frame + 1;
                    }
                    Err(err) => {
                        println!("CRITICAL ERROR reading FITS data: {}", err);
                        return;
                    }
                };
            }
        });

        let mut frame: usize = 0;

        for data in rx {
            let len = data.len() / 2;
            let mut sum: f32 = 0.0;
            let mut count: i32 = 0;

            //no mistake here, the initial ranges are supposed to be broken
            let mut frame_min = std::f32::MAX;
            let mut frame_max = std::f32::MIN;

            {
                let mut rdr = Cursor::new(data);
                //let vec = self.data_f16.get_mut(frame as usize).unwrap() ;
                let mut vec: Vec<f16> = Vec::with_capacity(len);

                for i in 0..len {
                    match rdr.read_u16::<LittleEndian>() {
                        Ok(u16) => {
                            let float16 = f16::from_bits(u16);
                            vec.push(float16);

                            let tmp = self.bzero + self.bscale * float16.to_f32();
                            if tmp.is_finite()
                                && tmp >= self.datamin
                                && tmp <= self.datamax
                                && tmp > self.ignrval
                            {
                                self.pixels[i as usize] += tmp * cdelt3;
                                self.mask[i as usize] = 255;

                                frame_min = frame_min.min(tmp);
                                frame_max = frame_max.max(tmp);

                                sum += tmp;
                                count += 1;
                            }
                        }
                        Err(err) => println!(
                            "LittleEndian --> LittleEndian u16 conversion error: {}",
                            err
                        ),
                    }
                }

                self.data_f16[frame as usize] = vec;
            }

            self.dmin = self.dmin.min(frame_min);
            self.dmax = self.dmax.max(frame_max);

            //mean and integrated intensities @ frame
            if count > 0 {
                self.mean_spectrum[frame as usize] = sum / (count as f32);
                self.integrated_spectrum[frame as usize] = sum * cdelt3;
            }

            frame = frame + 1;
            self.send_progress_notification(
                &server,
                &"processing FITS".to_owned(),
                total as i32,
                frame as i32,
            );
        }

        if frame != total {
            println!(
                "CRITICAL ERROR not all FITS cube frames have been read: {}/{}",
                frame, total
            );
            return false;
        };

        println!("[read_from_cache] elapsed time: {:?}", watch.elapsed());

        return true;
    }

    pub fn from_path(
        id: &String,
        flux: &String,
        filepath: &std::path::Path,
        url: &String,
        server: &Addr<server::SessionServer>,
    ) -> FITS {
        let mut fits = FITS::new(id, url, flux);
        fits.is_dummy = false;

        //load data from filepath
        let mut f = match File::open(filepath) {
            Ok(x) => x,
            Err(x) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, x);
                fits.status_code = 500;

                #[cfg(not(feature = "jvo"))]
                return fits;

                //a desperate attempt to download FITS using the ALMA URL (will fail for non-ALMA datasets)
                #[cfg(feature = "jvo")]
                {
                    let url = format!(
                        "http://{}:8060/skynode/getDataForALMA.do?db={}&table=cube&data_id={}_00_00_00",
                        JVO_FITS_SERVER, JVO_FITS_DB, id
                    );

                    return FITS::from_url(&id, &flux, &url, &server);
                }
            }
        };

        match f.metadata() {
            Ok(metadata) => {
                let len = metadata.len();

                println!("{:?}, {} bytes", f, len);

                fits.filesize = len;

                if len < FITS_CHUNK_LENGTH as u64 {
                    fits.status_code = 500;
                    return fits;
                };
            }
            Err(err) => {
                println!("CRITICAL ERROR file metadata reading problem: {}", err);
                fits.status_code = 500;
                return fits;
            }
        };

        let is_gzip = is_gzip_compressed(&mut f);
        let is_bzip2 = is_bzip2_compressed(&mut f);
        let is_compressed = is_bzip2 || is_gzip;

        println!(
            "is_gzip: {}, is_bzip2: {}, is_compressed: {}",
            is_gzip, is_bzip2, is_compressed
        );

        //OK, we have a FITS file with at least one chunk
        println!("{}: reading a FITS file header...", id);

        let mut f: Box<dyn Read + Send> = if is_compressed {
            if is_gzip {
                Box::new(GzDecoder::new(f))
            } else if is_bzip2 {
                Box::new(BzDecoder::new(f))
            } else {
                //this path is dummy, it will never be reached
                Box::new(f)
            }
        } else {
            Box::new(f)
        };

        let mut header: Vec<u8> = Vec::new();
        let mut no_hdu: i32 = 0;

        //try many times until the right header has been found
        while fits.naxis == 0 {
            header = Vec::new();
            let mut end: bool = false;

            while !end {
                //read a FITS chunk
                let mut chunk = [0; FITS_CHUNK_LENGTH];

                match f.read_exact(&mut chunk) {
                    Ok(()) => {
                        no_hdu = no_hdu + 1;

                        //parse a FITS header chunk
                        match fits.parse_fits_header_chunk(&chunk) {
                            Ok(x) => end = x,
                            Err(err) => {
                                println!("CRITICAL ERROR parsing FITS header: {}", err);
                                fits.status_code = 415;
                                return fits;
                            }
                        };

                        header.extend_from_slice(&chunk);
                    }
                    Err(err) => {
                        println!("CRITICAL ERROR reading FITS header: {}", err);
                        fits.status_code = 500;
                        return fits;
                    }
                };
            }
        }

        //test for frequency/velocity
        fits.frame_reference_unit();
        fits.frame_reference_type();

        if fits.restfrq > 0.0 {
            fits.has_frequency = true;
        }

        if fits.has_frequency || fits.has_velocity {
            fits.is_optical = false;
        }

        fits.has_header = true;

        {
            let fits = Arc::new(RwLock::new(Box::new(fits.clone())));
            DATASETS.write().insert(id.clone(), fits.clone());
        }

        println!("{}/#hdu = {}, {:?}", id, no_hdu, fits);

        fits.header = match String::from_utf8(header) {
            Ok(x) => x,
            Err(err) => {
                println!("FITS HEADER UTF8: {}", err);
                fits.status_code = 500;
                String::from("")
            }
        };

        //compress the FITS header
        /*if !header.is_empty() {
            fits.compressed_header = lz4_compress::compress(&header);
            println!("FITS header length {}, lz4-compressed {} bytes", header.len(), fits.compressed_header.len());
        }*/

        //next read the data HUD(s)
        let frame_size: usize = fits.init_data_storage();

        //let mut f = BufReader::with_capacity(frame_size, f);

        println!("FITS cube frame size: {} bytes", frame_size);

        let total = fits.depth;

        let cdelt3 = {
            if fits.has_velocity && fits.depth > 1 {
                fits.cdelt3 * fits.frame_multiplier / 1000.0
            } else {
                1.0
            }
        };

        println!("setting cdelt3 to {}", cdelt3);

        //check if bitpix == -32 and the F16 half-float cache file exists
        let filename = format!("{}/{}.bin", FITSCACHE, id.replace("/", "_"));
        let binpath = std::path::Path::new(&filename);

        #[cfg(not(feature = "zfp"))]
        let read_from_zfp = false;

        #[cfg(feature = "zfp")]
        let read_from_zfp = {
            if fits.bitpix == -32 {
                println!(
                    "{}: reading zfp-compressed half-float f16 data from cache",
                    id
                );

                fits.zfp_decompress(id, cdelt3 as f32, &server)
            } else {
                false
            }
        };

        if !read_from_zfp {
            if fits.bitpix == -32 && binpath.exists() {
                println!("{}: reading half-float f16 data from cache", id);

                if !fits.read_from_fits_or_cache_par(
                    binpath,
                    0,
                    frame_size / 2,
                    true,
                    cdelt3 as f32,
                    &server,
                ) {
                    println!("CRITICAL ERROR parallel reading from half-float cache");
                    fits.status_code = 500;

                    if !fits.read_from_cache(binpath, frame_size / 2, cdelt3 as f32, &server) {
                        println!("CRITICAL ERROR reading from half-float cache");
                        fits.status_code = 500;
                        return fits;
                    }
                }
            } else {
                if fits.bitpix == -32 && fits.depth > 1 && !is_compressed {
                    let offset = (no_hdu as usize) * FITS_CHUNK_LENGTH;
                    println!(
                        "{}: reading FITS data in parallel at an offset of {} bytes",
                        id, offset
                    );

                    if !fits.read_from_fits_or_cache_par(
                        filepath,
                        offset,
                        frame_size,
                        false,
                        cdelt3 as f32,
                        &server,
                    ) {
                        println!("CRITICAL ERROR reading from FITS file");
                        fits.status_code = 500;
                        return fits;
                    }
                } else {
                    //sequential reading
                    let (tx, rx) = mpsc::channel();

                    thread::spawn(move || {
                        let mut frame: usize = 0;

                        while frame < total {
                            //println!("requesting a cube frame {}/{}", frame, fits.depth);
                            let mut data: Vec<u8> = vec![0; frame_size];

                            //read a FITS cube frame
                            match f.read_exact(&mut data) {
                                Ok(()) => {
                                    //println!("sending a cube frame for processing {}/{}", frame+1, total);
                                    //send data for processing -> tx
                                    match tx.send(data) {
                                        Ok(()) => {}
                                        Err(err) => {
                                            println!("file reading thread: {}", err);
                                            return;
                                        }
                                    };

                                    frame = frame + 1;
                                }
                                Err(err) => {
                                    println!("CRITICAL ERROR reading FITS data: {}", err);
                                    return;
                                }
                            };
                        }
                    });

                    let mut frame: usize = 0;

                    for data in rx {
                        fits.process_cube_frame(&data, cdelt3 as f32, frame);
                        frame = frame + 1;
                        fits.send_progress_notification(
                            &server,
                            &"processing FITS".to_owned(),
                            total as i32,
                            frame as i32,
                        );
                    }

                    if frame != fits.depth {
                        println!(
                            "CRITICAL ERROR not all FITS cube frames have been read: {}/{}",
                            frame, fits.depth
                        );
                        fits.status_code = 500;
                        return fits;
                    }
                }
            }
        }

        //println!("mean spectrum: {:?}", fits.mean_spectrum);
        //println!("integrated spectrum: {:?}", fits.integrated_spectrum);

        //we've gotten so far, we have the data, pixels, mask and spectrum
        fits.has_data = true;
        fits.status_code = 200;

        if !fits.pixels.is_empty() && !fits.mask.is_empty() {
            //apply std::f32::NAN to masked pixels
            let mut ord_pixels: Vec<f32> = fits
                .pixels
                .par_iter()
                .zip(fits.mask.par_iter())
                .map(|(x, m)| if *m > 0 { *x } else { std::f32::NAN })
                .collect();

            //sort the pixels in parallel using rayon
            //let mut ord_pixels = fits.pixels.clone();
            //println!("unordered pixels: {:?}", ord_pixels);

            let watch = Instant::now();

            //ord_pixels.par_sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
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

            println!("[pixels] parallel sorting time: {:?}", watch.elapsed());

            fits.make_image_histogram(&ord_pixels);

            if fits.flux == "" {
                fits.histogram_classifier();
            };

            fits.make_vpx_image(); //was _vpx_, _j2k_, _wavelet_
        };

        fits.send_progress_notification(&server, &"processing FITS done".to_owned(), 0, 0);
        println!("{}: reading FITS data completed", id);

        //and lastly create a symbolic link in the FITSCACHE directory
        //#[cfg(not(feature = "jvo"))]
        {
            let filename = format!("{}/{}.fits", FITSCACHE, id.replace("/", "_"));
            let cachefile = std::path::Path::new(&filename);
            let _ = std::os::unix::fs::symlink(filepath, cachefile);
        }

        fits
    }

    pub fn from_url(
        id: &String,
        flux: &String,
        url: &String,
        server: &Addr<server::SessionServer>,
    ) -> FITS {
        let mut fits = FITS::new(id, url, flux);
        fits.is_dummy = false;

        println!("FITS::from_url({})", url);

        let tmp = format!("{}/{}.fits.tmp", FITSCACHE, id.replace("/", "_"));

        let mut cachefile = match File::create(&tmp) {
            Err(ref e) => {
                println!("Could not create {} ({})!", tmp, e);
                fits.status_code = 500;
                return fits;
            }
            Ok(file) => file,
        };

        /*let mut easy = Easy2::new(fits);
        easy.get(true).unwrap();
        easy.url(url).unwrap();
        easy.perform().unwrap();*/

        let mut buffer = Vec::new();
        let mut easy = Easy::new();

        //enable automatic URL relocations
        match easy.follow_location(true) {
            Ok(_) => {}
            Err(err) => println!("curl::follow_location: {}", err),
        }

        match easy.fail_on_error(true) {
            Ok(_) => {}
            Err(err) => println!("curl::fail_on_error: {}", err),
        }

        let mut header: Vec<u8> = Vec::new();
        let mut end: bool = false;
        let mut no_hdu: i32 = 0;
        let mut frame: usize = 0;
        let mut frame_size: usize = 0;

        let mut total = 0;
        let mut cdelt3 = 0.0;

        let mut is_compressed = false;
        let mut compression_checked = false;

        type BzStream = BzDecompressor<Vec<u8>>;
        type GzStream = GzDecompressor<Vec<u8>>;

        let mut bz_decoder: Option<Rc<RefCell<BzStream>>> = None;
        let mut gz_decoder: Option<Rc<RefCell<GzStream>>> = None;

        {
            easy.url(url).unwrap();
            easy.progress(true).unwrap();

            let mut transfer = easy.transfer();

            transfer
                .write_function(|data| {
                    //println!("curl received {} bytes", data.len());
                    fits.filesize = fits.filesize + data.len() as u64;

                    match cachefile.write_all(data) {
                        Ok(_) => {}
                        Err(err) => {
                            println!("cannot append to the temporary FITS file: {}", err);
                            fits.status_code = 500;
                        }
                    };

                    if !is_compressed {
                        buffer.extend_from_slice(data);
                    } else {
                        match &bz_decoder {
                            Some(decoder) => {
                                let mut decoder = decoder.borrow_mut();
                                match decoder.write_all(&data) {
                                    Ok(_) => {
                                        decoder.flush().unwrap();
                                        let out = decoder.get_mut();
                                        let len = out.len();
                                        if len > 0 {
                                            buffer.extend_from_slice(out);
                                            out.drain(0..out.len());
                                        }
                                    }
                                    Err(err) => {
                                        println!("Decompress: {}", err);
                                        fits.status_code = 500;
                                    }
                                }
                            }
                            None => {}
                        }

                        match &gz_decoder {
                            Some(decoder) => {
                                let mut decoder = decoder.borrow_mut();
                                match decoder.write_all(&data) {
                                    Ok(_) => {
                                        decoder.flush().unwrap();
                                        let out = decoder.get_mut();
                                        let len = out.len();
                                        if len > 0 {
                                            buffer.extend_from_slice(out);
                                            out.drain(0..out.len());
                                        }
                                    }
                                    Err(err) => {
                                        println!("Decompress: {}", err);
                                        fits.status_code = 500;
                                    }
                                }
                            }
                            None => {}
                        }
                    }

                    if !compression_checked && buffer.len() >= 10 {
                        print!(
                            "buffer length: {}, checking for compression...",
                            buffer.len()
                        );
                        //test for magick numbers and the deflate compression type
                        if buffer[0] == 0x1f && buffer[1] == 0x8b && buffer[2] == 0x08 {
                            let mut decoder = GzDecompressor::new(Vec::new());
                            is_compressed = true;
                            println!("gzip found.");

                            //decompress the incoming data
                            match decoder.write_all(&buffer) {
                                Ok(_) => {
                                    buffer.drain(0..buffer.len());
                                    decoder.flush().unwrap();
                                    let out = decoder.get_mut();
                                    let len = out.len();
                                    if len > 0 {
                                        buffer.extend_from_slice(out);
                                        out.drain(0..out.len());
                                    }
                                }
                                Err(err) => {
                                    println!("Decompress: {}", err);
                                    fits.status_code = 500;
                                }
                            }

                            gz_decoder = Some(Rc::new(RefCell::new(decoder)));
                        } else
                        //test for magick numbers and the bzip2 compression type
                        if buffer[0] == 0x42 && buffer[1] == 0x5a && buffer[2] == 0x68 {
                            let mut decoder = BzDecompressor::new(Vec::new());
                            is_compressed = true;
                            println!("bzip2 found.");

                            //decompress the incoming data
                            match decoder.write_all(&buffer) {
                                Ok(_) => {
                                    buffer.drain(0..buffer.len());
                                    decoder.flush().unwrap();
                                    let out = decoder.get_mut();
                                    let len = out.len();
                                    if len > 0 {
                                        buffer.extend_from_slice(out);
                                        out.drain(0..out.len());
                                    }
                                }
                                Err(err) => {
                                    println!("Decompress: {}", err);
                                    fits.status_code = 500;
                                }
                            }

                            bz_decoder = Some(Rc::new(RefCell::new(decoder)));
                        } else {
                            println!("no compression found.");
                        }

                        compression_checked = true;
                    }

                    //handle the header first
                    if !fits.has_header {
                        while buffer.len() >= FITS_CHUNK_LENGTH && !fits.has_header {
                            let chunk: Vec<u8> = buffer.drain(0..FITS_CHUNK_LENGTH).collect();

                            no_hdu = no_hdu + 1;

                            //parse a FITS header chunk
                            match fits.parse_fits_header_chunk(&chunk) {
                                Ok(x) => end = x,
                                Err(err) => {
                                    println!("CRITICAL ERROR parsing FITS header: {}", err);
                                    fits.status_code = 415;
                                    //terminate the transfer early
                                    return Ok(0);
                                }
                            };

                            header.extend_from_slice(&chunk);

                            //try again, there may be an image extension
                            if end && fits.naxis == 0 {
                                header = Vec::new();
                                end = false;
                            }

                            if end {
                                //test for frequency/velocity
                                fits.frame_reference_unit();
                                fits.frame_reference_type();

                                if fits.restfrq > 0.0 {
                                    fits.has_frequency = true;
                                }

                                fits.has_header = true;

                                {
                                    let fits = Arc::new(RwLock::new(Box::new(fits.clone())));
                                    DATASETS.write().insert(id.clone(), fits.clone());
                                }

                                println!("{}/#hdu = {}, {:?}", id, no_hdu, fits);

                                fits.header = match String::from_utf8(header.clone()) {
                                    Ok(x) => x,
                                    Err(err) => {
                                        println!("FITS HEADER UTF8: {}", err);
                                        fits.status_code = 500;
                                        String::from("")
                                    }
                                };

                                //prepare for reading the data HUD(s)
                                frame_size = fits.init_data_storage();

                                println!("FITS cube frame size: {} bytes", frame_size);

                                total = fits.depth;

                                cdelt3 = {
                                    if fits.has_velocity && fits.depth > 1 {
                                        fits.cdelt3 * fits.frame_multiplier / 1000.0
                                    } else {
                                        1.0
                                    }
                                }
                            }
                        }
                    } else {
                        //then the data part
                        if !fits.has_data {
                            //kB downloaded progress
                            fits.send_progress_notification(
                                &server,
                                &"downloading FITS".to_owned(),
                                (fits.depth * frame_size / 1024) as i32,
                                ((frame * frame_size + buffer.len().min(frame_size)) / 1024) as i32,
                            );

                            while buffer.len() >= frame_size && !fits.has_data {
                                let data: Vec<u8> = buffer.drain(0..frame_size).collect();

                                fits.process_cube_frame(&data, cdelt3 as f32, frame);
                                frame = frame + 1;

                                if frame == fits.depth {
                                    //all data frames have been received
                                    fits.has_data = true;
                                    fits.status_code = 200;
                                }
                            }
                        }
                    };

                    Ok(data.len())
                })
                .unwrap();

            let _ = transfer.perform();
        }

        println!(
            "{} bytes remaining in the libcurl download buffer; has_data: {}",
            buffer.len(),
            fits.has_data
        );

        if fits.has_data {
            if !fits.pixels.is_empty() && !fits.mask.is_empty() {
                //apply std::f32::NAN to masked pixels
                let mut ord_pixels: Vec<f32> = fits
                    .pixels
                    .par_iter()
                    .zip(fits.mask.par_iter())
                    .map(|(x, m)| if *m > 0 { *x } else { std::f32::NAN })
                    .collect();

                //sort the pixels in parallel using rayon
                //let mut ord_pixels = fits.pixels.clone();
                //println!("unordered pixels: {:?}", ord_pixels);

                let watch = Instant::now();

                //ord_pixels.par_sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
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

                println!("[pixels] parallel sorting time: {:?}", watch.elapsed());

                fits.make_image_histogram(&ord_pixels);

                if fits.flux == "" {
                    fits.histogram_classifier();
                };

                fits.make_vpx_image(); //was _vpx_, _j2k_, _wavelet_
            };

            fits.send_progress_notification(&server, &"downloading FITS done".to_owned(), 0, 0);
            println!("{}: reading FITS data completed", id);
        };

        if fits.filesize >= FITS_CHUNK_LENGTH as u64 {
            if fits.status_code == 200 {
                let filename = format!("{}/{}.fits", FITSCACHE, id.replace("/", "_"));
                let _ = std::fs::rename(tmp, filename);
            } else {
                let _ = std::fs::remove_file(tmp);
            }
        } else {
            fits.status_code = 404;
            fits.send_progress_notification(&server, &"error downloading FITS".to_owned(), 0, 0);
            let _ = std::fs::remove_file(tmp);
        };

        fits
    }

    fn send_progress_notification(
        &self,
        server: &Addr<server::SessionServer>,
        notification: &str,
        total: i32,
        running: i32,
    ) {
        server.do_send(server::WsMessage {
            notification: String::from(notification),
            total: total,
            running: running,
            elapsed: Instant::now().duration_since(self.created),
            dataset_id: self.dataset_id.clone(),
        });
    }

    fn init_data_storage(&mut self) -> usize {
        if self.width == 0 || self.height == 0 || self.depth == 0 {
            return 0;
        }

        let capacity = self.width * self.height;

        self.mask.resize(capacity, 0);
        self.pixels.resize(capacity, 0.0);

        self.mean_spectrum.resize(self.depth as usize, 0.0);
        self.integrated_spectrum.resize(self.depth as usize, 0.0);

        self.frame_min.resize(self.depth as usize, std::f32::MAX);
        self.frame_max.resize(self.depth as usize, std::f32::MIN);

        match self.bitpix {
            8 => self
                .data_u8
                .resize(self.depth as usize, Vec::with_capacity(capacity)),
            16 => self
                .data_i16
                .resize(self.depth as usize, Vec::with_capacity(capacity)),
            32 => self
                .data_i32
                .resize(self.depth as usize, Vec::with_capacity(capacity)),
            //-32 => self.data_f16.resize(self.depth as usize, Vec::with_capacity(capacity as usize)),
            -32 => self.data_f16.resize(self.depth as usize, Vec::new()),
            -64 => self
                .data_f64
                .resize(self.depth as usize, Vec::with_capacity(capacity)),
            _ => println!("unsupported bitpix: {}", self.bitpix),
        }

        capacity * ((self.bitpix.abs() / 8) as usize)
    }

    fn frame_reference_type(&mut self) {
        if self.ctype3.contains("F") || self.ctype3.contains("f") {
            self.has_frequency = true;
        }

        if self.ctype3.contains("V") || self.ctype3.contains("v") {
            self.has_velocity = true;
        }
    }

    fn frame_reference_unit(&mut self) {
        match self.cunit3.to_uppercase().as_ref() {
            "HZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1.0;
            }
            "KHZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1000.0;
            }
            "MHZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1000000.0;
            }
            "GHZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1000000000.0;
            }
            "THZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1000000000000.0;
            }
            "M/S" => {
                self.has_velocity = true;
                self.frame_multiplier = 1.0;
            }
            "KM/S" => {
                self.has_velocity = true;
                self.frame_multiplier = 1000.0;
            }
            _ => {}
        }
    }

    fn modify_partial_fits_header_chunk(
        &self,
        buf: &mut [u8],
        naxes: &[usize],
        x1: f64,
        y1: f64,
        start: f64,
    ) -> bool {
        let mut offset: usize = 0;

        while offset < FITS_CHUNK_LENGTH {
            let slice = &buf[offset..offset + FITS_LINE_LENGTH].to_vec();
            let line = match std::str::from_utf8(slice) {
                Ok(x) => x,
                Err(err) => {
                    println!("non-UTF8 characters found: {}, bytes: {:?}", err, slice);
                    return true;
                }
            };

            if line.contains("END       ") {
                return true;
            }

            if line.contains("NAXIS1  = ") {
                let new_value =
                    format!("NAXIS1  = {} / modified by fits_web_ql", naxes[0]).into_bytes();

                for i in 0..new_value.len().min(FITS_LINE_LENGTH) {
                    buf[offset + i] = new_value[i];
                }
            }

            if line.contains("NAXIS2  = ") {
                let new_value =
                    format!("NAXIS2  = {} / modified by fits_web_ql", naxes[1]).into_bytes();

                for i in 0..new_value.len().min(FITS_LINE_LENGTH) {
                    buf[offset + i] = new_value[i];
                }
            }

            if line.contains("NAXIS3  = ") {
                let new_value =
                    format!("NAXIS3  = {} / modified by fits_web_ql", naxes[2]).into_bytes();

                for i in 0..new_value.len().min(FITS_LINE_LENGTH) {
                    buf[offset + i] = new_value[i];
                }
            }

            if line.contains("NAXIS4  = ") {
                let new_value =
                    format!("NAXIS4  = {} / modified by fits_web_ql", naxes[3]).into_bytes();

                for i in 0..new_value.len().min(FITS_LINE_LENGTH) {
                    buf[offset + i] = new_value[i];
                }
            }

            if line.contains("CRPIX1  = ") {
                let new_value = format!("CRPIX1  = {} / modified by fits_web_ql", self.crpix1 - x1)
                    .into_bytes();

                for i in 0..new_value.len().min(FITS_LINE_LENGTH) {
                    buf[offset + i] = new_value[i];
                }
            }

            if line.contains("CRPIX2  = ") {
                let new_value = format!("CRPIX2  = {} / modified by fits_web_ql", self.crpix2 - y1)
                    .into_bytes();

                for i in 0..new_value.len().min(FITS_LINE_LENGTH) {
                    buf[offset + i] = new_value[i];
                }
            }

            if line.contains("CRPIX3  = ") {
                let new_value = format!(
                    "CRPIX3  = {} / modified by fits_web_ql",
                    self.crpix3 - start
                )
                .into_bytes();

                for i in 0..new_value.len().min(FITS_LINE_LENGTH) {
                    buf[offset + i] = new_value[i];
                }
            }

            offset = offset + FITS_LINE_LENGTH;
        }

        return false;
    }

    fn parse_fits_header_chunk(&mut self, buf: &[u8]) -> Result<bool, &str> {
        let mut offset: usize = 0;

        while offset < FITS_CHUNK_LENGTH {
            let slice = &buf[offset..offset + FITS_LINE_LENGTH];
            let line = match std::str::from_utf8(slice) {
                Ok(x) => x,
                Err(err) => {
                    println!("non-UTF8 characters found: {}", err);
                    return Err("non-UTF8 characters found in the FITS header");
                }
            };

            if line.contains("END       ") {
                return Ok(true);
            }

            if line.contains("TELESCOP= ") {
                let telescope = match scan_fmt_some!(line, "TELESCOP= {}", String) {
                    Some(x) => x.replace("'", "").to_lowercase(),
                    _ => String::from(""),
                };

                self.telescope = telescope;

                println!("telescope: {}", self.telescope);

                if self.telescope.contains("alma") {
                    //disable optical
                    self.is_optical = false;
                }

                if self.telescope.contains("vla") || self.telescope.contains("ska") {
                    //disable optical
                    self.is_optical = false;
                }

                if self.telescope.contains("aste") {
                    //disable optical
                    self.is_optical = false;
                }

                if self.telescope.contains("nro") {
                    //disable optical
                    self.is_optical = false;
                }

                if self.telescope.contains("nro45m") {
                    //disable optical
                    self.is_optical = false;
                    self.flux = String::from("logistic");
                }

                // Tomo-e Gozen
                if self.telescope.contains("kiso") {
                    //enable optical
                    self.is_optical = true;
                    self.flux = String::from("ratio");
                }
            }

            if line.contains("ASTRO-F") {
                //switch on optical astronomy settings
                self.is_optical = true;
                self.flux = String::from("logistic");
            }

            if line.contains("HSCPIPE") {
                //switch on optical astronomy settings
                self.is_optical = true;
                self.flux = String::from("ratio");
            }

            if line.contains("FRAMEID = ") {
                let frameid = match scan_fmt_some!(line, "FRAMEID = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                };

                if frameid.contains("SUPM") || frameid.contains("MCSM") {
                    //switch on optical astronomy settings
                    self.is_optical = true;
                    self.flux = String::from("ratio");
                }
            }

            {
                let tmp = line.to_lowercase();
                if tmp.contains("suzaku") || tmp.contains("hitomi") || tmp.contains("x-ray") {
                    //switch on JAXA X-Ray settings
                    self.is_optical = false;
                    self.is_xray = true;
                    self.flux = String::from("legacy");
                    if self.ignrval == std::f32::MIN {
                        self.ignrval = -1.0;
                    }
                }
            }

            if line.contains("OBJECT  = ") {
                self.obj_name = match Regex::new(r"'(.*?)'").unwrap().find(line) {
                    Some(obj_name) => String::from(obj_name.as_str()).replace("'", ""),
                    None => String::from(""),
                }
            }

            if line.contains("DATE-OBS= ") {
                self.obs_date = match scan_fmt_some!(line, "DATE-OBS= {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("LINE    = ") {
                self.line = match scan_fmt_some!(line, "LINE    = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("FILTER  = ") {
                self.filter = match scan_fmt_some!(line, "FILTER  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("J_LINE  = ") {
                self.line = match scan_fmt_some!(line, "J_LINE  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("SPECSYS = ") {
                self.specsys = match scan_fmt_some!(line, "SPECSYS = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("TIMESYS = ") {
                self.timesys = match scan_fmt_some!(line, "TIMESYS = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("BITPIX  = ") {
                self.bitpix = match scan_fmt_some!(line, "BITPIX  = {d}", i32) {
                    Some(x) => x,
                    _ => 0,
                }
            }

            if line.contains("NAXIS   = ") {
                self.naxis = match scan_fmt_some!(line, "NAXIS   = {d}", i32) {
                    Some(x) => x,
                    _ => 0,
                }
            }

            if line.contains("NAXIS1  = ") {
                self.width = match scan_fmt_some!(line, "NAXIS1  = {d}", usize) {
                    Some(x) => x,
                    _ => 0,
                };

                self.naxes[0] = self.width;
            }

            if line.contains("NAXIS2  = ") {
                self.height = match scan_fmt_some!(line, "NAXIS2  = {d}", usize) {
                    Some(x) => x,
                    _ => 0,
                };

                self.naxes[1] = self.height;
            }

            if line.contains("NAXIS3  = ") {
                self.depth = match scan_fmt_some!(line, "NAXIS3  = {d}", usize) {
                    Some(x) => x,
                    _ => 1,
                };

                self.naxes[2] = self.depth;
            }

            if line.contains("NAXIS4  = ") {
                self.polarisation = match scan_fmt_some!(line, "NAXIS4  = {d}", usize) {
                    Some(x) => x,
                    _ => 1,
                };

                self.naxes[3] = self.polarisation;
            }

            if line.contains("BTYPE   = ") {
                self.beam_type = match scan_fmt_some!(line, "BTYPE   = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("BUNIT   = ") {
                self.beam_unit = match scan_fmt_some!(line, "BUNIT   = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("BMAJ    = ") {
                let s = match scan_fmt_some!(line, "BMAJ    = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.bmaj = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("BMIN    = ") {
                let s = match scan_fmt_some!(line, "BMIN    = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.bmin = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("BPA     = ") {
                let s = match scan_fmt_some!(line, "BPA     = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.bpa = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("RESTFRQ = ") {
                let s = match scan_fmt_some!(line, "RESTFRQ = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.restfrq = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("RESTFREQ= ") {
                let s = match scan_fmt_some!(line, "RESTFREQ= {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.restfrq = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("OBSRA   = ") {
                let s = match scan_fmt_some!(line, "OBSRA   = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.obsra = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("RA_OBJ  = ") {
                let s = match scan_fmt_some!(line, "RA_OBJ  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.obsra = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("OBSDEC  = ") {
                let s = match scan_fmt_some!(line, "OBSDEC  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.obsdec = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("DEC_OBJ = ") {
                let s = match scan_fmt_some!(line, "DEC_OBJ = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.obsdec = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("DATAMIN = ") {
                let s = match scan_fmt_some!(line, "DATAMIN = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.datamin = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => std::f32::MIN,
                };

                if self.datamin == self.datamax {
                    self.datamin = std::f32::MIN;
                    self.datamax = std::f32::MAX;
                };
            }

            if line.contains("DATAMAX = ") {
                let s = match scan_fmt_some!(line, "DATAMAX = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.datamax = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => std::f32::MAX,
                };

                if self.datamin == self.datamax {
                    self.datamin = std::f32::MIN;
                    self.datamax = std::f32::MAX;
                };
            }

            if line.contains("BSCALE  = ") {
                let s = match scan_fmt_some!(line, "BSCALE  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.bscale = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("BZERO   = ") {
                let s = match scan_fmt_some!(line, "BZERO   = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.bzero = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("IGNRVAL = ") {
                let s = match scan_fmt_some!(line, "IGNRVAL = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.ignrval = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => std::f32::MIN,
                }
            }

            if line.contains("CRVAL1  = ") {
                let s = match scan_fmt_some!(line, "CRVAL1  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.crval1 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CRVAL2  = ") {
                let s = match scan_fmt_some!(line, "CRVAL2  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.crval2 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CRVAL3  = ") {
                let s = match scan_fmt_some!(line, "CRVAL3  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.crval3 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CDELT1  = ") {
                let s = match scan_fmt_some!(line, "CDELT1  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.cdelt1 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CDELT2  = ") {
                let s = match scan_fmt_some!(line, "CDELT2  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.cdelt2 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CDELT3  = ") {
                let s = match scan_fmt_some!(line, "CDELT3  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.cdelt3 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CRPIX1  = ") {
                let s = match scan_fmt_some!(line, "CRPIX1  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.crpix1 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CRPIX2  = ") {
                let s = match scan_fmt_some!(line, "CRPIX2  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.crpix2 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CRPIX3  = ") {
                let s = match scan_fmt_some!(line, "CRPIX3  = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.crpix3 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CUNIT1  = ") {
                self.cunit1 = match scan_fmt_some!(line, "CUNIT1  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("CUNIT2  = ") {
                self.cunit2 = match scan_fmt_some!(line, "CUNIT2  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("CUNIT3  = ") {
                self.cunit3 = match scan_fmt_some!(line, "CUNIT3  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("CTYPE1  = ") {
                self.ctype1 = match scan_fmt_some!(line, "CTYPE1  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("CTYPE2  = ") {
                self.ctype2 = match scan_fmt_some!(line, "CTYPE2  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("CTYPE3  = ") {
                self.ctype3 = match scan_fmt_some!(line, "CTYPE3  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from(""),
                }
            }

            if line.contains("CD1_1   = ") {
                let s = match scan_fmt_some!(line, "CD1_1   = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.cd1_1 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CD1_2   = ") {
                let s = match scan_fmt_some!(line, "CD1_2   = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.cd1_2 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CD2_1   = ") {
                let s = match scan_fmt_some!(line, "CD2_1   = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.cd2_1 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            if line.contains("CD2_2   = ") {
                let s = match scan_fmt_some!(line, "CD2_2   = {}", String) {
                    Some(x) => x,
                    _ => String::from(""),
                };

                self.cd2_2 = match s.parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => 0.0,
                }
            }

            offset = offset + FITS_LINE_LENGTH;
        }

        return Ok(false);
    }

    fn process_cube_frame(&mut self, buf: &[u8], cdelt3: f32, frame: usize) {
        let mut rdr = Cursor::new(buf);
        let len = self.width * self.height;

        let mut sum: f32 = 0.0;
        let mut count: i32 = 0;

        //no mistake here, the initial ranges are supposed to be broken
        let mut frame_min = std::f32::MAX;
        let mut frame_max = std::f32::MIN;

        match self.bitpix {
            8 => {
                for i in 0..len {
                    self.data_u8[frame].push(buf[i as usize]);

                    let tmp = self.bzero + self.bscale * (buf[i as usize] as f32);
                    if tmp.is_finite()
                        && tmp >= self.datamin
                        && tmp <= self.datamax
                        && tmp > self.ignrval
                    {
                        self.pixels[i as usize] += tmp * cdelt3;
                        self.mask[i as usize] = 255;

                        frame_min = frame_min.min(tmp);
                        frame_max = frame_max.max(tmp);

                        sum += tmp;
                        count += 1;
                    }
                }
            }

            16 => {
                for i in 0..len {
                    match rdr.read_i16::<BigEndian>() {
                        Ok(int16) => {
                            self.data_i16[frame].push(int16);

                            let tmp = self.bzero + self.bscale * (int16 as f32);
                            if tmp.is_finite()
                                && tmp >= self.datamin
                                && tmp <= self.datamax
                                && tmp > self.ignrval
                            {
                                self.pixels[i as usize] += tmp * cdelt3;
                                self.mask[i as usize] = 255;

                                frame_min = frame_min.min(tmp);
                                frame_max = frame_max.max(tmp);

                                sum += tmp;
                                count += 1;
                            }
                        }
                        Err(err) => {
                            println!("BigEndian --> LittleEndian i16 conversion error: {}", err)
                        }
                    }
                }
            }

            32 => {
                for i in 0..len {
                    match rdr.read_i32::<BigEndian>() {
                        Ok(int32) => {
                            self.data_i32[frame].push(int32);

                            let tmp = self.bzero + self.bscale * (int32 as f32);
                            if tmp.is_finite()
                                && tmp >= self.datamin
                                && tmp <= self.datamax
                                && tmp > self.ignrval
                            {
                                self.pixels[i as usize] += tmp * cdelt3;
                                self.mask[i as usize] = 255;

                                frame_min = frame_min.min(tmp);
                                frame_max = frame_max.max(tmp);

                                sum += tmp;
                                count += 1;
                            }
                        }
                        Err(err) => {
                            println!("BigEndian --> LittleEndian i32 conversion error: {}", err)
                        }
                    }
                }
            }

            -32 => {
                for i in 0..len {
                    match rdr.read_f32::<BigEndian>() {
                        Ok(float32) => {
                            let float16 = f16::from_f32(float32);
                            //println!("f32 = {} <--> f16 = {}", float32, float16);
                            self.data_f16[frame].push(float16);

                            let tmp = self.bzero + self.bscale * float32;
                            if tmp.is_finite()
                                && tmp >= self.datamin
                                && tmp <= self.datamax
                                && tmp > self.ignrval
                            {
                                self.pixels[i as usize] += tmp * cdelt3;
                                self.mask[i as usize] = 255;

                                frame_min = frame_min.min(tmp);
                                frame_max = frame_max.max(tmp);

                                sum += tmp;
                                count += 1;
                            }
                        }
                        Err(err) => {
                            println!("BigEndian --> LittleEndian f32 conversion error: {}", err)
                        }
                    }
                }
            }

            -64 => {
                for i in 0..len {
                    match rdr.read_f64::<BigEndian>() {
                        Ok(float64) => {
                            self.data_f64[frame].push(float64);

                            let tmp = self.bzero + self.bscale * (float64 as f32);
                            if tmp.is_finite()
                                && tmp >= self.datamin
                                && tmp <= self.datamax
                                && tmp > self.ignrval
                            {
                                self.pixels[i as usize] += tmp * cdelt3;
                                self.mask[i as usize] = 255;

                                frame_min = frame_min.min(tmp);
                                frame_max = frame_max.max(tmp);

                                sum += tmp;
                                count += 1;
                            }
                        }
                        Err(err) => {
                            println!("BigEndian --> LittleEndian f64 conversion error: {}", err)
                        }
                    }
                }
            }

            _ => {
                println!("unsupported bitpix: {}", self.bitpix);
                return;
            }
        };

        self.frame_min[frame] = self.frame_min[frame].min(frame_min);
        self.frame_max[frame] = self.frame_max[frame].max(frame_max);

        self.dmin = self.dmin.min(frame_min);
        self.dmax = self.dmax.max(frame_max);

        //mean and integrated intensities @ frame
        if count > 0 {
            self.mean_spectrum[frame] = sum / (count as f32);
            self.integrated_spectrum[frame] = sum * cdelt3;
        }
    }

    pub fn make_image_spectrum(
        &self,
        start: usize,
        end: usize,
    ) -> Option<(Vec<f32>, Vec<u8>, Vec<f32>, Vec<f32>)> {
        let watch = Instant::now();

        let num_threads = num_cpus::get_physical();

        let pool = match rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
        {
            Ok(pool) => pool,
            Err(err) => {
                println!(
                    "[make_image_spectrum] CRITICAL ERROR cannot create a thread pool: {:?}",
                    err
                );
                return None;
            }
        };

        println!("custom thread pool: {:?}", pool);

        let mut dmin = std::f32::MAX; //no mistake here
        let mut dmax = std::f32::MIN; //no mistake here

        let mut pixels: Vec<f32> = vec![0.0; self.pixels.len()];
        let mask: Vec<u8> = vec![0; self.mask.len()];

        //set up thread-local vectors
        let mut thread_pixels: Vec<RwLock<Vec<f32>>> = Vec::with_capacity(num_threads);
        let mut thread_mask: Vec<RwLock<Vec<u8>>> = Vec::with_capacity(num_threads);

        let mut thread_min: Vec<atomic::Atomic<f32>> = Vec::with_capacity(num_threads);
        let mut thread_max: Vec<atomic::Atomic<f32>> = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            thread_pixels.push(RwLock::new(vec![0.0; self.pixels.len()]));
            thread_mask.push(RwLock::new(vec![0; self.mask.len()]));

            thread_min.push(atomic::Atomic::new(std::f32::MAX));
            thread_max.push(atomic::Atomic::new(std::f32::MIN));
        }

        let total = end - start + 1;

        let cdelt3 = {
            if self.has_velocity && self.depth > 1 {
                self.cdelt3 * self.frame_multiplier / 1000.0
            } else {
                1.0
            }
        } as f32;

        let mut thread_mean_spectrum: Vec<atomic::Atomic<f32>> = Vec::with_capacity(total);
        let mut thread_integrated_spectrum: Vec<atomic::Atomic<f32>> = Vec::with_capacity(total);

        for _ in 0..total {
            thread_mean_spectrum.push(atomic::Atomic::new(0.0));
            thread_integrated_spectrum.push(atomic::Atomic::new(0.0));
        }

        pool.install(|| {
            (0..total).into_par_iter().for_each(|index| {
                let frame = start + index;

                let tid = match pool.current_thread_index() {
                    Some(tid) => tid,
                    None => 0,
                };

                println!("tid: {}, index: {}, frame: {}", tid, index, frame);

                //parallel data processing
                let mut frame_min = std::f32::MAX;
                let mut frame_max = std::f32::MIN;

                let mut mean_spectrum = 0.0_f32;
                let mut integrated_spectrum = 0.0_f32;

                match self.bitpix {
                    8 => {
                        let mut references: [f32; 4] =
                            [frame_min, frame_max, mean_spectrum, integrated_spectrum];

                        let vec = &self.data_u8[frame];
                        let vec_ptr = vec.as_ptr() as *mut u8;
                        let vec_len = vec.len();

                        let mut pixels = thread_pixels[tid].write();
                        let mask = thread_mask[tid].write();
                        let mask_ptr = mask.as_ptr() as *mut u8;
                        let mask_len = mask.len();

                        unsafe {
                            let vec_raw = slice::from_raw_parts_mut(vec_ptr, vec_len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                            spmd::make_image_spectrumU8_minmax(
                                vec_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.ignrval,
                                self.datamin,
                                self.datamax,
                                cdelt3,
                                pixels.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                vec_len as u32,
                                references.as_mut_ptr(),
                            );
                        }

                        frame_min = references[0];
                        frame_max = references[1];
                        mean_spectrum = references[2];
                        integrated_spectrum = references[3];
                    }
                    16 => {
                        let mut references: [f32; 4] =
                            [frame_min, frame_max, mean_spectrum, integrated_spectrum];

                        let vec = &self.data_i16[frame];
                        let vec_ptr = vec.as_ptr() as *mut i16;
                        let vec_len = vec.len();

                        let mut pixels = thread_pixels[tid].write();
                        let mask = thread_mask[tid].write();
                        let mask_ptr = mask.as_ptr() as *mut u8;
                        let mask_len = mask.len();

                        unsafe {
                            let vec_raw = slice::from_raw_parts_mut(vec_ptr, vec_len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                            spmd::make_image_spectrumI16_minmax(
                                vec_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.ignrval,
                                self.datamin,
                                self.datamax,
                                cdelt3,
                                pixels.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                vec_len as u32,
                                references.as_mut_ptr(),
                            );
                        }

                        frame_min = references[0];
                        frame_max = references[1];
                        mean_spectrum = references[2];
                        integrated_spectrum = references[3];
                    }
                    32 => {
                        let mut references: [f32; 4] =
                            [frame_min, frame_max, mean_spectrum, integrated_spectrum];

                        let vec = &self.data_i32[frame];
                        let vec_ptr = vec.as_ptr() as *mut i32;
                        let vec_len = vec.len();

                        let mut pixels = thread_pixels[tid].write();
                        let mask = thread_mask[tid].write();
                        let mask_ptr = mask.as_ptr() as *mut u8;
                        let mask_len = mask.len();

                        unsafe {
                            let vec_raw = slice::from_raw_parts_mut(vec_ptr, vec_len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                            spmd::make_image_spectrumI32_minmax(
                                vec_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.ignrval,
                                self.datamin,
                                self.datamax,
                                cdelt3,
                                pixels.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                vec_len as u32,
                                references.as_mut_ptr(),
                            );
                        }

                        frame_min = references[0];
                        frame_max = references[1];
                        mean_spectrum = references[2];
                        integrated_spectrum = references[3];
                    }
                    -32 => {
                        let mut references: [f32; 4] =
                            [frame_min, frame_max, mean_spectrum, integrated_spectrum];

                        let vec = &self.data_f16[frame];
                        let vec_ptr = vec.as_ptr() as *mut i16;
                        let vec_len = vec.len();

                        let mut pixels = thread_pixels[tid].write();
                        let mask = thread_mask[tid].write();
                        let mask_ptr = mask.as_ptr() as *mut u8;
                        let mask_len = mask.len();

                        unsafe {
                            let vec_raw = slice::from_raw_parts_mut(vec_ptr, vec_len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                            spmd::make_image_spectrumF16_minmax(
                                vec_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.ignrval,
                                self.datamin,
                                self.datamax,
                                cdelt3,
                                pixels.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                vec_len as u32,
                                references.as_mut_ptr(),
                            );
                        }

                        frame_min = references[0];
                        frame_max = references[1];
                        mean_spectrum = references[2];
                        integrated_spectrum = references[3];
                    }
                    -64 => {
                        let mut references: [f32; 4] =
                            [frame_min, frame_max, mean_spectrum, integrated_spectrum];

                        let vec = &self.data_f64[frame];
                        let vec_ptr = vec.as_ptr() as *mut f64;
                        let vec_len = vec.len();

                        let mut pixels = thread_pixels[tid].write();
                        let mask = thread_mask[tid].write();
                        let mask_ptr = mask.as_ptr() as *mut u8;
                        let mask_len = mask.len();

                        unsafe {
                            let vec_raw = slice::from_raw_parts_mut(vec_ptr, vec_len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                            spmd::make_image_spectrumF64_minmax(
                                vec_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.ignrval,
                                self.datamin,
                                self.datamax,
                                cdelt3,
                                pixels.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                vec_len as u32,
                                references.as_mut_ptr(),
                            );
                        }

                        frame_min = references[0];
                        frame_max = references[1];
                        mean_spectrum = references[2];
                        integrated_spectrum = references[3];
                    }
                    _ => println!("unsupported bitpix: {}", self.bitpix),
                }

                thread_mean_spectrum[index].store(mean_spectrum, Ordering::SeqCst);
                thread_integrated_spectrum[index].store(integrated_spectrum, Ordering::SeqCst);

                let current_min = thread_min[tid].load(Ordering::SeqCst);
                thread_min[tid].store(frame_min.min(current_min), Ordering::SeqCst);

                let current_max = thread_max[tid].load(Ordering::SeqCst);
                thread_max[tid].store(frame_max.max(current_max), Ordering::SeqCst);
                //end of parallel data processing
            })
        });

        let mean_spectrum: Vec<f32> = thread_mean_spectrum
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        let integrated_spectrum: Vec<f32> = thread_integrated_spectrum
            .iter()
            .map(|x| x.load(Ordering::SeqCst))
            .collect();

        //then fuse self.pixels and self.mask with the local thread versions
        for tid in 0..num_threads {
            dmin = dmin.min(thread_min[tid].load(Ordering::SeqCst));
            dmax = dmax.max(thread_max[tid].load(Ordering::SeqCst));

            let mut pixels_tid = thread_pixels[tid].write();

            let mask_tid = thread_mask[tid].read();
            let mask_tid_ptr = mask_tid.as_ptr() as *mut u8;

            let mask_ptr = mask.as_ptr() as *mut u8;

            let total_size = pixels.len();

            unsafe {
                let mask_raw = slice::from_raw_parts_mut(mask_ptr, total_size);
                let mask_tid_raw = slice::from_raw_parts_mut(mask_tid_ptr, total_size);

                spmd::join_pixels_masks(
                    pixels.as_mut_ptr(),
                    pixels_tid.as_mut_ptr(),
                    mask_raw.as_mut_ptr(),
                    mask_tid_raw.as_mut_ptr(),
                    cdelt3,
                    total_size as u32,
                );
            }
        }

        println!("[make_image_spectrum] elapsed time: {:?}", watch.elapsed());

        //println!("mean spectrum: {:?}", mean_spectrum);
        //println!("integrated spectrum: {:?}", integrated_spectrum);

        Some((pixels, mask, mean_spectrum, integrated_spectrum))
    }

    pub fn make_data_histogram(&self) {
        println!("global dmin = {}, dmax = {}", self.dmin, self.dmax);

        self.data_hist.write().resize(NBINS2, 0);

        fn increment_histogram(
            x: f32,
            datamin: f32,
            datamax: f32,
            dmin: f32,
            dmax: f32,
            hist: &mut Vec<i64>,
        ) {
            if x.is_finite() && x >= datamin && x <= datamax {
                //add it to a local histogram
                let bin = (NBINS2 as f32) * (x - dmin) / (dmax - dmin);
                let index = num::clamp(bin as usize, 0, NBINS2 - 1);
                hist[index] = hist[index] + 1;
            }
        }

        fn update_deviation(
            x: f32,
            datamin: f32,
            datamax: f32,
            median: f32,
            mad: &mut f32,
            mad_p: &mut f32,
            mad_n: &mut f32,
            count: &mut i64,
            count_p: &mut i64,
            count_n: &mut i64,
        ) {
            if x.is_finite() && x >= datamin && x <= datamax {
                *mad = *mad + (x - median).abs();
                *count = *count + 1;

                if x > median {
                    *mad_p = *mad_p + (x - median);
                    *count_p = *count_p + 1;
                };

                if x < median {
                    *mad_n = *mad_n + (median - x);
                    *count_n = *count_n + 1;
                };
            }
        }

        //skip most of the data, take every nth value
        //this will produce an approximate all-data histogram
        let data_step = {
            let total_size = (self.width * self.height) as usize;

            //an adaptive step based on the amount of data
            if total_size >= 100 * NBINS2 {
                1000
            } else if total_size >= 10 * NBINS2 {
                100
            } else if total_size >= NBINS2 {
                10
            } else {
                //take all data by default
                1
            }
        };

        let watch = Instant::now();

        //parallel histogram of all data
        //actually serial so as not to affect other threads, smooth real-time spectrum calculation etc?
        //into_par_iter or into_iter
        let num_threads = num_cpus::get_physical();
        let pool = match rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
        {
            Ok(pool) => pool,
            Err(err) => {
                println!("{:?}, switching to a global rayon pool", err);
                return;
            }
        };

        pool.install(|| {
            (0..self.depth).into_par_iter().for_each(|frame| {
                //init a local histogram
                let mut hist = vec![0; NBINS2];

                //build a local histogram using frame data
                match self.bitpix {
                    8 => {
                        for x in self.data_u8[frame as usize].iter().step_by(data_step) {
                            let tmp = self.bzero + self.bscale * (*x as f32);
                            increment_histogram(
                                tmp,
                                self.datamin,
                                self.datamax,
                                self.dmin,
                                self.dmax,
                                &mut hist,
                            );
                        }
                    }
                    16 => {
                        for x in self.data_i16[frame as usize].iter().step_by(data_step) {
                            let tmp = self.bzero + self.bscale * (*x as f32);
                            increment_histogram(
                                tmp,
                                self.datamin,
                                self.datamax,
                                self.dmin,
                                self.dmax,
                                &mut hist,
                            );
                        }
                    }
                    32 => {
                        for x in self.data_i32[frame as usize].iter().step_by(data_step) {
                            let tmp = self.bzero + self.bscale * (*x as f32);
                            increment_histogram(
                                tmp,
                                self.datamin,
                                self.datamax,
                                self.dmin,
                                self.dmax,
                                &mut hist,
                            );
                        }
                    }
                    -32 => {
                        /*self.data_f16[frame as usize].iter()
                        .zip(self.mask.iter())
                            .for_each(|(x, m)| {*/
                        for x in self.data_f16[frame as usize].iter().step_by(data_step) {
                            //            if *m {
                            let tmp = self.bzero + self.bscale * (*x).to_f32(); //convert from half to f32
                            increment_histogram(
                                tmp,
                                self.datamin,
                                self.datamax,
                                self.dmin,
                                self.dmax,
                                &mut hist,
                            );
                        }
                        //    })
                    }
                    -64 => {
                        for x in self.data_f64[frame as usize].iter().step_by(data_step) {
                            let tmp = self.bzero + self.bscale * (*x as f32);
                            increment_histogram(
                                tmp,
                                self.datamin,
                                self.datamax,
                                self.dmin,
                                self.dmax,
                                &mut hist,
                            );
                        }
                    }
                    _ => println!("unsupported bitpix: {}", self.bitpix),
                };

                //get a write lock to the global histogram, update it
                let mut data_hist = self.data_hist.write();

                for i in 0..NBINS2 {
                    data_hist[i] = data_hist[i] + hist[i];
                }
            });
        });

        let data_hist = self.data_hist.read();

        let mut total: i64 = 0;

        for i in 0..NBINS2 {
            total += data_hist[i];
        }

        //find the percentiles and/or the median and madN, madP
        let mut cumulative: i64 = 0;
        let mut pos = 0;

        for i in 0..NBINS2 {
            if cumulative + data_hist[i] >= (total >> 1) {
                pos = i;
                break;
            };

            cumulative += data_hist[i];
        }

        let dx = (self.dmax - self.dmin) / ((NBINS2 << 1) as f32);
        let median = self.dmin + (((pos << 1) + 1) as f32) * dx;
        *self.data_median.write() = median;

        //mutex-protected global variables
        let data_count = RwLock::new(0_i64);
        let data_count_p = RwLock::new(0_i64);
        let data_count_n = RwLock::new(0_i64);

        //into_par_iter or into_iter
        pool.install(|| {
            (0..self.depth).into_par_iter().for_each(|frame| {
                //init local mad, madP, madN
                let mut mad = 0.0_f32;
                let mut mad_p = 0.0_f32;
                let mut mad_n = 0.0_f32;
                let mut count = 0_i64;
                let mut count_p = 0_i64;
                let mut count_n = 0_i64;

                //build a local histogram using frame data
                match self.bitpix {
                    8 => {
                        for x in self.data_u8[frame as usize].iter().step_by(data_step) {
                            let tmp = self.bzero + self.bscale * (*x as f32);
                            update_deviation(
                                tmp,
                                self.datamin,
                                self.datamax,
                                median,
                                &mut mad,
                                &mut mad_p,
                                &mut mad_n,
                                &mut count,
                                &mut count_p,
                                &mut count_n,
                            );
                        }
                    }
                    16 => {
                        for x in self.data_i16[frame as usize].iter().step_by(data_step) {
                            let tmp = self.bzero + self.bscale * (*x as f32);
                            update_deviation(
                                tmp,
                                self.datamin,
                                self.datamax,
                                median,
                                &mut mad,
                                &mut mad_p,
                                &mut mad_n,
                                &mut count,
                                &mut count_p,
                                &mut count_n,
                            );
                        }
                    }
                    32 => {
                        for x in self.data_i32[frame as usize].iter().step_by(data_step) {
                            let tmp = self.bzero + self.bscale * (*x as f32);
                            update_deviation(
                                tmp,
                                self.datamin,
                                self.datamax,
                                median,
                                &mut mad,
                                &mut mad_p,
                                &mut mad_n,
                                &mut count,
                                &mut count_p,
                                &mut count_n,
                            );
                        }
                    }
                    -32 => {
                        for x in self.data_f16[frame as usize].iter().step_by(data_step) {
                            //            if *m {
                            let tmp = self.bzero + self.bscale * (*x).to_f32(); //convert from half to f32
                            update_deviation(
                                tmp,
                                self.datamin,
                                self.datamax,
                                median,
                                &mut mad,
                                &mut mad_p,
                                &mut mad_n,
                                &mut count,
                                &mut count_p,
                                &mut count_n,
                            );
                        }
                    }
                    -64 => {
                        for x in self.data_f64[frame as usize].iter().step_by(data_step) {
                            let tmp = self.bzero + self.bscale * (*x as f32);
                            update_deviation(
                                tmp,
                                self.datamin,
                                self.datamax,
                                median,
                                &mut mad,
                                &mut mad_p,
                                &mut mad_n,
                                &mut count,
                                &mut count_p,
                                &mut count_n,
                            );
                        }
                    }
                    _ => println!("unsupported bitpix: {}", self.bitpix),
                };

                //update global mad,count
                let mut data_mad = self.data_mad.write();
                let mut data_mad_p = self.data_mad_p.write();
                let mut data_mad_n = self.data_mad_n.write();

                let mut data_count = data_count.write();
                let mut data_count_p = data_count_p.write();
                let mut data_count_n = data_count_n.write();

                *data_mad += mad;
                *data_mad_p += mad_p;
                *data_mad_n += mad_n;

                *data_count += count;
                *data_count_p += count_p;
                *data_count_n += count_n;
            });
        });

        let mut data_mad = self.data_mad.write();
        let mut data_mad_p = self.data_mad_p.write();
        let mut data_mad_n = self.data_mad_n.write();

        let data_count = data_count.read();
        let data_count_p = data_count_p.read();
        let data_count_n = data_count_n.read();

        if *data_count > 0 {
            *data_mad /= *data_count as f32;
        };

        if *data_count_p > 0 {
            *data_mad_p /= *data_count_p as f32;
        };

        if *data_count_n > 0 {
            *data_mad_n /= *data_count_n as f32;
        };

        println!(
            "median of an approximate all-data histogram: {} at pos {}, mad = {}, mad_p = {}, mad_n = {}",
            *self.data_median.read(),
            pos,
            *data_mad,
            *data_mad_p,
            *data_mad_n
        );

        println!(
            "all-data histogram adaptive step {}, elapsed time {:?}",
            data_step,
            watch.elapsed()
        );
    }

    pub fn get_image_histogram(
        &self,
        ord_pixels: &Vec<f32>,
        pixels: &Vec<f32>,
        mask: &Vec<u8>,
    ) -> Option<(Vec<i32>, f32, f32, f32, f32, f32, f32, f32)> {
        let mut len = ord_pixels.len();
        let mut hist: Vec<i32> = vec![0; NBINS];

        //ignore all the NaNs at the end of the vector
        while !ord_pixels[len - 1].is_finite() {
            len = len - 1;

            if len == 1 {
                break;
            }
        }

        let pmin = ord_pixels[0];
        let pmax = ord_pixels[len - 1];

        let median = {
            if len.is_odd() {
                ord_pixels[len >> 1]
            } else {
                (ord_pixels[(len >> 1) - 1] + ord_pixels[len >> 1]) / 2.0
            }
        };

        //a single-threaded version (seems to be more efficient than rayon in this case)
        let mut mad: f32 = 0.0;
        let mut count: i32 = 0;
        let mut mad_n: f32 = 0.0;
        let mut count_n: i32 = 0;
        let mut mad_p: f32 = 0.0;
        let mut count_p: i32 = 0;

        let watch = Instant::now();

        //reset the length
        len = self.pixels.len();

        for i in 0..len {
            if mask[i] > 0 {
                let x = pixels[i];

                mad += (x - median).abs();
                count += 1;

                if x > median {
                    mad_p += x - median;
                    count_p += 1;
                }

                if x < median {
                    mad_n += median - x;
                    count_n += 1;
                }
            }
        }

        mad = if count > 0 { mad / (count as f32) } else { 0.0 };

        mad_p = if count_p > 0 {
            mad_p / (count_p as f32)
        } else {
            mad
        };

        mad_n = if count_n > 0 {
            mad_n / (count_n as f32)
        } else {
            mad
        };

        //ALMAWebQL-style
        let u = 7.5_f32;
        let mut black = pmin.max(median - u * mad_n);
        let mut white = pmax.min(median + u * mad_p);
        let mut sensitivity = 1.0 / (white - black);
        let mut ratio_sensitivity = sensitivity;

        //SubaruWebQL-style
        if self.is_optical {
            let u = 0.5_f32;
            let v = 15.0_f32;
            black = pmin.max(median - u * mad);
            white = pmax.min(median + u * mad);
            sensitivity = 1.0 / (v * mad);
            ratio_sensitivity = match self.auto_brightness(pixels, mask, black, sensitivity) {
                Some(x) => x,
                None => sensitivity,
            };
        };

        println!(
            "pixels: range {} ~ {}, median = {}, mad = {}, mad_p = {}, mad_n = {}, black = {}, white = {}, sensitivity = {}, elapsed time {:?}",
            pmin,
            pmax,
            median,
            mad,
            mad_p,
            mad_n,
            black,
            white,
            sensitivity,
            watch.elapsed()
        );

        //the histogram part
        let dx = (pmax - pmin) / (NBINS as f32);

        if dx <= 0.0 {
            return None;
        }

        let mut bin = pmin + dx;
        let mut index = 0;

        let watch = Instant::now();

        for x in ord_pixels {
            while x >= &bin {
                bin = bin + dx;
                index = index + 1;
            }

            if x >= &pmin && x <= &pmax && index < NBINS {
                hist[index] = hist[index] + 1;
            }
        }

        println!("histogram creation elapsed time {:?}", watch.elapsed());

        Some((
            hist,
            pmin,
            pmax,
            black,
            white,
            median,
            sensitivity,
            ratio_sensitivity,
        ))
    }

    fn make_image_histogram(&mut self, ord_pixels: &Vec<f32>) {
        let mut len = ord_pixels.len();

        //println!("{:?}", ord_pixels);

        //ignore all the NaNs at the end of the vector
        while !ord_pixels[len - 1].is_finite() {
            len = len - 1;

            if len == 1 {
                break;
            }
        }

        let pmin = ord_pixels[0];
        let pmax = ord_pixels[len - 1];

        let median = {
            if len.is_odd() {
                ord_pixels[len >> 1]
            } else {
                (ord_pixels[(len >> 1) - 1] + ord_pixels[len >> 1]) / 2.0
            }
        };

        /*let watch = Instant::now();

        let (mut mad, count) : (f32, i32) = rayon::join(
            || {
            ord_pixels.par_iter()
                .map(|&x| (x - median).abs())
                .sum()
            },
            || {
            self.mask.par_iter()
                .map(|&m| m as i32)
                .sum()
            }
        );

        let (mut mad_p, count_p) : (f32, i32) = rayon::join(
             || {
            ord_pixels.iter()
                .zip(self.mask.iter())
                .map(|(x, m)| {
                    if *m && x > &median {
                        x - median
                    }
                    else {
                        0.0
                    }
                })
                .sum()
            },
            || {
            ord_pixels.iter()
                .zip(self.mask.iter())
                .map(|(x, m)| {
                    if *m && x > &median {
                        1
                    }
                    else {
                        0
                    }
                })
                .sum()
            }
        );

         let (mut mad_n, count_n) : (f32, i32) = rayon::join(
             || {
            ord_pixels.iter()
                .zip(self.mask.iter())
                .map(|(x, m)| {
                    if *m && x < &median {
                        median - x
                    }
                    else {
                        0.0
                    }
                })
                .sum()
            },
            || {
            ord_pixels.iter()
                .zip(self.mask.iter())
                .map(|(x, m)| {
                    if *m && x < &median {
                        1
                    }
                    else {
                        0
                    }
                })
                .sum()
            }
        );*/

        //a single-threaded version (seems to be more efficient than rayon in this case)
        let mut mad: f32 = 0.0;
        let mut count: i32 = 0;
        let mut mad_n: f32 = 0.0;
        let mut count_n: i32 = 0;
        let mut mad_p: f32 = 0.0;
        let mut count_p: i32 = 0;

        let watch = Instant::now();

        //reset the length
        len = self.pixels.len();

        for i in 0..len {
            if self.mask[i] > 0 {
                let x = self.pixels[i];

                mad += (x - median).abs();
                count += 1;

                if x > median {
                    mad_p += x - median;
                    count_p += 1;
                }

                if x < median {
                    mad_n += median - x;
                    count_n += 1;
                }
            }
        }

        mad = if count > 0 { mad / (count as f32) } else { 0.0 };

        mad_p = if count_p > 0 {
            mad_p / (count_p as f32)
        } else {
            mad
        };

        mad_n = if count_n > 0 {
            mad_n / (count_n as f32)
        } else {
            mad
        };

        //ALMAWebQL-style
        let u = 7.5_f32;
        let mut black = pmin.max(median - u * mad_n);
        let mut white = pmax.min(median + u * mad_p);
        let mut sensitivity = 1.0 / (white - black);
        let mut ratio_sensitivity = sensitivity;

        //SubaruWebQL-style
        if self.is_optical {
            let u = 0.5_f32;
            let v = 15.0_f32;
            black = pmin.max(median - u * mad);
            white = pmax.min(median + u * mad);
            sensitivity = 1.0 / (v * mad);
            ratio_sensitivity =
                match self.auto_brightness(&self.pixels, &self.mask, black, sensitivity) {
                    Some(x) => x,
                    None => sensitivity,
                };
        };

        println!(
            "pixels: range {} ~ {}, median = {}, mad = {}, mad_p = {}, mad_n = {}, black = {}, white = {}, sensitivity = {}, elapsed time {:?}",
            pmin,
            pmax,
            median,
            mad,
            mad_p,
            mad_n,
            black,
            white,
            sensitivity,
            watch.elapsed()
        );

        self.pmin = pmin;
        self.pmax = pmax;
        self.median = median;
        self.black = black;
        self.white = white;
        self.sensitivity = sensitivity;
        self.ratio_sensitivity = ratio_sensitivity;
        self.mad = mad;
        self.mad_n = mad_n;
        self.mad_p = mad_p;

        //the histogram part
        let dx = (pmax - pmin) / (NBINS as f32);
        self.hist.resize(NBINS, 0);

        if dx <= 0.0 {
            return;
        }

        let mut bin = pmin + dx;
        let mut index = 0;

        let watch = Instant::now();

        for x in ord_pixels {
            while x >= &bin {
                bin = bin + dx;
                index = index + 1;
            }

            if x >= &pmin && x <= &pmax && index < NBINS {
                self.hist[index] = self.hist[index] + 1;
            }
        }

        println!("histogram creation elapsed time {:?}", watch.elapsed());
    }

    fn histogram_classifier(&mut self) {
        if self.hist.len() <= 0 {
            return;
        }

        let mut cdf: Vec<i64> = vec![0; NBINS];
        let mut slot: Vec<f64> = vec![0.0; NBINS];

        let mut total: i64 = self.hist[0] as i64;
        cdf[0] = total;

        for i in 1..NBINS {
            cdf[i] = cdf[i - 1] + (self.hist[i] as i64);
            total += self.hist[i] as i64;
        }

        for i in 0..NBINS {
            slot[i] = (cdf[i] as f64) / (total as f64);
        }

        let watch = Instant::now();

        self.flux = match logistic_regression_classifier(&slot) {
            0 => String::from("legacy"),
            1 => String::from("linear"),
            2 => String::from("logistic"),
            3 => String::from("ratio"),
            4 => String::from("square"),
            _ => String::from("legacy"),
        };

        println!("histogram classifier elapsed time {:?}", watch.elapsed());
    }

    fn data_to_luminance_u8(
        &self,
        frame: usize,
        flux: &String,
        _pool: &Option<rayon::ThreadPool>,
    ) -> Option<Vec<u8>> {
        if self.data_u8[frame].len() == 0 {
            return None;
        };

        //calculate white, black, sensitivity from the data_histogram
        let u = 7.5_f32;
        //let v = 15.0_f32 ;

        let median = *self.data_median.read();
        let mut black = self
            .dmin
            .max((*self.data_median.read()) - u * (*self.data_mad_n.read()));
        let mut white = self
            .dmax
            .min((*self.data_median.read()) + u * (*self.data_mad_p.read()));
        let mut sensitivity = 1.0 / (white - black);
        let mut ratio_sensitivity = sensitivity;

        //SubaruWebQL-style
        if self.is_optical {
            let u = 0.5_f32;
            let v = 15.0_f32;
            black = self
                .dmin
                .max((*self.data_median.read()) - u * (*self.data_mad.read()));
            white = self
                .dmax
                .min((*self.data_median.read()) + u * (*self.data_mad.read()));
            sensitivity = 1.0 / (v * (*self.data_mad.read()));

            // re-use the auto-brightness factor
            let factor = self.ratio_sensitivity / self.sensitivity;
            ratio_sensitivity = sensitivity * factor;
        };

        let res = match flux.as_ref() {
            "linear" => {
                let slope = 1.0 / (white - black);

                self.data_u8[frame]
                    .par_iter()
                    .zip(self.mask.par_iter())
                    .map(|(x, m)| {
                        if *m > 0 {
                            let x = self.bzero + self.bscale * (*x as f32);
                            let pixel = num::clamp((x - black) * slope, 0.0, 1.0);
                            (255.0 * pixel) as u8
                        } else {
                            0
                        }
                    })
                    .collect()
            }
            "logistic" => self.data_u8[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = num::clamp(
                            1.0 / (1.0 + (-6.0 * (x - median) * sensitivity).exp()),
                            0.0,
                            1.0,
                        );
                        (255.0 * pixel) as u8
                    } else {
                        0
                    }
                })
                .collect(),
            "ratio" => self.data_u8[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = 5.0 * (x - black) * ratio_sensitivity;

                        if pixel > 0.0 {
                            (255.0 * pixel / (1.0 + pixel)) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
            "square" => self.data_u8[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = (x - black) * sensitivity;

                        if pixel > 0.0 {
                            (255.0 * num::clamp(pixel * pixel, 0.0, 1.0)) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
            //by default assume "legacy"
            _ => self.data_u8[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = 0.5 + (x - self.dmin) / (self.dmax - self.dmin);

                        if pixel > 0.0 {
                            (255.0
                                * num::clamp(
                                    (pixel.ln() - self.lmin) / (self.lmax - self.lmin),
                                    0.0,
                                    1.0,
                                )) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
        };

        Some(res)
    }

    fn data_to_luminance_i16(
        &self,
        frame: usize,
        flux: &String,
        _pool: &Option<rayon::ThreadPool>,
    ) -> Option<Vec<u8>> {
        if self.data_i16[frame].len() == 0 {
            return None;
        };

        //calculate white, black, sensitivity from the data_histogram
        let u = 7.5_f32;
        //let v = 15.0_f32 ;

        let median = *self.data_median.read();
        let mut black = self
            .dmin
            .max((*self.data_median.read()) - u * (*self.data_mad_n.read()));
        let mut white = self
            .dmax
            .min((*self.data_median.read()) + u * (*self.data_mad_p.read()));
        let mut sensitivity = 1.0 / (white - black);
        let mut ratio_sensitivity = sensitivity;

        //SubaruWebQL-style
        if self.is_optical {
            let u = 0.5_f32;
            let v = 15.0_f32;
            black = self
                .dmin
                .max((*self.data_median.read()) - u * (*self.data_mad.read()));
            white = self
                .dmax
                .min((*self.data_median.read()) + u * (*self.data_mad.read()));
            sensitivity = 1.0 / (v * (*self.data_mad.read()));

            // re-use the auto-brightness factor
            let factor = self.ratio_sensitivity / self.sensitivity;
            ratio_sensitivity = sensitivity * factor;
        };

        let res = match flux.as_ref() {
            "linear" => {
                let slope = 1.0 / (white - black);

                self.data_i16[frame]
                    .par_iter()
                    .zip(self.mask.par_iter())
                    .map(|(x, m)| {
                        if *m > 0 {
                            let x = self.bzero + self.bscale * (*x as f32);
                            let pixel = num::clamp((x - black) * slope, 0.0, 1.0);
                            (255.0 * pixel) as u8
                        } else {
                            0
                        }
                    })
                    .collect()
            }
            "logistic" => self.data_i16[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = num::clamp(
                            1.0 / (1.0 + (-6.0 * (x - median) * sensitivity).exp()),
                            0.0,
                            1.0,
                        );
                        (255.0 * pixel) as u8
                    } else {
                        0
                    }
                })
                .collect(),
            "ratio" => self.data_i16[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = 5.0 * (x - black) * ratio_sensitivity;

                        if pixel > 0.0 {
                            (255.0 * pixel / (1.0 + pixel)) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
            "square" => self.data_i16[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = (x - black) * sensitivity;

                        if pixel > 0.0 {
                            (255.0 * num::clamp(pixel * pixel, 0.0, 1.0)) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
            //by default assume "legacy"
            _ => self.data_i16[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = 0.5 + (x - self.dmin) / (self.dmax - self.dmin);

                        if pixel > 0.0 {
                            (255.0
                                * num::clamp(
                                    (pixel.ln() - self.lmin) / (self.lmax - self.lmin),
                                    0.0,
                                    1.0,
                                )) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
        };

        Some(res)
    }

    fn data_to_luminance_i32(
        &self,
        frame: usize,
        flux: &String,
        _pool: &Option<rayon::ThreadPool>,
    ) -> Option<Vec<u8>> {
        if self.data_i32[frame].len() == 0 {
            return None;
        };

        //calculate white, black, sensitivity from the data_histogram
        let u = 7.5_f32;
        //let v = 15.0_f32 ;

        let median = *self.data_median.read();
        let mut black = self
            .dmin
            .max((*self.data_median.read()) - u * (*self.data_mad_n.read()));
        let mut white = self
            .dmax
            .min((*self.data_median.read()) + u * (*self.data_mad_p.read()));
        let mut sensitivity = 1.0 / (white - black);
        let mut ratio_sensitivity = sensitivity;

        //SubaruWebQL-style
        if self.is_optical {
            let u = 0.5_f32;
            let v = 15.0_f32;
            black = self
                .dmin
                .max((*self.data_median.read()) - u * (*self.data_mad.read()));
            white = self
                .dmax
                .min((*self.data_median.read()) + u * (*self.data_mad.read()));
            sensitivity = 1.0 / (v * (*self.data_mad.read()));

            // re-use the auto-brightness factor
            let factor = self.ratio_sensitivity / self.sensitivity;
            ratio_sensitivity = sensitivity * factor;
        };

        let res = match flux.as_ref() {
            "linear" => {
                let slope = 1.0 / (white - black);

                self.data_i32[frame]
                    .par_iter()
                    .zip(self.mask.par_iter())
                    .map(|(x, m)| {
                        if *m > 0 {
                            let x = self.bzero + self.bscale * (*x as f32);
                            let pixel = num::clamp((x - black) * slope, 0.0, 1.0);
                            (255.0 * pixel) as u8
                        } else {
                            0
                        }
                    })
                    .collect()
            }
            "logistic" => self.data_i32[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = num::clamp(
                            1.0 / (1.0 + (-6.0 * (x - median) * sensitivity).exp()),
                            0.0,
                            1.0,
                        );
                        (255.0 * pixel) as u8
                    } else {
                        0
                    }
                })
                .collect(),
            "ratio" => self.data_i32[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = 5.0 * (x - black) * ratio_sensitivity;

                        if pixel > 0.0 {
                            (255.0 * pixel / (1.0 + pixel)) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
            "square" => self.data_i32[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = (x - black) * sensitivity;

                        if pixel > 0.0 {
                            (255.0 * num::clamp(pixel * pixel, 0.0, 1.0)) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
            //by default assume "legacy"
            _ => self.data_i32[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = 0.5 + (x - self.dmin) / (self.dmax - self.dmin);

                        if pixel > 0.0 {
                            (255.0
                                * num::clamp(
                                    (pixel.ln() - self.lmin) / (self.lmax - self.lmin),
                                    0.0,
                                    1.0,
                                )) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
        };

        Some(res)
    }

    fn data_to_luminance_f16(
        &self,
        frame: usize,
        flux: &String,
        pool: &Option<rayon::ThreadPool>,
    ) -> Option<Vec<u8>> {
        if self.data_f16[frame].len() == 0 {
            return None;
        };

        //calculate white, black, sensitivity from the data_histogram
        let u = 7.5_f32;
        //let v = 15.0_f32 ;

        let median = *self.data_median.read();
        let mut black = self
            .dmin
            .max((*self.data_median.read()) - u * (*self.data_mad_n.read()));
        let mut white = self
            .dmax
            .min((*self.data_median.read()) + u * (*self.data_mad_p.read()));
        let mut sensitivity = 1.0 / (white - black);
        let mut ratio_sensitivity = sensitivity;

        //SubaruWebQL-style
        if self.is_optical {
            let u = 0.5_f32;
            let v = 15.0_f32;
            black = self
                .dmin
                .max((*self.data_median.read()) - u * (*self.data_mad.read()));
            white = self
                .dmax
                .min((*self.data_median.read()) + u * (*self.data_mad.read()));
            sensitivity = 1.0 / (v * (*self.data_mad.read()));

            // re-use the auto-brightness factor
            let factor = self.ratio_sensitivity / self.sensitivity;
            ratio_sensitivity = sensitivity * factor;
        };

        //interfacing with the Intel SPMD Program Compiler
        let vec = &self.data_f16[frame];
        let len = vec.len();
        let tmp = (len / (1024 * 1024)).max(1);
        let num_threads = tmp.min(num_cpus::get_physical());
        let work_size = len / num_threads;

        println!(
            "data_to_luminance: tmp = {}, num_threads = {}, work_size = {}",
            tmp, num_threads, work_size
        );

        let y: Vec<u8> = vec![0; len];

        match pool {
            Some(pool) => pool.install(|| {
                (0..num_threads).into_par_iter().for_each(|index| {
                    let offset = index * work_size;

                    let work_size = if index == num_threads - 1 {
                        len - offset
                    } else {
                        work_size
                    };

                    let vec = &vec[offset..offset + work_size];
                    let ptr = vec.as_ptr() as *mut i16;
                    let len = vec.len();

                    println!(
                        "index: {}, offset: {}, work_size: {}, len = {}",
                        index, offset, work_size, len
                    );

                    let mask = &self.mask[offset..offset + work_size];
                    let mask_ptr = mask.as_ptr() as *mut u8;
                    let mask_len = mask.len();

                    let y = &y[offset..offset + work_size]; //partial outputs go in here
                    let y_ptr = y.as_ptr() as *mut u8;

                    match flux.as_ref() {
                        "linear" => {
                            let slope = 1.0 / (white - black);

                            unsafe {
                                let raw = slice::from_raw_parts_mut(ptr, len);
                                let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                                let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                                spmd::data_to_luminance_f16_linear(
                                    raw.as_mut_ptr(),
                                    mask_raw.as_mut_ptr(),
                                    self.bzero,
                                    self.bscale,
                                    black,
                                    slope,
                                    y_raw.as_mut_ptr(),
                                    len as u32,
                                );
                            }
                        }
                        "logistic" => unsafe {
                            let raw = slice::from_raw_parts_mut(ptr, len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                            let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                            spmd::data_to_luminance_f16_logistic(
                                raw.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                median,
                                sensitivity,
                                y_raw.as_mut_ptr(),
                                len as u32,
                            );
                        },
                        "ratio" => unsafe {
                            let raw = slice::from_raw_parts_mut(ptr, len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                            let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                            spmd::data_to_luminance_f16_ratio(
                                raw.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                black,
                                ratio_sensitivity,
                                y_raw.as_mut_ptr(),
                                len as u32,
                            );
                        },
                        "square" => unsafe {
                            let raw = slice::from_raw_parts_mut(ptr, len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                            let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                            spmd::data_to_luminance_f16_square(
                                raw.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                black,
                                sensitivity,
                                y_raw.as_mut_ptr(),
                                len as u32,
                            );
                        },
                        //by default assume "legacy"
                        _ => unsafe {
                            let raw = slice::from_raw_parts_mut(ptr, len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                            let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                            spmd::data_to_luminance_f16_legacy(
                                raw.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.dmin,
                                self.dmax,
                                self.lmin,
                                self.lmax,
                                y_raw.as_mut_ptr(),
                                len as u32,
                            );
                        },
                    }
                });
            }),
            None => {
                (0..num_threads).into_par_iter().for_each(|index| {
                    let offset = index * work_size;

                    let work_size = if index == num_threads - 1 {
                        len - offset
                    } else {
                        work_size
                    };

                    let vec = &vec[offset..offset + work_size];
                    let ptr = vec.as_ptr() as *mut i16;
                    let len = vec.len();

                    println!(
                        "index: {}, offset: {}, work_size: {}, len = {}",
                        index, offset, work_size, len
                    );

                    let mask = &self.mask[offset..offset + work_size];
                    let mask_ptr = mask.as_ptr() as *mut u8;
                    let mask_len = mask.len();

                    let y = &y[offset..offset + work_size]; //partial outputs go in here
                    let y_ptr = y.as_ptr() as *mut u8;

                    match flux.as_ref() {
                        "linear" => {
                            let slope = 1.0 / (white - black);

                            unsafe {
                                let raw = slice::from_raw_parts_mut(ptr, len);
                                let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                                let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                                spmd::data_to_luminance_f16_linear(
                                    raw.as_mut_ptr(),
                                    mask_raw.as_mut_ptr(),
                                    self.bzero,
                                    self.bscale,
                                    black,
                                    slope,
                                    y_raw.as_mut_ptr(),
                                    len as u32,
                                );
                            }
                        }
                        "logistic" => unsafe {
                            let raw = slice::from_raw_parts_mut(ptr, len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                            let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                            spmd::data_to_luminance_f16_logistic(
                                raw.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                median,
                                sensitivity,
                                y_raw.as_mut_ptr(),
                                len as u32,
                            );
                        },
                        "ratio" => unsafe {
                            let raw = slice::from_raw_parts_mut(ptr, len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                            let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                            spmd::data_to_luminance_f16_ratio(
                                raw.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                black,
                                sensitivity,
                                y_raw.as_mut_ptr(),
                                len as u32,
                            );
                        },
                        "square" => unsafe {
                            let raw = slice::from_raw_parts_mut(ptr, len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                            let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                            spmd::data_to_luminance_f16_square(
                                raw.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                black,
                                sensitivity,
                                y_raw.as_mut_ptr(),
                                len as u32,
                            );
                        },
                        //by default assume "legacy"
                        _ => unsafe {
                            let raw = slice::from_raw_parts_mut(ptr, len);
                            let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);
                            let y_raw = slice::from_raw_parts_mut(y_ptr, len);

                            spmd::data_to_luminance_f16_legacy(
                                raw.as_mut_ptr(),
                                mask_raw.as_mut_ptr(),
                                self.bzero,
                                self.bscale,
                                self.dmin,
                                self.dmax,
                                self.lmin,
                                self.lmax,
                                y_raw.as_mut_ptr(),
                                len as u32,
                            );
                        },
                    }
                });
            }
        };

        Some(y)
    }

    fn data_to_luminance_f64(
        &self,
        frame: usize,
        flux: &String,
        _pool: &Option<rayon::ThreadPool>,
    ) -> Option<Vec<u8>> {
        if self.data_f64[frame].len() == 0 {
            return None;
        };

        //calculate white, black, sensitivity from the data_histogram
        let u = 7.5_f32;
        //let v = 15.0_f32 ;

        let median = *self.data_median.read();
        let mut black = self
            .dmin
            .max((*self.data_median.read()) - u * (*self.data_mad_n.read()));
        let mut white = self
            .dmax
            .min((*self.data_median.read()) + u * (*self.data_mad_p.read()));
        let mut sensitivity = 1.0 / (white - black);
        let mut ratio_sensitivity = sensitivity;

        //SubaruWebQL-style
        if self.is_optical {
            let u = 0.5_f32;
            let v = 15.0_f32;
            black = self
                .dmin
                .max((*self.data_median.read()) - u * (*self.data_mad.read()));
            white = self
                .dmax
                .min((*self.data_median.read()) + u * (*self.data_mad.read()));
            sensitivity = 1.0 / (v * (*self.data_mad.read()));

            // re-use the auto-brightness factor
            let factor = self.ratio_sensitivity / self.sensitivity;
            ratio_sensitivity = sensitivity * factor;
        };

        let res = match flux.as_ref() {
            "linear" => {
                let slope = 1.0 / (white - black);

                self.data_f64[frame]
                    .par_iter()
                    .zip(self.mask.par_iter())
                    .map(|(x, m)| {
                        if *m > 0 {
                            let x = self.bzero + self.bscale * (*x as f32);
                            let pixel = num::clamp((x - black) * slope, 0.0, 1.0);
                            (255.0 * pixel) as u8
                        } else {
                            0
                        }
                    })
                    .collect()
            }
            "logistic" => self.data_f64[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = num::clamp(
                            1.0 / (1.0 + (-6.0 * (x - median) * sensitivity).exp()),
                            0.0,
                            1.0,
                        );
                        (255.0 * pixel) as u8
                    } else {
                        0
                    }
                })
                .collect(),
            "ratio" => self.data_f64[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = 5.0 * (x - black) * ratio_sensitivity;

                        if pixel > 0.0 {
                            (255.0 * pixel / (1.0 + pixel)) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
            "square" => self.data_f64[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = (x - black) * sensitivity;

                        if pixel > 0.0 {
                            (255.0 * num::clamp(pixel * pixel, 0.0, 1.0)) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
            //by default assume "legacy"
            _ => self.data_f64[frame]
                .par_iter()
                .zip(self.mask.par_iter())
                .map(|(x, m)| {
                    if *m > 0 {
                        let x = self.bzero + self.bscale * (*x as f32);
                        let pixel = 0.5 + (x - self.dmin) / (self.dmax - self.dmin);

                        if pixel > 0.0 {
                            (255.0
                                * num::clamp(
                                    (pixel.ln() - self.lmin) / (self.lmax - self.lmin),
                                    0.0,
                                    1.0,
                                )) as u8
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                })
                .collect(),
        };

        Some(res)
    }

    fn data_to_luminance(
        &self,
        frame: usize,
        flux: &String,
        pool: &Option<rayon::ThreadPool>,
    ) -> Option<Vec<u8>> {
        match self.bitpix {
            8 => self.data_to_luminance_u8(frame, flux, pool),
            16 => self.data_to_luminance_i16(frame, flux, pool),
            32 => self.data_to_luminance_i32(frame, flux, pool),
            -32 => self.data_to_luminance_f16(frame, flux, pool),
            -64 => self.data_to_luminance_f64(frame, flux, pool),
            _ => {
                println!("unsupported bitpix: {}", self.bitpix);
                None
            }
        }
    }

    pub fn pixels_to_luminance(
        &self,
        pixels: &Vec<f32>,
        mask: &Vec<u8>,
        pmin: f32,
        pmax: f32,
        lmin: f32,
        lmax: f32,
        black: f32,
        white: f32,
        median: f32,
        sensitivity: f32,
        ratio_sensitivity: f32,
        flux: &String,
        pool: &Option<rayon::ThreadPool>,
    ) -> Vec<u8> {
        match flux.as_ref() {
            "linear" => {
                let slope = 1.0 / (white - black);

                match pool {
                    Some(pool) => pool.install(|| {
                        pixels
                            .par_iter()
                            .zip(mask.par_iter())
                            .map(|(x, m)| {
                                if *m > 0 {
                                    let pixel = num::clamp((x - black) * slope, 0.0, 1.0);
                                    (255.0 * pixel) as u8
                                } else {
                                    0
                                }
                            })
                            .collect()
                    }),
                    None => pixels
                        .par_iter()
                        .zip(mask.par_iter())
                        .map(|(x, m)| {
                            if *m > 0 {
                                let pixel = num::clamp((x - black) * slope, 0.0, 1.0);
                                (255.0 * pixel) as u8
                            } else {
                                0
                            }
                        })
                        .collect(),
                }
            }
            "logistic" => match pool {
                Some(pool) => pool.install(|| {
                    pixels
                        .par_iter()
                        .zip(mask.par_iter())
                        .map(|(x, m)| {
                            if *m > 0 {
                                let pixel = num::clamp(
                                    1.0 / (1.0 + (-6.0 * (x - median) * sensitivity).exp()),
                                    0.0,
                                    1.0,
                                );
                                (255.0 * pixel) as u8
                            } else {
                                0
                            }
                        })
                        .collect()
                }),
                None => pixels
                    .par_iter()
                    .zip(mask.par_iter())
                    .map(|(x, m)| {
                        if *m > 0 {
                            let pixel = num::clamp(
                                1.0 / (1.0 + (-6.0 * (x - median) * sensitivity).exp()),
                                0.0,
                                1.0,
                            );
                            (255.0 * pixel) as u8
                        } else {
                            0
                        }
                    })
                    .collect(),
            },
            "ratio" => match pool {
                Some(pool) => pool.install(|| {
                    pixels
                        .par_iter()
                        .zip(mask.par_iter())
                        .map(|(x, m)| {
                            if *m > 0 {
                                let pixel = 5.0 * (x - black) * ratio_sensitivity;

                                if pixel > 0.0 {
                                    (255.0 * pixel / (1.0 + pixel)) as u8
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        })
                        .collect()
                }),
                None => pixels
                    .par_iter()
                    .zip(mask.par_iter())
                    .map(|(x, m)| {
                        if *m > 0 {
                            let pixel = 5.0 * (x - black) * ratio_sensitivity;

                            if pixel > 0.0 {
                                (255.0 * pixel / (1.0 + pixel)) as u8
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    })
                    .collect(),
            },
            "square" => match pool {
                Some(pool) => pool.install(|| {
                    pixels
                        .par_iter()
                        .zip(mask.par_iter())
                        .map(|(x, m)| {
                            if *m > 0 {
                                let pixel = (x - black) * sensitivity;

                                if pixel > 0.0 {
                                    (255.0 * num::clamp(pixel * pixel, 0.0, 1.0)) as u8
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        })
                        .collect()
                }),
                None => pixels
                    .par_iter()
                    .zip(mask.par_iter())
                    .map(|(x, m)| {
                        if *m > 0 {
                            let pixel = (x - black) * sensitivity;

                            if pixel > 0.0 {
                                (255.0 * num::clamp(pixel * pixel, 0.0, 1.0)) as u8
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    })
                    .collect(),
            },
            //by default assume "legacy"
            _ => match pool {
                Some(pool) => pool.install(|| {
                    pixels
                        .par_iter()
                        .zip(mask.par_iter())
                        .map(|(x, m)| {
                            if *m > 0 {
                                let pixel = 0.5 + (x - pmin) / (pmax - pmin);

                                if pixel > 0.0 {
                                    (255.0
                                        * num::clamp((pixel.ln() - lmin) / (lmax - lmin), 0.0, 1.0))
                                        as u8
                                } else {
                                    0
                                }
                            } else {
                                0
                            }
                        })
                        .collect()
                }),
                None => pixels
                    .par_iter()
                    .zip(mask.par_iter())
                    .map(|(x, m)| {
                        if *m > 0 {
                            let pixel = 0.5 + (x - pmin) / (pmax - pmin);

                            if pixel > 0.0 {
                                (255.0 * num::clamp((pixel.ln() - lmin) / (lmax - lmin), 0.0, 1.0))
                                    as u8
                            } else {
                                0
                            }
                        } else {
                            0
                        }
                    })
                    .collect(),
            },
        }
    }

    pub fn get_video_frame(
        &self,
        frame: usize,
        width: u32,
        height: u32,
        flux: &String,
        pool: &Option<rayon::ThreadPool>,
    ) -> Option<Vec<u8>> {
        println!("frame index = {}", frame);

        let watch = Instant::now();

        let y: Vec<u8> = match self.data_to_luminance(frame, flux, pool) {
            Some(y) => y,
            None => vec![0; (width * height) as usize],
        };

        //invert and downscale
        let mut dst = vec![0; (width * height) as usize];
        self.resize_and_invert(&y, &mut dst, width, height, libyuv_FilterMode_kFilterBox);

        println!("Y video plane preparation time: {:?}", watch.elapsed());

        Some(dst)
    }

    #[cfg(feature = "vp9")]
    pub fn get_vpx_frame(
        &self,
        frame: f64,
        ref_freq: f64,
        width: u32,
        height: u32,
        flux: &String,
    ) -> Option<vpx_image> {
        //get a frame index (frame_start = frame_end = frame)
        let frame = match self.get_spectrum_range(frame, frame, ref_freq) {
            Some((frame, _)) => frame,
            None => {
                println!("error: an invalid spectrum range");
                return None;
            }
        };

        println!("frame index = {}", frame);

        let watch = Instant::now();

        //let w = self.width as u32 ;
        //let h = self.height as u32 ;
        let align = 1;

        let mut raw: vpx_image = vpx_image::default();
        let ret = unsafe {
            vpx_img_alloc(
                &mut raw,
                vpx_img_fmt::VPX_IMG_FMT_I420,
                width,
                height,
                align,
            )
        };

        if ret.is_null() {
            println!("VP9 video frame error: image allocation failed");
            return None;
        };

        // calls to `std::mem::forget` with a value that implements `Copy` does nothing
        // mem::forget(ret); // img and ret are the same

        let stride_u = raw.stride[1];
        let stride_v = raw.stride[2];
        let count = stride_u * stride_v;

        let mut y: Vec<u8> = match self.data_to_luminance(frame, flux) {
            Some(y) => y,
            None => vec![0; (width * height) as usize],
        };

        //invert and downscale
        let mut dst = vec![0; (width * height) as usize];
        self.resize_and_invert(&y, &mut dst, width, height, libyuv_FilterMode_kFilterBox);
        y = dst;

        let u: &[u8] = &vec![128; count as usize];
        let v: &[u8] = &vec![128; count as usize];

        raw.planes[0] = unsafe { mem::transmute(y.as_ptr()) };
        raw.planes[1] = unsafe { mem::transmute(u.as_ptr()) };
        raw.planes[2] = unsafe { mem::transmute(v.as_ptr()) };

        raw.stride[0] = width as i32;

        //flip the FITS image vertically
        //unsafe { vpx_img_flip(&mut raw) };
        //no need to use libvpx to invert the image, libyuv does it for us

        println!("VP9 video frame preparation time: {:?}", watch.elapsed());

        Some(raw)
    }

    #[cfg(feature = "ipp")]
    pub fn resize_and_invert(
        &self,
        src: &Vec<u8>,
        dst: &mut Vec<u8>,
        width: u32,
        height: u32,
        _: u32,
    ) {
        let num_threads = ((self.height as u32 / HEIGHT_PER_THREAD).max(1) as usize)
            .min(num_cpus::get_physical());
        println!(
            "[ipp_resize] src height: {}, using {} threads",
            self.height, num_threads
        );

        let pool = match rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
        {
            Ok(pool) => pool,
            Err(err) => {
                println!("[ipp_resize] {:?}", err);
                return;
            }
        };

        let srcSize = ipp_sys::IppiSize {
            width: self.width as i32,
            height: self.height as i32,
        };
        let dstSize = ipp_sys::IppiSize {
            width: width as i32,
            height: height as i32,
        };

        let srcStep = srcSize.width;
        let dstStep = dstSize.width;

        let mut specSize: i32 = 0;
        let mut initSize: i32 = 0;

        ipp_assert!(ipp_sys::ippiResizeGetSize_8u(
            srcSize,
            dstSize,
            IPPI_INTER_LANCZOS,
            0,
            &mut specSize,
            &mut initSize
        ));

        //memory allocation
        let pInitBuf = unsafe { ipp_sys::ippsMalloc_8u(initSize) };
        let pSpec = unsafe { ipp_sys::ippsMalloc_8u(specSize) };

        if pInitBuf.is_null() || pSpec.is_null() {
            println!("[ipp_resize] memory allocation error. aborting.");

            unsafe {
                ipp_sys::ippsFree(pInitBuf as *mut std::ffi::c_void);
                ipp_sys::ippsFree(pSpec as *mut std::ffi::c_void);
            };

            return;
        }

        let status = unsafe {
            ipp_sys::ippiResizeLanczosInit_8u(
                srcSize,
                dstSize,
                3,
                pSpec as *mut ipp_sys::IppiResizeSpec_32f,
                pInitBuf,
            )
        };
        unsafe { ipp_sys::ippsFree(pInitBuf as *mut std::ffi::c_void) };

        if status != ipp_sys::ippStsNoErr as i32 {
            println!("[ipp_resize] memory allocation error. aborting.");

            unsafe { ipp_sys::ippsFree(pSpec as *mut std::ffi::c_void) };
            return;
        }

        let mut borderSize: ipp_sys::IppiBorderSize = unsafe { mem::zeroed() };
        let status = unsafe {
            ipp_sys::ippiResizeGetBorderSize_8u(
                pSpec as *const ipp_sys::IppiResizeSpec_32f,
                &mut borderSize,
            )
        };

        if status != ipp_sys::ippStsNoErr as i32 {
            println!("[ipp_resize] memory allocation error. aborting.");

            unsafe { ipp_sys::ippsFree(pSpec as *mut std::ffi::c_void) };
            return;
        }

        //prepare the buffers
        let slice = dstSize.height / (num_threads as i32);
        let tail = dstSize.height % (num_threads as i32);

        let dstTileSize = ipp_sys::IppiSize {
            width: dstSize.width,
            height: slice,
        };

        let dstLastTileSize = ipp_sys::IppiSize {
            width: dstSize.width,
            height: slice + tail,
        };

        let mut bufSize1: i32 = 0;
        let mut bufSize2: i32 = 0;

        unsafe {
            ipp_sys::ippiResizeGetBufferSize_8u(
                pSpec as *const ipp_sys::IppiResizeSpec_32f,
                dstTileSize,
                ipp_sys::IppChannels::ippC1,
                &mut bufSize1,
            );
            ipp_sys::ippiResizeGetBufferSize_8u(
                pSpec as *const ipp_sys::IppiResizeSpec_32f,
                dstLastTileSize,
                ipp_sys::IppChannels::ippC1,
                &mut bufSize2,
            );
        };

        let pBuffer =
            unsafe { ipp_sys::ippsMalloc_8u(bufSize1 * (num_threads as i32 - 1) + bufSize2) };

        //parallel downscaling
        if !pBuffer.is_null() {
            let pBufferPtr = unsafe {
                slice::from_raw_parts_mut(
                    pBuffer,
                    (bufSize1 as usize) * (num_threads - 1) + (bufSize2 as usize),
                )
            };

            let pSpecPtr = unsafe { slice::from_raw_parts_mut(pSpec, specSize as usize) };

            pool.install(|| {
                (0..num_threads).into_par_iter().for_each(|i| {
                    let mut srcOffset: ipp_sys::IppiPoint = unsafe { mem::zeroed() };
                    let mut dstOffset: ipp_sys::IppiPoint = unsafe { mem::zeroed() };

                    let mut srcSizeT = ipp_sys::IppiSize { ..srcSize };
                    let mut dstSizeT = ipp_sys::IppiSize { ..dstTileSize };

                    dstSizeT.height = slice;
                    dstOffset.y += (i as i32) * slice;

                    if i == num_threads - 1 {
                        dstSizeT = ipp_sys::IppiSize { ..dstLastTileSize };
                    };

                    let status = unsafe {
                        ipp_sys::ippiResizeGetSrcRoi_8u(
                            pSpecPtr.as_ptr() as *const ipp_sys::IppiResizeSpec_32f,
                            dstOffset,
                            dstSizeT,
                            &mut srcOffset,
                            &mut srcSizeT,
                        )
                    };

                    if status == ipp_sys::ippStsNoErr as i32 {
                        let pSrcT = srcOffset.y * srcStep;
                        //let pDstT = dstOffset.y * dstStep;
                        //reverse the vertical order of tiles (a mirror image)
                        let pDstT = if i == num_threads - 1 {
                            0
                        } else {
                            (dstSize.height - (i as i32 + 1) * slice) * dstStep
                        };

                        let pOneBuf = i * (bufSize1 as usize);

                        let pSrc = &src[pSrcT as usize..];
                        let pDst = &dst[pDstT as usize..];
                        let pOneBuf = &pBufferPtr[pOneBuf..];

                        ipp_assert!(ipp_sys::ippiResizeLanczos_8u_C1R(
                            pSrc.as_ptr() as *mut u8,
                            srcStep,
                            pDst.as_ptr() as *mut u8,
                            dstStep,
                            dstOffset,
                            dstSizeT,
                            ipp_sys::_IppiBorderType::ippBorderRepl,
                            ptr::null(),
                            pSpecPtr.as_ptr() as *const ipp_sys::IppiResizeSpec_32f,
                            pOneBuf.as_ptr() as *mut u8
                        ));

                        //vertical mirror-image revert the buffer
                        unsafe {
                            spmd::revert_image_u8(
                                pDst.as_ptr() as *mut u8,
                                dstSizeT.width,
                                dstSizeT.height,
                            )
                        };
                    }
                })
            });
        }

        //memory clean-up
        unsafe { ipp_sys::ippsFree(pSpec as *mut std::ffi::c_void) };
        unsafe { ipp_sys::ippsFree(pBuffer as *mut std::ffi::c_void) };
    }

    #[cfg(not(feature = "ipp"))]
    pub fn resize_and_invert(
        &self,
        src: &Vec<u8>,
        dst: &mut Vec<u8>,
        width: u32,
        height: u32,
        filter: u32,
    ) {
        /*let src_width = self.width as f32 ;
        let src_height = self.height as f32 ;
        let scale_factor: f32 = (src_width as f32) / (width as f32) ;
        let filter_width = scale_factor * 1.0f32 ;
        let delta = filter_width;//.round();

        let len = src.len();

        println!("[custom_resize]: scale_factor = {}, filter_width = {}, delta = {}", scale_factor, filter_width, delta);*/

        //try the libyuv library
        unsafe {
            libyuv_ScalePlane(
                src.as_ptr(),
                self.width as i32,
                self.width as i32,
                -(self.height as i32),
                dst.as_mut_ptr(),
                width as i32,
                width as i32,
                height as i32,
                filter,
            );
        };

        /*let src_ptr = src.as_ptr() as *mut i8;
        let src_len = src.len() ;

        let dst_ptr = dst.as_ptr() as *mut i8;
        let dst_len = dst.len() ;

        unsafe {
            let src_raw = slice::from_raw_parts_mut(src_ptr, src_len);
            let dst_raw = slice::from_raw_parts_mut(dst_ptr, dst_len);

            bilinear_resize(src_raw.as_mut_ptr(), src_len as i32, dst_raw.as_mut_ptr(), dst_len as i32, src_width as i32, src_height as i32, width, height, scale_factor, filter_width) ;
        }*/

        //for each new downsized pixel
        /*for dst_y in 0..height {
            for dst_x in 0..width {
                let orig_x = scale_factor * (dst_x as f32) ;
                let orig_y = scale_factor * (dst_y as f32) ;

                let mut accum = 0.0f32;
                let mut pixel = 0.0f32;

                let mut src_x = orig_x - delta ;
                let mut src_y = orig_y - delta ;

                while src_x <= orig_x + delta {
                    while src_y <= orig_y + delta {
                        let coeff_x = 1.0 - (orig_x - src_x).abs()/scale_factor;
                        let coeff_y = 1.0 - (orig_y - src_y).abs()/scale_factor;
                        let coeff = coeff_x * coeff_y ;

                        let src_index = src_y.round() * (src_width) + src_x.round() ;
                        let src_index: usize = num::clamp(src_index as usize, 0, len-1) as usize;

                        pixel += (src[src_index] as f32) * coeff ;
                        accum += coeff ;
                        src_y += 1.0f32;//delta ;
                    }
                    src_x += 1.0f32;//delta ;
                }

                dst[(dst_y*width + dst_x) as usize] = num::clamp( (pixel / accum).round() as i32, 0, 255) as u8;
            }
        }*/
    }

    fn make_hevc_viewport(&self, dimx: u32, dimy: u32, y: &Vec<u8>) -> Option<Vec<Vec<u8>>> {
        let param: *mut x265_param = unsafe { x265_param_alloc() };

        if param.is_null() {
            return None;
        }

        let preset = CString::new("superfast").unwrap();
        let tune = CString::new("zerolatency").unwrap();

        unsafe {
            x265_param_default_preset(param, preset.as_ptr(), tune.as_ptr());

            (*param).fpsNum = 10;
            (*param).fpsDenom = 1;
        };

        //HEVC config
        unsafe {
            (*param).bRepeatHeaders = 1;
            (*param).internalCsp = X265_CSP_I400 as i32;
            (*param).internalBitDepth = 8;
            (*param).sourceWidth = dimx as i32;
            (*param).sourceHeight = dimy as i32;

            //constant quality
            (*param).rc.rateControlMode = X265_RC_METHODS_X265_RC_CQP as i32;
            (*param).rc.qp = 31;
        };

        let pic: *mut x265_picture = unsafe { x265_picture_alloc() };
        let enc: *mut x265_encoder = unsafe { x265_encoder_open(param) };
        unsafe { x265_picture_init(param, pic) };

        //HEVC-encode a still viewport
        let watch = Instant::now();

        unsafe {
            (*pic).stride[0] = dimx as i32;
            (*pic).planes[0] = y.as_ptr() as *mut std::os::raw::c_void;
        }

        let mut nal_count: u32 = 0;
        let mut p_nal: *mut x265_nal = ptr::null_mut();
        let p_out: *mut x265_picture = ptr::null_mut();

        //encode
        let ret = unsafe { x265_encoder_encode(enc, &mut p_nal, &mut nal_count, pic, p_out) };

        println!(
            "x265 hevc viewport encode time: {:?}, speed {} frames per second, ret = {}, nal_count = {}",
            watch.elapsed(),
            1000000000 / watch.elapsed().as_nanos(),
            ret,
            nal_count
        );

        //y falls out of scope
        unsafe {
            (*pic).stride[0] = 0 as i32;
            (*pic).planes[0] = ptr::null_mut();
        }

        let mut frames: Vec<Vec<u8>> = Vec::new();

        //process all NAL units one by one
        if nal_count > 0 {
            let nal_units = unsafe { std::slice::from_raw_parts(p_nal, nal_count as usize) };

            for unit in nal_units {
                println!("NAL unit type: {}, size: {}", unit.type_, unit.sizeBytes);

                let payload =
                    unsafe { std::slice::from_raw_parts(unit.payload, unit.sizeBytes as usize) };

                frames.push(payload.to_vec());
            }
        }

        //flush the encoder to signal the end
        loop {
            let ret = unsafe {
                x265_encoder_encode(enc, &mut p_nal, &mut nal_count, ptr::null_mut(), p_out)
            };

            if ret > 0 {
                println!("flushing the encoder, residual nal_count = {}", nal_count);

                let nal_units = unsafe { std::slice::from_raw_parts(p_nal, nal_count as usize) };

                for unit in nal_units {
                    println!("NAL unit type: {}, size: {}", unit.type_, unit.sizeBytes);

                    let payload = unsafe {
                        std::slice::from_raw_parts(unit.payload, unit.sizeBytes as usize)
                    };

                    frames.push(payload.to_vec());
                }
            } else {
                break;
            }
        }

        //release memory
        unsafe {
            if !param.is_null() {
                x265_param_free(param);
            }

            if !enc.is_null() {
                x265_encoder_close(enc);
            }

            if !pic.is_null() {
                x265_picture_free(pic);
            }
        }

        if frames.len() > 0 { Some(frames) } else { None }
    }

    fn make_vpx_viewport(&self, dimx: u32, dimy: u32, y: &Vec<u8>) -> Option<Vec<Vec<u8>>> {
        let watch = Instant::now();

        let mut image_frame: Vec<u8> = Vec::new();

        let mut raw: vpx_image = vpx_image::default();
        let mut ctx = vpx_codec_ctx_t {
            name: ptr::null(),
            iface: ptr::null_mut(),
            err: VPX_CODEC_ERROR,
            err_detail: ptr::null(),
            init_flags: 0,
            config: vpx_codec_ctx__bindgen_ty_1 { enc: ptr::null() },
            priv_: ptr::null_mut(),
        };

        let align = 1;

        //a workaround around a bug in libvpx triggered when h > w (dimy > dimx)
        let ret = if dimx > dimy {
            unsafe { vpx_img_alloc(&mut raw, vpx_img_fmt::VPX_IMG_FMT_I420, dimx, dimy, align) }
        } else {
            unsafe { vpx_img_alloc(&mut raw, vpx_img_fmt::VPX_IMG_FMT_I420, dimy, dimx, align) }
        };

        if ret.is_null() {
            println!("VP9 image frame error: image allocation failed");
            return None;
        }
        // calls to `std::mem::forget` with a value that implements `Copy` does nothing
        // mem::forget(ret); // img and ret are the same
        print!("dimx: {}, dimy: {}, {:#?}", dimx, dimy, raw);

        //I420
        let stride_u = raw.stride[1];
        let stride_v = raw.stride[2];
        let count = stride_u * stride_v;

        let u: &[u8] = &vec![128; count as usize];
        let v: &[u8] = &vec![128; count as usize];

        raw.planes[0] = unsafe { mem::transmute(y.as_ptr()) };
        raw.planes[1] = unsafe { mem::transmute(u.as_ptr()) };
        raw.planes[2] = unsafe { mem::transmute(v.as_ptr()) };

        //a workaround around a bug in libvpx triggered when h > w (dimy > dimx)
        raw.stride[0] = if dimx > dimy {
            dimx as i32
        } else {
            dimy as i32
        };

        //let mut cfg = vpx_codec_enc_cfg::default();
        let mut cfg = vpx_codec_enc_config_init();
        let mut ret = unsafe { vpx_codec_enc_config_default(vpx_codec_vp9_cx(), &mut cfg, 0) };

        if ret != VPX_CODEC_OK {
            println!("VP9 image frame error: default Configuration failed");

            //release the image
            unsafe { vpx_img_free(&mut raw) };

            return None;
        }

        //a workaround around a bug in libvpx triggered when h > w (dimy > dimx)
        if dimx > dimy {
            cfg.g_w = dimx;
            cfg.g_h = dimy;
        } else {
            cfg.g_w = dimy;
            cfg.g_h = dimx;
        }

        cfg.rc_min_quantizer = 10;
        cfg.rc_max_quantizer = 42;
        cfg.rc_target_bitrate = 4096; // [kilobits per second]
        cfg.g_pass = vpx_enc_pass::VPX_RC_ONE_PASS;
        cfg.g_threads = num_cpus::get_physical().min(4) as u32; //set the upper limit on the number of threads to 4

        ret = unsafe {
            vpx_codec_enc_init_ver(
                &mut ctx,
                vpx_codec_vp9_cx(),
                &mut cfg,
                0,
                VPX_ENCODER_ABI_VERSION as i32,
            )
        };

        if ret != VPX_CODEC_OK {
            println!("VP9 image frame error: codec init failed {:?}", ret);

            unsafe { vpx_img_free(&mut raw) };

            return None;
        }

        ret = unsafe {
            vpx_codec_control_(&mut ctx, vp8e_enc_control_id::VP8E_SET_CPUUSED as i32, 8)
        };

        if ret != VPX_CODEC_OK {
            println!("VP9: error setting VP8E_SET_CPUUSED {:?}", ret);
        }

        let mut flags = 0;
        flags |= VPX_EFLAG_FORCE_KF;

        //call encode_frame with a valid image
        match encode_frame(ctx, raw, 0, flags as i64, VPX_DL_BEST_QUALITY as u64) {
            Ok(res) => match res {
                Some(res) => image_frame = res,
                _ => {}
            },
            Err(err) => {
                println!("codec error: {:?}", err);

                unsafe { vpx_img_free(&mut raw) };
                unsafe { vpx_codec_destroy(&mut ctx) };

                return None;
            }
        };

        //flush the encoder to signal the end
        match flush_frame(ctx, VPX_DL_BEST_QUALITY as u64) {
            Ok(res) => match res {
                Some(res) => image_frame = res,
                _ => {}
            },
            Err(err) => {
                println!("codec error: {:?}", err);

                unsafe { vpx_img_free(&mut raw) };
                unsafe { vpx_codec_destroy(&mut ctx) };

                return None;
            }
        };

        if image_frame.is_empty() {
            println!("VP9 image frame error: no image packet produced");

            unsafe { vpx_img_free(&mut raw) };

            unsafe { vpx_codec_destroy(&mut ctx) };

            return None;
        }

        println!("VP9 image frame encode time: {:?}", watch.elapsed());

        unsafe { vpx_img_free(&mut raw) };
        unsafe { vpx_codec_destroy(&mut ctx) };

        Some(vec![image_frame; 1])
    }

    fn auto_brightness(
        &self,
        pixels: &Vec<f32>,
        mask: &Vec<u8>,
        black: f32,
        initial_sensitivity: f32,
    ) -> Option<f32> {
        println!("auto-adjusting brightness");
        let watch = Instant::now();

        if !initial_sensitivity.is_finite() {
            return None;
        }

        let target_brightness: f32 = 0.1;
        let max_iter = 20;
        let mut iter = 0;

        let mut sensitivity = initial_sensitivity;
        let mut a = 0.01 * sensitivity;
        let mut b = 100.0 * sensitivity;

        //perform the first step manually (verify that br(a) <= target_brightness <= br(b) )
        let a_brightness = self.get_brightness(pixels, mask, black, a);
        let b_brightness = self.get_brightness(pixels, mask, black, b);

        if target_brightness < a_brightness || target_brightness > b_brightness {
            return None;
        }

        loop {
            iter = iter + 1;
            sensitivity = (a + b) / 2.0;
            let brightness = self.get_brightness(pixels, mask, black, sensitivity);

            println!(
                "iteration: {}, sensitivity: {}, brightness: {} divergence: {}",
                iter,
                sensitivity,
                brightness,
                (target_brightness - brightness).abs()
            );

            if brightness > target_brightness {
                b = sensitivity
            }

            if brightness < target_brightness {
                a = sensitivity
            }

            if iter > max_iter {
                break;
            }

            if (target_brightness - brightness).abs() < 0.1 * target_brightness {
                break;
            }
        }

        //an approximate solution
        sensitivity = (a + b) / 2.0;

        println!(
            "final sensitivity: {}, elapsed time: {:?}",
            sensitivity,
            watch.elapsed()
        );

        Some(sensitivity)
    }

    fn get_brightness(
        &self,
        pixels: &Vec<f32>,
        mask: &Vec<u8>,
        black: f32,
        sensitivity: f32,
    ) -> f32 {
        let len = pixels.len();
        let tmp = (len / (1024 * 1024)).max(1);
        let num_threads = tmp.min(num_cpus::get_physical());
        let work_size = len / num_threads;

        /*println!(
            "get_brightness: tmp = {}, num_threads = {}, work_size = {}",
            tmp, num_threads, work_size
        );*/

        let sum: f32 = (0..num_threads)
            .into_par_iter()
            .map(|index| {
                let offset = index * work_size;

                let work_size = if index == num_threads - 1 {
                    len - offset
                } else {
                    work_size
                };

                let vec = &pixels[offset..offset + work_size];
                let ptr = vec.as_ptr() as *mut f32;
                let len = vec.len();

                let mask = &mask[offset..offset + work_size];
                let mask_ptr = mask.as_ptr() as *mut u8;
                let mask_len = mask.len();

                let brightness = unsafe {
                    let vec_raw = slice::from_raw_parts_mut(ptr, len);
                    let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                    spmd::pixels_mean_brightness_ratio(
                        vec_raw.as_mut_ptr(),
                        mask_raw.as_mut_ptr(),
                        black,
                        sensitivity,
                        len as u32,
                    )
                };

                /*println!(
                    "index: {}, offset: {}, work_size: {}, len = {}, brightness: {}",
                    index, offset, work_size, len, brightness
                );*/

                brightness
            })
            .sum();

        sum / (num_threads as f32)
    }

    fn make_vpx_image(&mut self) {
        //check if the .img binary image file is already in the IMAGECACHE

        let filename = format!("{}/{}.img", IMAGECACHE, self.dataset_id.replace("/", "_"));
        let filepath = std::path::Path::new(&filename);

        if filepath.exists() {
            return;
        }

        let watch = Instant::now();

        let mut image_frame: Vec<u8> = Vec::new();

        let mut w = self.width as u32;
        let mut h = self.height as u32;
        let pixel_count = (w as u64) * (h as u64);

        //multithreading
        /*let mut num_threads = 1;
        let mut thread_height = self.height;
        let mut thread_h = self.height;*/

        if pixel_count > IMAGE_PIXEL_COUNT_LIMIT {
            let ratio: f32 = ((pixel_count as f32) / (IMAGE_PIXEL_COUNT_LIMIT as f32)).sqrt();

            if ratio > 4.5 {
                //default scaling, no optimisations
                w = ((w as f32) / ratio) as u32;
                h = ((h as f32) / ratio) as u32;

                println!(
                    "downscaling the image from {}x{} to {}x{}, default ratio: {}",
                    self.width, self.height, w, h, ratio
                );

            /*num_threads = num_cpus::get_physical();
            thread_height = self.height / num_threads;
            thread_h = (h as usize) / num_threads;

            println!(
                "multi-threaded downscaling: thread_height = {}, thread_h = {}, #threads = {}",
                thread_height, thread_h, num_threads
            );*/
            } else if ratio > 3.0 {
                // 1/4
                w = w / 4;
                h = h / 4;

                println!(
                    "downscaling the image from {}x{} to {}x{} (1/4)",
                    self.width, self.height, w, h
                );
            } else if ratio > 2.25 {
                // 3/8
                w = 3 * w / 8;
                h = (h * 3 + 7) / 8;

                println!(
                    "downscaling the image from {}x{} to {}x{} (3/8)",
                    self.width, self.height, w, h
                );
            } else if ratio > 1.5 {
                // 1/2
                w = w / 2;
                h = h / 2;

                println!(
                    "downscaling the image from {}x{} to {}x{} (1/2)",
                    self.width, self.height, w, h
                );
            } else if ratio > 1.0 {
                // 3/4
                w = 3 * w / 4;
                h = 3 * h / 4;

                println!(
                    "downscaling the image from {}x{} to {}x{} (3/4)",
                    self.width, self.height, w, h
                );
            }
        }

        let mut raw: vpx_image = vpx_image::default();
        let mut ctx = vpx_codec_ctx_t {
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
            unsafe { vpx_img_alloc(&mut raw, vpx_img_fmt::VPX_IMG_FMT_I420, w, h, align) }
        //I420 or I444
        } else {
            unsafe { vpx_img_alloc(&mut raw, vpx_img_fmt::VPX_IMG_FMT_I420, h, w, align) }
            //I420 or I444
        };

        if ret.is_null() {
            println!("VP9 image frame error: image allocation failed");
            return;
        }
        // calls to `std::mem::forget` with a value that implements `Copy` does nothing
        // mem::forget(ret); // img and ret are the same
        println!("{:#?}", raw);

        let mut y: Vec<u8> = {
            let watch = Instant::now();

            let y: Vec<u8> = self.pixels_to_luminance(
                &self.pixels,
                &self.mask,
                self.pmin,
                self.pmax,
                self.lmin,
                self.lmax,
                self.black,
                self.white,
                self.median,
                self.sensitivity,
                self.ratio_sensitivity,
                &self.flux,
                &None,
            );

            println!(
                "VP9 image frame pixels to luminance time: {:?}",
                watch.elapsed()
            );

            y
        };

        {
            let watch = Instant::now();

            let mut dst = vec![0; (w as usize) * (h as usize)];
            self.resize_and_invert(&y, &mut dst, w, h, libyuv_FilterMode_kFilterBox);
            y = dst;

            println!(
                "VP9 image frame inverting/downscaling time: {:?}",
                watch.elapsed()
            );
        }

        let alpha_frame = {
            let watch = Instant::now();

            //invert/downscale the mask (alpha channel) without interpolation
            let mut alpha = vec![0; (w as usize) * (h as usize)];

            self.resize_and_invert(&self.mask, &mut alpha, w, h, libyuv_FilterMode_kFilterNone);

            let compressed_alpha = lz4_compress::compress(&alpha);

            println!(
                "alpha original length {}, lz4-compressed {} bytes, elapsed time {:?}",
                alpha.len(),
                compressed_alpha.len(),
                watch.elapsed()
            );

            compressed_alpha
        };

        /*let u : Vec<u8> = {
            self.mask.par_iter()
                .map(|&m| if m {
                    255
                }
                else {
                    0
                }
                )
                .collect()
        };*/

        //I444
        //let v : &[u8] = &vec![128; pixel_count as usize];

        //let y : &[u8] = &vec![128; pixel_count as usize];
        //I420
        let stride_u = raw.stride[1];
        let stride_v = raw.stride[2];
        let count = stride_u * stride_v;

        /*let u : &[u8] = &vec![128; (pixel_count/4) as usize];
        let v : &[u8] = &vec![128; (pixel_count/4) as usize];*/

        let u: &[u8] = &vec![128; count as usize];
        let v: &[u8] = &vec![128; count as usize];

        raw.planes[0] = unsafe { mem::transmute(y.as_ptr()) };
        raw.planes[1] = unsafe { mem::transmute(u.as_ptr()) };
        raw.planes[2] = unsafe { mem::transmute(v.as_ptr()) };

        //a workaround around a bug in libvpx triggered when h > w
        raw.stride[0] = if w > h { w as i32 } else { h as i32 };
        /*raw.stride[1] = (w/2) as i32 ;
        raw.stride[2] = (h/2) as i32 ;*/

        /*for i in 0..frame.buf.count() {
            let s: &[u8] = frame.buf.as_slice(i).unwrap();
            img.planes[i] = unsafe { mem::transmute(s.as_ptr()) };
            img.stride[i] = frame.buf.linesize(i).unwrap() as i32;
        }*/

        //let mut cfg = vpx_codec_enc_cfg::default();
        let mut cfg = vpx_codec_enc_config_init();
        let mut ret = unsafe { vpx_codec_enc_config_default(vpx_codec_vp9_cx(), &mut cfg, 0) };

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
        cfg.g_threads = num_cpus::get_physical().min(4) as u32; //set the upper limit on the number of threads to 4

        ret = unsafe {
            vpx_codec_enc_init_ver(
                &mut ctx,
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
            vpx_codec_control_(&mut ctx, vp8e_enc_control_id::VP8E_SET_CPUUSED as i32, 8)
        };

        if ret != VPX_CODEC_OK {
            println!("VP9: error setting VP8E_SET_CPUUSED {:?}", ret);
        }

        let mut flags = 0;
        flags |= VPX_EFLAG_FORCE_KF;

        //flip the FITS image vertically
        //unsafe { vpx_img_flip(&mut raw) };//disabled, libyuv will handle it

        //call encode_frame with a valid image
        match encode_frame(ctx, raw, 0, flags as i64, VPX_DL_BEST_QUALITY as u64) {
            Ok(res) => match res {
                Some(res) => image_frame = res,
                _ => {}
            },
            Err(err) => {
                println!("codec error: {:?}", err);

                unsafe { vpx_img_free(&mut raw) };
                unsafe { vpx_codec_destroy(&mut ctx) };

                return;
            }
        };

        //flush the encoder to signal the end
        match flush_frame(ctx, VPX_DL_BEST_QUALITY as u64) {
            Ok(res) => match res {
                Some(res) => image_frame = res,
                _ => {}
            },
            Err(err) => {
                println!("codec error: {:?}", err);

                unsafe { vpx_img_free(&mut raw) };
                unsafe { vpx_codec_destroy(&mut ctx) };

                return;
            }
        };

        //println!("{:?}", image_frame);

        if image_frame.is_empty() {
            println!("VP9 image frame error: no image packet produced");

            unsafe { vpx_img_free(&mut raw) };
            unsafe { vpx_codec_destroy(&mut ctx) };

            return;
        }

        println!("VP9 image frame encode time: {:?}", watch.elapsed());

        unsafe { vpx_img_free(&mut raw) };
        unsafe { vpx_codec_destroy(&mut ctx) };

        let tmp_filename = format!(
            "{}/{}.img.tmp",
            IMAGECACHE,
            self.dataset_id.replace("/", "_")
        );
        let tmp_filepath = std::path::Path::new(&tmp_filename);

        let mut buffer = match File::create(tmp_filepath) {
            Ok(f) => f,
            Err(err) => {
                println!("{}", err);
                return;
            }
        };

        let image_frame = FITSImage {
            identifier: String::from("VP9"),
            width: w,
            height: h,
            image: image_frame,
            alpha: alpha_frame,
        };

        match encode_to_vec(&image_frame, config::legacy()) {
            Ok(bin) => {
                println!("FITSImage binary length: {}", bin.len());

                match buffer.write_all(&bin) {
                    Ok(()) => {
                        //remove (rename) the temporary file
                        let _ = std::fs::rename(tmp_filepath, filepath);
                    }
                    Err(err) => {
                        println!(
                            "image cache write error: {}, removing the temporary file",
                            err
                        );
                        let _ = std::fs::remove_file(tmp_filepath);
                    }
                }
            }
            Err(err) => println!("error serializing a FITSImage structure: {}", err),
        }
    }

    pub fn get_viewport(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        user: &Option<UserParams>,
        wasm: bool,
        pool: &Option<rayon::ThreadPool>,
    ) -> Option<(u32, u32, Vec<Vec<u8>>, Vec<u8>, String)> {
        //spatial range checks
        let width = self.width as i32;
        let height = self.height as i32;

        if x1 < 0 || x1 > width - 1 {
            return None;
        }

        if x2 < 0 || x2 > width - 1 {
            return None;
        }

        if y1 < 0 || y1 > height - 1 {
            return None;
        }

        if y2 < 0 || y2 > height - 1 {
            return None;
        }

        /*let x1 = num::clamp(x1, 0, self.width - 1) as usize;
        let y1 = num::clamp(y1, 0, self.height - 1) as usize;

        let x2 = num::clamp(x2, 0, self.width - 1) as usize;
        let y2 = num::clamp(y2, 0, self.height - 1) as usize;*/

        let dimx = x2 - x1 + 1;
        let dimy = y2 - y1 + 1;

        if dimx < 0 || dimy < 0 {
            return None;
        }

        let (master_pixels, master_mask) = match user {
            Some(params) => (&params.pixels, &params.mask),
            None => (&self.pixels, &self.mask),
        };

        let mut pixels: Vec<f32> = vec![0.0; (dimx as usize) * (dimy as usize)];
        let mut mask: Vec<u8> = vec![0; (dimx as usize) * (dimy as usize)];

        for j in y1..y2 + 1 {
            let src_offset = (j * width) as usize;
            let mut dst_offset = (((dimy - 1) - (j - y1)) * dimx) as usize;

            for i in x1..x2 + 1 {
                let valid_pixel = (i >= 0) && (i < width) && (j >= 0) && (j < height);

                if valid_pixel {
                    pixels[dst_offset] = master_pixels[src_offset + i as usize];
                    mask[dst_offset] = master_mask[src_offset + i as usize];
                }

                dst_offset = dst_offset + 1;
            }
        }

        let y = match user {
            Some(params) => self.pixels_to_luminance(
                &pixels,
                &mask,
                params.pmin,
                params.pmax,
                params.lmin,
                params.lmax,
                params.black,
                params.white,
                params.median,
                params.sensitivity,
                params.ratio_sensitivity,
                &params.flux,
                pool,
            ),
            None => self.pixels_to_luminance(
                &pixels,
                &mask,
                self.pmin,
                self.pmax,
                self.lmin,
                self.lmax,
                self.black,
                self.white,
                self.median,
                self.sensitivity,
                self.ratio_sensitivity,
                &self.flux,
                pool,
            ),
        };

        //x265 can only work with dimensions >= 32; in addition libxpv seems more efficient compression-size-wise for small images...
        let method = if !wasm {
            println!("wasm unsupported, switching over to VP9");
            fits::Codec::VPX
        } else {
            if dimx < 128 || dimy < 128 {
                println!(
                    "viewport too small ({}x{}), switching over to VP9",
                    dimx, dimy
                );
                Codec::VPX
            } else {
                println!("wasm supported, using HEVC");
                fits::Codec::HEVC
            }
        };

        let alpha = lz4_compress::compress(&mask);

        match method {
            Codec::VPX => match self.make_vpx_viewport(dimx as u32, dimy as u32, &y) {
                Some(frame) => Some((dimx as u32, dimy as u32, frame, alpha, String::from("VP9"))),
                None => None,
            },
            Codec::HEVC => match self.make_hevc_viewport(dimx as u32, dimy as u32, &y) {
                Some(frame) => Some((dimx as u32, dimy as u32, frame, alpha, String::from("HEVC"))),
                None => None,
            },
        }
    }

    fn split_wcs(coord: &String) -> (String, String) {
        let tmp: Vec<String> = coord.split(':').map(|s| s.to_string()).collect();

        if tmp.len() == 2 {
            let key = &tmp[0];
            let value = &tmp[1].replace("\\", "");

            return (key.trim().to_string(), value.trim().to_string());
        };

        return (String::from("N/A"), String::from("N/A"));
    }

    pub fn get_csv_spectrum(
        &self,
        ra: &String,
        dec: &String,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        beam: Beam,
        intensity: Intensity,
        frame_start: f64,
        frame_end: f64,
        ref_freq: f64,
        delta_v: f64,
        rest: bool,
        pool: &Option<rayon::ThreadPool>,
    ) -> Option<String> {
        if self.depth <= 1 {
            return None;
        }

        // viewport dimensions
        let dimx = (x2 - x1 + 1).abs();
        let dimy = (y2 - y1 + 1).abs();

        let cx = 1 + (x1 + x2) >> 1;
        let cy = 1 + (y1 + y2) >> 1;

        let rx = (x2 - x1).abs() >> 1;
        let ry = (y2 - y1).abs() >> 1;

        let (start, end) = match self.get_spectrum_range(frame_start, frame_end, ref_freq) {
            Some(frame) => frame,
            None => {
                println!("error: an invalid spectrum range");
                return None;
            }
        };

        let (lng_value, lat_value) = self.pix_to_world(cx, cy);
        let (ra1, dec1) = self.pix_to_world(cx - rx, cy - ry);
        let (ra2, dec2) = self.pix_to_world(cx + rx, cy + ry);

        let beam_width = (ra2 - ra1).abs(); // [deg]
        let beam_height = (dec2 - dec1).abs(); // [deg]

        println!(
            "first channel: {}, last channel: {}, ra {} [deg], dec {} [deg], beam width [deg]: {}, beam height [deg]: {}",
            start, end, lng_value, lat_value, beam_width, beam_height
        );

        let mut intensity_column = format!("intensity [{}", self.beam_unit);

        match intensity {
            Intensity::Mean => {
                intensity_column = format!("mean {}", intensity_column);
            }
            Intensity::Integrated => {
                intensity_column = format!("integrated {}", intensity_column);

                if self.has_velocity {
                    intensity_column = format!("{}•km/s", intensity_column);
                };
            }
        };

        intensity_column = format!("{}]", intensity_column);

        let beam_type = match beam {
            Beam::Circle => String::from("circle"),
            Beam::Square => String::from("square/rect."),
        };

        let mut frequency_column = format!("frequency [GHz]");

        if rest {
            frequency_column = format!("rest {}", frequency_column);
        };

        let (ra_suffix, ra_value) = FITS::split_wcs(ra);
        let (dec_suffix, dec_value) = FITS::split_wcs(dec);

        let ra_column = format!("ra ({})", ra_suffix);
        let dec_column = format!("dec ({})", dec_suffix);

        let lng_column = "wcs.lng [deg]";
        let lat_column = "wcs.lat [deg]";

        println!(
            "intensity column: '{}', frequency column: '{}', ra column: '{}', dec column: '{}'",
            intensity_column, frequency_column, ra_column, dec_column
        );

        let mut has_header = false;

        match self.get_spectrum(
            x1,
            y1,
            x2,
            y2,
            beam.clone(),
            intensity,
            frame_start,
            frame_end,
            ref_freq,
            pool,
        ) {
            Some(spectrum) => {
                // create an in-memory CSV writer
                /*let mut wtr = WriterBuilder::new()
                .terminator(Terminator::CRLF)
                .quote_style(QuoteStyle::Never)
                .from_writer(vec![]);*/

                // let's roll our own in-memory CSV writer
                let mut stream = BufWriter::new(Vec::new());

                // create a '# comment' CSV header with information common to all rows

                // ra/dec
                let _ = stream.write(format!("# {}: {}\n", ra_column, ra_value).as_bytes());
                let _ = stream.write(format!("# {}: {}\n", dec_column, dec_value).as_bytes());

                // lng / lat [deg]
                let _ = stream.write(format!("# {}: {}\n", lng_column, lng_value).as_bytes());
                let _ = stream.write(format!("# {}: {}\n", lat_column, lat_value).as_bytes());

                // beam type
                let _ = stream.write(format!("# region type: {}\n", beam_type).as_bytes());

                // beam cx / cy [px]
                let _ = stream.write(format!("# region centre (x) [px]: {}\n", cx).as_bytes());
                let _ = stream.write(format!("# region centre (y) [px]: {}\n", cy).as_bytes());

                match beam {
                    Beam::Circle => {
                        // beam diameter [deg]
                        let _ = stream
                            .write(format!("# region diameter [deg]: {}\n", beam_width).as_bytes());

                        // beam diameter [px]
                        let _ =
                            stream.write(format!("# region diameter [px]: {}\n", dimx).as_bytes());
                    }
                    Beam::Square => {
                        // beam width / height [deg]
                        let _ = stream
                            .write(format!("# region width [deg]: {}\n", beam_width).as_bytes());
                        let _ = stream
                            .write(format!("# region height [deg]: {}\n", beam_height).as_bytes());

                        // beam width / height [px]
                        let _ = stream.write(format!("# region width [px]: {}\n", dimx).as_bytes());
                        let _ =
                            stream.write(format!("# region height [px]: {}\n", dimy).as_bytes());
                    }
                };

                // specsys
                let _ = stream
                    .write(format!("# spectral reference frame: {}\n", self.specsys).as_bytes());

                // deltaV [km/s]
                let _ = stream.write(
                    format!("# source velocity [km/s]: {}\n", (delta_v / 1000.0)).as_bytes(),
                );

                // ref_freq [GHz]
                if ref_freq > 0.0 {
                    let _ = stream.write(
                        format!("# reference frequency [GHz]: {}\n", (ref_freq / 1.0e9)).as_bytes(),
                    );
                }

                for i in 0..spectrum.len() {
                    let frame = start + i + 1;

                    let (f, v) = self.get_frame2freq_vel(frame, ref_freq, delta_v, rest);

                    /*println!(
                        "channel: {}, f: {} GHz, v: {} km/s, intensity: {}",
                        frame, f, v, spectrum[i]
                    );*/

                    if f != std::f64::NAN && v != std::f64::NAN {
                        // write the CSV header
                        if !has_header {
                            let _ = stream.write(b"\"channel\",");
                            let _ = stream.write(format!("\"{}\",", frequency_column).as_bytes());
                            let _ = stream.write(b"\"velocity [km/s]\",");
                            let _ = stream.write(format!("\"{}\"\n", intensity_column).as_bytes());

                            has_header = true;
                        }

                        // write out CSV values
                        let _ = stream.write(format!("{},", frame).as_bytes());
                        let _ = stream.write(format!("{},", f).as_bytes());
                        let _ = stream.write(format!("{},", v).as_bytes());
                        let _ = stream.write(format!("{}\n", spectrum[i]).as_bytes());

                        continue;
                    }

                    if v != std::f64::NAN {
                        // write the CSV header
                        if !has_header {
                            let _ = stream.write(b"\"channel\",");
                            let _ = stream.write(b"\"velocity [km/s]\",");
                            let _ = stream.write(format!("\"{}\"\n", intensity_column).as_bytes());

                            has_header = true;
                        }

                        // write out CSV values
                        let _ = stream.write(format!("{},", frame).as_bytes());
                        let _ = stream.write(format!("{},", v).as_bytes());
                        let _ = stream.write(format!("{}\n", spectrum[i]).as_bytes());

                        continue;
                    }

                    if f != std::f64::NAN {
                        // write the CSV header
                        if !has_header {
                            let _ = stream.write(b"\"channel\",");
                            let _ = stream.write(format!("\"{}\",", frequency_column).as_bytes());
                            let _ = stream.write(format!("\"{}\"\n", intensity_column).as_bytes());

                            has_header = true;
                        }

                        // write out CSV values
                        let _ = stream.write(format!("{},", frame).as_bytes());
                        let _ = stream.write(format!("{},", f).as_bytes());
                        let _ = stream.write(format!("{}\n", spectrum[i]).as_bytes());

                        continue;
                    }
                }

                // flush the CSV stream
                let _ = stream.flush();

                match stream.into_inner() {
                    Ok(w) => match String::from_utf8(w) {
                        Ok(csv) => Some(csv),
                        Err(err) => {
                            println!("csv2utf8 conversion error: {}", err);
                            None
                        }
                    },
                    Err(err) => {
                        println!("CSV into_inner() error: {}", err);
                        None
                    }
                }
            }
            None => None,
        }
    }

    pub fn get_spectrum(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        beam: Beam,
        intensity: Intensity,
        frame_start: f64,
        frame_end: f64,
        ref_freq: f64,
        pool: &Option<rayon::ThreadPool>,
    ) -> Option<Vec<f32>> {
        if self.depth <= 1 {
            return None;
        }

        //spatial range checks
        let x1 = num::clamp(x1, 0, self.width as i32 - 1) as usize;
        let y1 = num::clamp(y1, 0, self.height as i32 - 1) as usize;

        let x2 = num::clamp(x2, 0, self.width as i32 - 1) as usize;
        let y2 = num::clamp(y2, 0, self.height as i32 - 1) as usize;

        let cdelt3 = {
            if self.has_velocity && self.depth > 1 {
                self.cdelt3 * self.frame_multiplier / 1000.0
            } else {
                1.0
            }
        };

        match self.get_spectrum_range(frame_start, frame_end, ref_freq) {
            Some((start, end)) => {
                println!("start:{} end:{}", start, end);

                let mean = match intensity {
                    Intensity::Mean => true,
                    _ => false,
                };

                let watch = Instant::now();

                let spectrum: Vec<f32> = match beam {
                    Beam::Circle => {
                        //calculate the centre and squared radius
                        let cx = (x1 + x2) >> 1;
                        let cy = (y1 + y2) >> 1;
                        let r = ((x2 - x1) >> 1).min((y2 - y1) >> 1);
                        let r2 = r * r;

                        println!("cx = {}, cy = {}, r = {}", cx, cy, r);

                        match pool {
                            Some(pool) => pool.install(|| {
                                (start..end + 1)
                                    .into_par_iter()
                                    .map(|frame| {
                                        self.get_radial_spectrum_at_ispc(
                                            frame,
                                            x1,
                                            x2,
                                            y1,
                                            y2,
                                            cx,
                                            cy,
                                            r2,
                                            mean,
                                            cdelt3 as f32,
                                        )
                                    })
                                    .collect()
                            }),
                            None => (start..end + 1)
                                .into_par_iter()
                                .map(|frame| {
                                    self.get_radial_spectrum_at_ispc(
                                        frame,
                                        x1,
                                        x2,
                                        y1,
                                        y2,
                                        cx,
                                        cy,
                                        r2,
                                        mean,
                                        cdelt3 as f32,
                                    )
                                })
                                .collect(),
                        }
                    }
                    _ => match pool {
                        Some(pool) => pool.install(|| {
                            (start..end + 1)
                                .into_par_iter()
                                .map(|frame| {
                                    self.get_square_spectrum_at_ispc(
                                        frame,
                                        x1,
                                        x2,
                                        y1,
                                        y2,
                                        mean,
                                        cdelt3 as f32,
                                    )
                                })
                                .collect()
                        }),
                        None => (start..end + 1)
                            .into_par_iter()
                            .map(|frame| {
                                self.get_square_spectrum_at_ispc(
                                    frame,
                                    x1,
                                    x2,
                                    y1,
                                    y2,
                                    mean,
                                    cdelt3 as f32,
                                )
                            })
                            .collect(),
                    },
                };

                //println!("{:?}", spectrum);
                println!(
                    "spectrum length = {}, elapsed time: {:?}",
                    spectrum.len(),
                    watch.elapsed()
                );

                //return the spectrum
                Some(spectrum)
            }
            None => {
                println!("error: an invalid spectrum range");
                None
            }
        }
    }

    fn get_radial_spectrum_at_ispc(
        &self,
        frame: usize,
        x1: usize,
        x2: usize,
        y1: usize,
        y2: usize,
        cx: usize,
        cy: usize,
        r2: usize,
        mean: bool,
        cdelt3: f32,
    ) -> f32 {
        match self.bitpix {
            -32 => {
                let vec = &self.data_f16[frame];
                let ptr = vec.as_ptr() as *mut i16;
                let len = vec.len();

                if len > 0 {
                    unsafe {
                        let raw = slice::from_raw_parts_mut(ptr, len);

                        let spectrum = spmd::calculate_radial_spectrumF16(
                            raw.as_mut_ptr(),
                            self.bzero,
                            self.bscale,
                            self.datamin,
                            self.datamax,
                            self.width as u32,
                            x1 as i32,
                            x2 as i32,
                            y1 as i32,
                            y2 as i32,
                            cx as i32,
                            cy as i32,
                            r2 as i32,
                            mean,
                            cdelt3,
                        );

                        spectrum
                    }
                } else {
                    0.0
                }
            }
            _ => {
                println!(
                    "SIMD support for bitpix={} unavailable, switching to normal Rust",
                    self.bitpix
                );

                self.get_radial_spectrum_at(frame, x1, x2, y1, y2, cx, cy, r2, mean, cdelt3)
            }
        }
    }

    fn get_radial_spectrum_at(
        &self,
        frame: usize,
        x1: usize,
        x2: usize,
        y1: usize,
        y2: usize,
        cx: usize,
        cy: usize,
        r2: usize,
        mean: bool,
        cdelt3: f32,
    ) -> f32 {
        let mut sum: f32 = 0.0;
        let mut count: i32 = 0;

        match self.bitpix {
            8 => {
                let vec = &self.data_u8[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let int8 = vec[offset + x];

                            let tmp = self.bzero + self.bscale * (int8 as f32);
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                let dist2 = (cx - x) * (cx - x) + (cy - y) * (cy - y);

                                if dist2 <= r2 {
                                    sum += tmp;
                                    count += 1;
                                };
                            };
                        }
                    }
                }
            }
            16 => {
                let vec = &self.data_i16[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let int16 = vec[offset + x];

                            let tmp = self.bzero + self.bscale * (int16 as f32);
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                let dist2 = (cx - x) * (cx - x) + (cy - y) * (cy - y);

                                if dist2 <= r2 {
                                    sum += tmp;
                                    count += 1;
                                };
                            };
                        }
                    }
                }
            }
            32 => {
                let vec = &self.data_i32[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let int32 = vec[offset + x];

                            let tmp = self.bzero + self.bscale * (int32 as f32);
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                let dist2 = (cx - x) * (cx - x) + (cy - y) * (cy - y);

                                if dist2 <= r2 {
                                    sum += tmp;
                                    count += 1;
                                };
                            };
                        }
                    }
                }
            }
            -32 => {
                let vec = &self.data_f16[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let float16 = vec[offset + x];
                            //for float16 in &vec[(offset+x1)..(offset+x2)] {//there is no x
                            if float16.is_finite() {
                                let tmp = self.bzero + self.bscale * float16.to_f32();
                                if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                    let dist2 = (cx - x) * (cx - x) + (cy - y) * (cy - y);

                                    if dist2 <= r2 {
                                        sum += tmp;
                                        count += 1;
                                    };
                                };
                            };
                        }
                    }
                }
            }
            -64 => {
                let vec = &self.data_f64[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let float64 = vec[offset + x];

                            if float64.is_finite() {
                                let tmp = self.bzero + self.bscale * (float64 as f32);
                                if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                    let dist2 = (cx - x) * (cx - x) + (cy - y) * (cy - y);

                                    if dist2 <= r2 {
                                        sum += tmp;
                                        count += 1;
                                    };
                                };
                            };
                        }
                    }
                }
            }
            _ => println!("unsupported bitpix: {}", self.bitpix),
        };

        if count > 0 {
            if mean {
                //mean intensity
                sum / (count as f32)
            } else {
                //integrated intensity
                sum * cdelt3
            }
        } else {
            0.0
        }
    }

    fn get_square_spectrum_at_ispc(
        &self,
        frame: usize,
        x1: usize,
        x2: usize,
        y1: usize,
        y2: usize,
        mean: bool,
        cdelt3: f32,
    ) -> f32 {
        match self.bitpix {
            -32 => {
                let vec = &self.data_f16[frame];
                let ptr = vec.as_ptr() as *mut i16;
                let len = vec.len();

                if len > 0 {
                    unsafe {
                        let raw = slice::from_raw_parts_mut(ptr, len);

                        let spectrum = spmd::calculate_square_spectrumF16(
                            raw.as_mut_ptr(),
                            self.bzero,
                            self.bscale,
                            self.datamin,
                            self.datamax,
                            self.width as u32,
                            x1 as i32,
                            x2 as i32,
                            y1 as i32,
                            y2 as i32,
                            mean,
                            cdelt3,
                        );

                        spectrum
                    }
                } else {
                    0.0
                }
            }
            _ => {
                println!(
                    "SIMD support for bitpix={} unavailable, switching to normal Rust",
                    self.bitpix
                );

                self.get_square_spectrum_at(frame, x1, x2, y1, y2, mean, cdelt3)
            }
        }
    }

    fn get_square_spectrum_at(
        &self,
        frame: usize,
        x1: usize,
        x2: usize,
        y1: usize,
        y2: usize,
        mean: bool,
        cdelt3: f32,
    ) -> f32 {
        let mut sum: f32 = 0.0;
        let mut count: i32 = 0;

        match self.bitpix {
            8 => {
                let vec = &self.data_u8[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let int8 = vec[offset + x];

                            let tmp = self.bzero + self.bscale * (int8 as f32);
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                sum += tmp;
                                count += 1;
                            };
                        }
                    }
                }
            }
            16 => {
                let vec = &self.data_i16[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let int16 = vec[offset + x];

                            let tmp = self.bzero + self.bscale * (int16 as f32);
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                sum += tmp;
                                count += 1;
                            };
                        }
                    }
                }
            }
            32 => {
                let vec = &self.data_i32[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let int32 = vec[offset + x];

                            let tmp = self.bzero + self.bscale * (int32 as f32);
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                sum += tmp;
                                count += 1;
                            };
                        }
                    }
                }
            }
            -32 => {
                let vec = &self.data_f16[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let float16 = vec[offset + x];

                            if float16.is_finite() {
                                let tmp = self.bzero + self.bscale * float16.to_f32();
                                if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                    sum += tmp;
                                    count += 1;
                                };
                            };
                        }
                    }
                }
            }
            -64 => {
                let vec = &self.data_f64[frame];
                if vec.len() > 0 {
                    for y in y1..y2 {
                        let offset = y * self.width as usize;
                        for x in x1..x2 {
                            let float64 = vec[offset + x];

                            if float64.is_finite() {
                                let tmp = self.bzero + self.bscale * (float64 as f32);
                                if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                    sum += tmp;
                                    count += 1;
                                };
                            };
                        }
                    }
                }
            }
            _ => println!("unsupported bitpix: {}", self.bitpix),
        };

        if count > 0 {
            if mean {
                //mean intensity
                sum / (count as f32)
            } else {
                //integrated intensity
                sum * cdelt3
            }
        } else {
            0.0
        }
    }

    fn Einstein_velocity_addition(v1: f64, v2: f64) -> f64 {
        let c = 299792458_f64; //speed of light [m/s]

        return (v1 + v2) / (1.0 + v1 * v2 / (c * c));
    }

    fn Einstein_relative_velocity(f: f64, f0: f64, delta_v: f64) -> f64 {
        let c = 299792458_f64; //speed of light [m/s]

        let f_ratio = f / f0;
        let v = (1.0 - f_ratio * f_ratio) / (1.0 + f_ratio * f_ratio) * c;

        return FITS::Einstein_velocity_addition(v, delta_v);
    }

    fn relativistic_rest_frequency(f: f64, delta_v: f64) -> f64 {
        let c = 299792458_f64; //speed of light [m/s]

        let beta = delta_v / c;

        let tmp = ((1.0 + beta) / (1.0 - beta)).sqrt();

        return f * tmp;
    }

    fn get_frame2freq_vel(
        &self,
        frame: usize,
        ref_freq: f64,
        delta_v: f64,
        rest: bool,
    ) -> (f64, f64) {
        let has_velocity = self.has_velocity;

        let has_frequency = if ref_freq > 0.0 {
            true
        } else {
            self.has_frequency
        };

        if has_velocity && has_frequency {
            let c = 299792458_f64; //speed of light [m/s]

            // go from v to f then apply a Δv correction to v
            let mut v = self.crval3 * self.frame_multiplier
                + self.cdelt3 * self.frame_multiplier * (frame as f64 - self.crpix3); // [m/s]

            let mut f = ref_freq * ((1.0 - v / c) / (1.0 + v / c)).sqrt(); // [Hz]

            if rest {
                f = FITS::relativistic_rest_frequency(f, delta_v);
            }

            // find the corresponding velocity
            v = FITS::Einstein_relative_velocity(f, ref_freq, delta_v);

            return (f / 1.0e9, v / 1000.0); // [GHz], [km/s]
        };

        let val = self.crval3 * self.frame_multiplier
            + self.cdelt3 * self.frame_multiplier * (frame as f64 - self.crpix3);

        if has_frequency {
            let f = if rest {
                FITS::relativistic_rest_frequency(val, delta_v)
            } else {
                val
            };

            // find the corresponding velocity
            let v = FITS::Einstein_relative_velocity(f, ref_freq, delta_v);

            return (f / 1.0e9, v / 1000.0); // [GHz], [km/s]
        };

        if has_velocity {
            // no frequency info, only velocity

            // what about Δv ???

            return (std::f64::NAN, val / 1000.0); // [km/s]
        };

        // if we got to this point there is nothing to report ...
        return (std::f64::NAN, std::f64::NAN);
    }

    fn pix_to_world(&self, x: i32, y: i32) -> (f64, f64) {
        let ra = if self.ctype1.contains("RA")
            || self.ctype1.contains("GLON")
            || self.ctype1.contains("ELON")
        {
            self.crval1 + (x as f64 - self.crpix1) * self.cdelt1 // [deg]
        } else {
            std::f64::NAN
        };

        let dec = if self.ctype2.contains("DEC")
            || self.ctype2.contains("GLAT")
            || self.ctype2.contains("ELAT")
        {
            self.crval2 + (y as f64 - self.crpix2) * self.cdelt2 // [deg]
        } else {
            std::f64::NAN
        };

        return (ra, dec);
    }

    pub fn get_spectrum_range(
        &self,
        frame_start: f64,
        frame_end: f64,
        ref_freq: f64,
    ) -> Option<(usize, usize)> {
        if self.depth > 1 {
            if self.has_velocity && ref_freq > 0.0 {
                return Some(self.get_freq2vel_bounds(frame_start, frame_end, ref_freq));
            };

            if self.has_frequency && ref_freq > 0.0 {
                return Some(self.get_frequency_bounds(frame_start, frame_end));
            };

            if self.has_velocity {
                return Some(self.get_velocity_bounds(frame_start, frame_end));
            }
        } else {
            return Some((0, 0));
        };

        /*
        //a default empty range
        None
        */
        //by default return the original range as index numbers - 1
        let mut start = frame_start.round() as usize - 1;
        let mut end = frame_end.round() as usize - 1;

        if end < start {
            let tmp = start;
            start = end;
            end = tmp;
        };

        start = start.max(0);
        start = start.min(self.depth - 1);

        end = end.max(0);
        end = end.min(self.depth - 1);

        Some((start, end))
    }

    fn get_freq2vel_bounds(
        &self,
        frame_start: f64,
        frame_end: f64,
        ref_freq: f64,
    ) -> (usize, usize) {
        let c = 299792458_f64; //speed of light [m/s]

        let f_ratio = frame_start / ref_freq;
        let v1 = (1.0 - f_ratio * f_ratio) / (1.0 + f_ratio * f_ratio) * c;

        let f_ratio = frame_end / ref_freq;
        let v2 = (1.0 - f_ratio * f_ratio) / (1.0 + f_ratio * f_ratio) * c;

        let x1 = self.crpix3
            + (v1 - self.crval3 * self.frame_multiplier) / (self.cdelt3 * self.frame_multiplier)
            - 1.0;
        let x2 = self.crpix3
            + (v2 - self.crval3 * self.frame_multiplier) / (self.cdelt3 * self.frame_multiplier)
            - 1.0;

        let mut start = x1.round() as usize;
        let mut end = x2.round() as usize;

        if self.cdelt3 < 0.0 {
            start = self.depth - 1 - start;
            end = self.depth - 1 - end;
        };

        if end < start {
            let tmp = start;
            start = end;
            end = tmp;
        };

        start = start.max(0);
        start = start.min(self.depth - 1);

        end = end.max(0);
        end = end.min(self.depth - 1);

        (start, end)
    }

    fn get_frequency_bounds(&self, freq_start: f64, freq_end: f64) -> (usize, usize) {
        let mut start = 0;
        let mut end = self.depth - 1;

        if freq_start > 0.0 && freq_end > 0.0 {
            let f1 = self.crval3 * self.frame_multiplier
                + self.cdelt3 * self.frame_multiplier * (1.0 - self.crpix3);
            let f2 = self.crval3 * self.frame_multiplier
                + self.cdelt3 * self.frame_multiplier * ((self.depth as f64) - self.crpix3);

            let band_lo = f1.min(f2);
            let band_hi = f1.max(f2);

            if self.cdelt3 > 0.0 {
                start = ((freq_start - band_lo) / (band_hi - band_lo) * (self.depth as f64 - 1.0))
                    .round() as usize;
                end = ((freq_end - band_lo) / (band_hi - band_lo) * (self.depth as f64 - 1.0))
                    .round() as usize;
            } else {
                start = ((band_hi - freq_start) / (band_hi - band_lo) * (self.depth as f64 - 1.0))
                    .round() as usize;
                end = ((band_hi - freq_end) / (band_hi - band_lo) * (self.depth as f64 - 1.0))
                    .round() as usize;
            };

            if end < start {
                let tmp = start;
                start = end;
                end = tmp;
            };

            start = start.max(0);
            start = start.min(self.depth - 1);

            end = end.max(0);
            end = end.min(self.depth - 1);
        };

        (start, end)
    }

    fn get_velocity_bounds(&self, vel_start: f64, vel_end: f64) -> (usize, usize) {
        let mut start;
        let mut end;

        let v1 = self.crval3 * self.frame_multiplier
            + self.cdelt3 * self.frame_multiplier * (1.0 - self.crpix3);
        let v2 = self.crval3 * self.frame_multiplier
            + self.cdelt3 * self.frame_multiplier * ((self.depth as f64) - self.crpix3);

        let band_lo = v1.min(v2);
        let band_hi = v1.max(v2);

        if self.cdelt3 > 0.0 {
            start = ((vel_start - band_lo) / (band_hi - band_lo) * (self.depth as f64 - 1.0))
                .round() as usize;
            end = ((vel_end - band_lo) / (band_hi - band_lo) * (self.depth as f64 - 1.0)).round()
                as usize;
        } else {
            start = ((band_hi - vel_start) / (band_hi - band_lo) * (self.depth as f64 - 1.0))
                .round() as usize;
            end = ((band_hi - vel_end) / (band_hi - band_lo) * (self.depth as f64 - 1.0)).round()
                as usize;
        };

        if end < start {
            let tmp = start;
            start = end;
            end = tmp;
        };

        start = start.max(0);
        start = start.min(self.depth - 1);

        end = end.max(0);
        end = end.min(self.depth - 1);

        (start, end)
    }

    pub fn get_frequency_range(&self) -> (f64, f64) {
        let mut fmin: f64 = 0.0;
        let mut fmax: f64 = 0.0;

        if self.depth > 1 && self.has_frequency {
            let f1;
            let f2;

            if self.has_velocity {
                let c = 299792458_f64; //speed of light [m/s]

                let v1: f64 = self.crval3 * self.frame_multiplier
                    + self.cdelt3 * self.frame_multiplier * (1.0 - self.crpix3);

                let v2: f64 = self.crval3 * self.frame_multiplier
                    + self.cdelt3 * self.frame_multiplier * ((self.depth as f64) - self.crpix3);

                f1 = self.restfrq * ((1.0 - v1 / c) / (1.0 + v1 / c)).sqrt();
                f2 = self.restfrq * ((1.0 - v2 / c) / (1.0 + v2 / c)).sqrt();
            } else {
                f1 = self.crval3 * self.frame_multiplier
                    + self.cdelt3 * self.frame_multiplier * (1.0 - self.crpix3);

                f2 = self.crval3 * self.frame_multiplier
                    + self.cdelt3 * self.frame_multiplier * ((self.depth as f64) - self.crpix3);
            };

            fmin = f1.min(f2);
            fmax = f1.max(f2);
        }

        (fmin / 1000000000.0, fmax / 1000000000.0)
    }

    pub fn to_json(&self) -> String {
        let value = json!({
                "HEADER" : self.header,
                "width" : self.width,
                "height" : self.height,
                "depth" : self.depth,
                "polarisation" : self.polarisation,
                "filesize" : self.filesize,
                "url": self.url,
                "IGNRVAL" : self.ignrval,
                "CRVAL1" : self.crval1,
                "CRVAL2" : self.crval2,
                "CRVAL3" : self.crval3,
                "CDELT1" : self.cdelt1,
                "CDELT2" : self.cdelt2,
                "CDELT3" : self.cdelt3,
                "CRPIX1" : self.crpix1,
                "CRPIX2" : self.crpix2,
                "CRPIX3" : self.crpix3,
                "CUNIT1" : self.cunit1,
                "CUNIT2" : self.cunit2,
                "CUNIT3" : self.cunit3,
                "CTYPE1" : self.ctype1,
                "CTYPE2" : self.ctype2,
                "CTYPE3" : self.ctype3,
                "CD1_1" : self.cd1_1,
                "CD1_2" : self.cd1_2,
                "CD2_1" : self.cd2_1,
                "CD2_2" : self.cd2_2,
                "BMAJ" : self.bmaj,
                "BMIN" : self.bmin,
                "BPA" : self.bpa,
                "BUNIT" : self.beam_unit,
                "BTYPE" : self.beam_type,
                "SPECSYS" : self.specsys,
                "RESTFRQ" : self.restfrq,
                "OBSRA" : self.obsra,
                "OBSDEC" : self.obsdec,
                "OBJECT" : self.obj_name,
                "DATEOBS" : self.obs_date,
                "TIMESYS" : self.timesys,
                "LINE" : self.line,
                "FILTER" : self.filter,
                "mean_spectrum" : &self.mean_spectrum,
                "integrated_spectrum" : &self.integrated_spectrum,
                /* the histogram part, pixel min, max etc... */
                "min" : self.pmin,
                "max" : self.pmax,
                "median" : self.median,
                "sensitivity" : self.sensitivity,
                "ratio_sensitivity" : self.ratio_sensitivity,
                "black" : self.black,
                "white" : self.white,
                "flux" : self.flux,
                "histogram" : &self.hist,
                "is_optical" : &self.is_optical,
                "is_xray" : &self.is_xray,
        });

        value.to_string()
    }

    pub fn get_cutout_data(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        frame_start: f64,
        frame_end: f64,
        ref_freq: f64,
    ) -> Option<Vec<u8>> {
        //spatial range checks
        let x1 = num::clamp(x1, 0, self.width as i32 - 1);
        let y1 = num::clamp(y1, 0, self.height as i32 - 1);

        let x2 = num::clamp(x2, 0, self.width as i32 - 1);
        let y2 = num::clamp(y2, 0, self.height as i32 - 1);

        let (start, end) = match self.get_spectrum_range(frame_start, frame_end, ref_freq) {
            Some((start, end)) => {
                println!("[fits.get_region] start:{} end:{}", start, end);
                (start, end)
            }
            None => {
                println!("error: an invalid spectrum range");
                return None;
            }
        };

        let partial_width = (x2 - x1).abs() as usize;
        let partial_height = (y2 - y1).abs() as usize;
        let partial_depth = end - start + 1;

        let partial_data_size =
            partial_height * partial_width * partial_depth * ((self.bitpix.abs() / 8) as usize);
        let mut no_units = partial_data_size / FITS_CHUNK_LENGTH;

        if partial_data_size % FITS_CHUNK_LENGTH > 0 {
            no_units += 1;
        }

        let partial_capacity = self.header.len() + no_units * FITS_CHUNK_LENGTH;
        let mut partial_fits = Vec::with_capacity(partial_capacity);

        let naxes = [partial_width, partial_height, partial_depth, 1];

        //open the original FITS file
        let filename = format!("{}/{}.fits", FITSCACHE, self.dataset_id.replace("/", "_"));
        let filepath = std::path::Path::new(&filename);

        let mut f = match File::open(filepath) {
            Ok(x) => x,
            Err(x) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, x);

                return None;
            }
        };

        //CANNOT SEEK A COMPRESSED FILE!!!
        //check if a file is compressed (try it with GzDecode)
        let is_compressed = {
            let gunzip = GzDecoder::new(&f);

            match gunzip.header() {
                Some(_) => true,
                None => false,
            }
        };

        if is_compressed {
            println!(
                "seek() is not available for a compressed file, aborting a partial FITS cut-out"
            );
            return None;
        }

        //reset the file
        if let Err(err) = f.seek(SeekFrom::Start(0)) {
            println!("CRITICAL ERROR seeking within the FITS file: {}", err);
            return None;
        }

        let mut header_end: bool = false;

        while !header_end {
            //read a FITS chunk
            let mut chunk = [0; FITS_CHUNK_LENGTH];

            match f.read_exact(&mut chunk) {
                Ok(()) => {
                    //parse and modify a FITS header chunk
                    header_end = self.modify_partial_fits_header_chunk(
                        &mut chunk,
                        &naxes,
                        x1 as f64,
                        y1 as f64,
                        start as f64,
                    );
                    partial_fits.extend_from_slice(&chunk);
                }
                Err(err) => {
                    println!("CRITICAL ERROR reading FITS header: {}", err);
                    return None;
                }
            };
        }

        let frame_size = self.width * self.height * ((self.bitpix.abs() / 8) as usize);
        let header_size = self.header.len();

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for frame in start..end + 1 {
                let offset = header_size + frame * frame_size;

                if let Err(err) = f.seek(SeekFrom::Start(offset as u64)) {
                    println!("CRITICAL ERROR seeking within the FITS file: {}", err);
                    return;
                }

                //f now points to the start of a correct frame
                //read a chunk of <frame_size> bytes
                let mut data: Vec<u8> = vec![0; frame_size];

                //read a FITS cube frame
                match f.read_exact(&mut data) {
                    Ok(()) => {
                        match tx.send(data) {
                            Ok(()) => {}
                            Err(err) => {
                                println!("file reading thread: {}", err);
                                return;
                            }
                        };
                    }
                    Err(err) => {
                        println!("CRITICAL ERROR reading FITS data: {}", err);
                        return;
                    }
                }
            }
        });

        let mut frame: usize = 0;

        for data in rx {
            frame = frame + 1;

            //println!("read {}/{} FITS cube frames", frame, partial_depth);

            for y in y1..y2 {
                let src_offset = ((y as usize) * self.width + (x1 as usize))
                    * ((self.bitpix.abs() / 8) as usize);

                let partial_row_size = partial_width * ((self.bitpix.abs() / 8) as usize);

                partial_fits.extend_from_slice(&data[src_offset..src_offset + partial_row_size]);
            }
        }

        if frame != partial_depth {
            println!(
                "CRITICAL ERROR not all FITS cube frames have been read: {}/{}",
                frame, partial_depth
            );
            return None;
        }

        println!(
            "FITS cut-out length: {}, capacity: {}",
            partial_fits.len(),
            partial_capacity
        );

        let padding = partial_capacity - partial_fits.len();

        //pad the FITS cut-out to the nearest FITS_CHUNK_LENGTH
        if padding > 0 {
            partial_fits.extend_from_slice(&vec![0; padding]);

            println!(
                "padded FITS cut-out length: {}, capacity: {}",
                partial_fits.len(),
                partial_capacity
            );
        }

        Some(partial_fits)
    }

    pub fn get_cutout_stream(
        &self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        frame_start: f64,
        frame_end: f64,
        ref_freq: f64,
    ) -> Option<mpsc::Receiver<Vec<u8>>> {
        //spatial range checks
        let x1 = num::clamp(x1, 0, self.width as i32 - 1);
        let y1 = num::clamp(y1, 0, self.height as i32 - 1);

        let x2 = num::clamp(x2, 0, self.width as i32 - 1);
        let y2 = num::clamp(y2, 0, self.height as i32 - 1);

        let (start, end) = match self.get_spectrum_range(frame_start, frame_end, ref_freq) {
            Some((start, end)) => {
                println!("[fits.get_region] start:{} end:{}", start, end);
                (start, end)
            }
            None => {
                println!("error: an invalid spectrum range");
                return None;
            }
        };

        let (stream_tx, stream_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel();

        let partial_width = (x2 - x1).abs() as usize;
        let partial_height = (y2 - y1).abs() as usize;
        let partial_depth = end - start + 1;

        let partial_data_size =
            partial_height * partial_width * partial_depth * ((self.bitpix.abs() / 8) as usize);
        let mut no_units = partial_data_size / FITS_CHUNK_LENGTH;

        if partial_data_size % FITS_CHUNK_LENGTH > 0 {
            no_units += 1;
        }

        let partial_capacity = self.header.len() + no_units * FITS_CHUNK_LENGTH;
        let mut partial_size = 0;

        let naxes = [partial_width, partial_height, partial_depth, 1];

        //open the original FITS file
        let filename = format!("{}/{}.fits", FITSCACHE, self.dataset_id.replace("/", "_"));
        let filepath = std::path::Path::new(&filename);

        let mut f = match File::open(filepath) {
            Ok(x) => x,
            Err(x) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, x);

                return None;
            }
        };

        //CANNOT SEEK A COMPRESSED FILE!!!
        //check if a file is compressed (try it with GzDecode)
        let is_compressed = {
            let gunzip = GzDecoder::new(&f);

            match gunzip.header() {
                Some(_) => true,
                None => false,
            }
        };

        if is_compressed {
            println!(
                "seek() is not available for a compressed file, aborting a partial FITS cut-out"
            );
            return None;
        }

        //reset the file
        if let Err(err) = f.seek(SeekFrom::Start(0)) {
            println!("CRITICAL ERROR seeking within the FITS file: {}", err);
            return None;
        }

        let mut header_end: bool = false;

        while !header_end {
            //read a FITS chunk
            let mut chunk = [0; FITS_CHUNK_LENGTH];

            match f.read_exact(&mut chunk) {
                Ok(()) => {
                    //parse and modify a FITS header chunk
                    header_end = self.modify_partial_fits_header_chunk(
                        &mut chunk,
                        &naxes,
                        x1 as f64,
                        y1 as f64,
                        start as f64,
                    );

                    partial_size += chunk.len();

                    let stream = stream_tx.clone();
                    match stream.send(chunk.to_vec()) {
                        Ok(()) => {}
                        Err(err) => {
                            println!("CRITICAL ERROR sending partial_fits: {}", err);
                            return None;
                        }
                    }
                }
                Err(err) => {
                    println!("CRITICAL ERROR reading FITS header: {}", err);
                    return None;
                }
            };
        }

        let frame_size = self.width * self.height * ((self.bitpix.abs() / 8) as usize);
        let header_size = self.header.len();

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for frame in start..end + 1 {
                let offset = header_size + frame * frame_size;

                if let Err(err) = f.seek(SeekFrom::Start(offset as u64)) {
                    println!("CRITICAL ERROR seeking within the FITS file: {}", err);
                    return;
                }

                //f now points to the start of a correct frame
                //read a chunk of <frame_size> bytes
                let mut data: Vec<u8> = vec![0; frame_size];

                //read a FITS cube frame
                match f.read_exact(&mut data) {
                    Ok(()) => {
                        match tx.send(data) {
                            Ok(()) => {}
                            Err(err) => {
                                println!("file reading thread: {}", err);
                                return;
                            }
                        };
                    }
                    Err(err) => {
                        println!("CRITICAL ERROR reading FITS data: {}", err);
                        return;
                    }
                }
            }
        });

        let fits_width = self.width;
        let fits_bitpix = self.bitpix;

        thread::spawn(move || {
            let mut frame: usize = 0;

            for data in rx {
                frame = frame + 1;

                //println!("read {}/{} FITS cube frames", frame, partial_depth);

                for y in y1..y2 {
                    let src_offset = ((y as usize) * fits_width + (x1 as usize))
                        * ((fits_bitpix.abs() / 8) as usize);

                    let partial_row_size = partial_width * ((fits_bitpix.abs() / 8) as usize);

                    let slice = &data[src_offset..src_offset + partial_row_size];
                    partial_size += slice.len();

                    let stream = stream_tx.clone();
                    match stream.send(slice.to_vec()) {
                        Ok(()) => {}
                        Err(err) => {
                            println!("CRITICAL ERROR sending partial_fits: {}", err);
                            return;
                        }
                    }
                }
            }

            if frame != partial_depth {
                println!(
                    "CRITICAL ERROR not all FITS cube frames have been read: {}/{}",
                    frame, partial_depth
                );
                return;
            }

            println!(
                "FITS cut-out length: {}, capacity: {}",
                partial_size, partial_capacity
            );

            let padding = partial_capacity - partial_size;

            //pad the FITS cut-out to the nearest FITS_CHUNK_LENGTH
            if padding > 0 {
                let slice = &vec![0; padding];
                partial_size += slice.len();

                let stream = stream_tx.clone();
                match stream.send(slice.to_vec()) {
                    Ok(()) => {}
                    Err(err) => {
                        println!("CRITICAL ERROR sending partial_fits: {}", err);
                        return;
                    }
                }

                println!(
                    "padded FITS cut-out length: {}, capacity: {}",
                    partial_size, partial_capacity
                );
            }
        });

        Some(stream_rx)
    }

    pub fn get_full_stream(&self) -> Option<mpsc::Receiver<Vec<u8>>> {
        let (stream_tx, stream_rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) =
            mpsc::channel();

        //open the original FITS file
        let filename = format!("{}/{}.fits", FITSCACHE, self.dataset_id.replace("/", "_"));
        let filepath = std::path::Path::new(&filename);

        let mut f = match File::open(filepath) {
            Ok(x) => x,
            Err(x) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, x);

                return None;
            }
        };

        //reset the file
        if let Err(err) = f.seek(SeekFrom::Start(0)) {
            println!("CRITICAL ERROR seeking within the FITS file: {}", err);
            return None;
        }

        // read the whole file in chunks
        thread::spawn(move || {
            let stream = stream_tx.clone();

            // make a 256KB buffer
            let mut buffer = [0; 262144];

            loop {
                let bytes_read = match f.read(&mut buffer) {
                    // read up to 256KB
                    Ok(0) => return, // reached EOF
                    Ok(n) => n,
                    Err(_) => return, // ignore errors
                };

                let slice = &buffer[0..bytes_read];
                match stream.send(slice.to_vec()) {
                    Ok(()) => {}
                    Err(err) => {
                        println!("CRITICAL ERROR STREAMING FULL FITS: {}", err);
                        return;
                    }
                }
            }
        });

        Some(stream_rx)
    }

    #[cfg(feature = "zfp")]
    fn zfp_compress(&self) -> bool {
        #[cfg(not(feature = "raid"))]
        {
            let filename = format!("{}/{}.zfp", FITSCACHE, self.dataset_id.replace("/", "_"));
            let zfp_dir = std::path::Path::new(&filename);

            //check if the zfp directory already exists in the FITSCACHE
            if !zfp_dir.exists() {
                match std::fs::create_dir(zfp_dir) {
                    Ok(_) => {
                        println!("{}: created an empty zfp cache directory", self.dataset_id);
                    }
                    Err(err) => {
                        println!("error creating a zfp cache directory: {}", err);
                        return false;
                    }
                }
            }

            //look for a hidden ok file
            let mut ok_file = std::path::PathBuf::from(zfp_dir);
            ok_file.push(".ok");

            if ok_file.exists() {
                return true;
            }
        }

        #[cfg(feature = "raid")]
        {
            let mut ok_exists = true;

            // iterate through all RAID-0 volumes checking / creating cache directories
            for raid_volume in 0..RAID_COUNT {
                let filename = format!(
                    "{}{}/{}/{}.zfp",
                    RAID_PREFIX,
                    raid_volume,
                    FITSCACHE,
                    self.dataset_id.replace("/", "_")
                );
                let zfp_dir = std::path::Path::new(&filename);

                //check if the zfp directory already exists in the FITSCACHE
                if !zfp_dir.exists() {
                    match std::fs::create_dir(zfp_dir) {
                        Ok(_) => {
                            println!(
                                "{}: created an empty zfp cache directory in a RAID-0 subvolume {}",
                                self.dataset_id, raid_volume
                            );
                        }
                        Err(err) => {
                            println!("error creating a zfp cache directory: {}", err);
                            return false;
                        }
                    }
                }

                //look for a hidden ok file
                let mut ok_file = std::path::PathBuf::from(zfp_dir);
                ok_file.push(".ok");

                ok_exists = ok_exists && ok_file.exists();
            }

            // cache files already exist in RAID-0 volumes
            if ok_exists {
                return true;
            }
        }

        println!(
            "{}: writing zfp-compressed half-float f16 data to cache",
            self.dataset_id
        );

        //use a sequential loop with OpenMP zfp_compress execution policy
        //so as not to overload the server
        let success = (0..self.depth)
            .into_iter() //into_par_iter or into_iter
            .map(|frame| {
                let vec = &self.data_f16[frame];
                let len = vec.len();
                let mut res = true;

                let mut array: Vec<f32> = Vec::with_capacity(len);
                let mut mask: Vec<u8> = Vec::with_capacity(len);

                let frame_min = self.frame_min[frame];
                let frame_max = self.frame_max[frame];

                for x in vec.iter() {
                    let tmp = self.bzero + self.bscale * x.to_f32(); //convert from half to f32

                    if tmp.is_finite()
                        && tmp >= self.datamin
                        && tmp <= self.datamax
                        && tmp > self.ignrval
                    {
                        /*let pixel = 0.5_f32 + (tmp - frame_min) / (frame_max - frame_min);
                        array.push(pixel.ln());*/
                        array.push(tmp);
                        mask.push(255)
                    } else {
                        array.push(0.0);
                        mask.push(0);
                    }
                }

                //compress the data array

                /* allocate meta data for the 2D array a[self.width][self.height] */
                let data_type = zfp_type_zfp_type_float;
                let field = unsafe {
                    zfp_field_2d(
                        array.as_mut_ptr() as *mut std::ffi::c_void,
                        data_type,
                        self.width as usize,
                        self.height as usize,
                    )
                };

                /* allocate meta data for a compressed stream */
                let zfp = unsafe { zfp_stream_open(std::ptr::null_mut() as *mut bitstream) };

                /* set compression mode and parameters */
                let rate: f64 = 8.0;
                unsafe { zfp_stream_set_rate(zfp, rate, data_type, 2, 0) };
                /*let tolerance = 1.0e-3;
                unsafe { zfp_stream_set_accuracy(zfp, tolerance) };*/
                /*let precision = 11; //was 14; 10 is not enough, 11 bits is borderline
                unsafe { zfp_stream_set_precision(zfp, precision) };*/

                //use only half the number of CPUs in OpenMP
                /*#[cfg(not(feature = "cuda"))]
                {
                    let num_threads = num_cpus::get_physical() / 2).max(1);
                    //no need to call zfp_stream_set_execution(zfp, zfp_exec_policy_zfp_exec_omp)
                    unsafe { zfp_stream_set_omp_threads(zfp, num_threads as u32) };
                }*/

                #[cfg(feature = "cuda")]
                {
                    let ret =
                        unsafe { zfp_stream_set_execution(zfp, zfp_exec_policy_zfp_exec_cuda) };

                    if ret == 0 {
                        println!("failed to set the execution policy to zfp_exec_cuda");
                    }
                }

                /* allocate buffer for compressed data */
                let bufsize = unsafe { zfp_stream_maximum_size(zfp, field) };
                let mut buffer: Vec<u8> = vec![0; bufsize as usize];

                /* associate bit stream with allocated buffer */
                let stream =
                    unsafe { stream_open(buffer.as_mut_ptr() as *mut std::ffi::c_void, bufsize) };
                unsafe {
                    zfp_stream_set_bit_stream(zfp, stream);
                    zfp_stream_rewind(zfp);
                }

                /* compress array and output compressed stream */
                let zfpsize = unsafe { zfp_compress(zfp, field) };
                if zfpsize == 0 {
                    println!("compression failed");
                    res = false;
                }
                /*else {
                      let original_size = self.width * self.height * std::mem::size_of::<f32>();
                      let ratio = (original_size as f64) / (zfpsize as f64);

                      println!("plane {}, bufsize: {} bytes, original size: {} bytes, compressed size: {} bytes, ratio: {}", frame, bufsize, original_size, zfpsize, ratio);
                }*/

                /* clean up */
                unsafe {
                    zfp_field_free(field);
                    zfp_stream_close(zfp);
                    stream_close(stream);
                }

                //upon success continue with the process, compress the mask
                if res {
                    let zfp_frame = ZFPMaskedArray {
                        array: buffer[0..zfpsize as usize].to_vec(),
                        mask: lz4_compress::compress(&mask),
                        frame_min: frame_min,
                        frame_max: frame_max,
                        //precision: precision,
                        rate: rate,
                    };

                    #[cfg(feature = "raid")]
                    let raid_volume = frame % RAID_COUNT;

                    #[cfg(feature = "raid")]
                    let filename = format!(
                        "{}{}/{}/{}.zfp",
                        RAID_PREFIX,
                        raid_volume,
                        FITSCACHE,
                        self.dataset_id.replace("/", "_")
                    );

                    #[cfg(not(feature = "raid"))]
                    let filename =
                        format!("{}/{}.zfp", FITSCACHE, self.dataset_id.replace("/", "_"));

                    let zfp_dir = std::path::Path::new(&filename);

                    let mut cache_file = std::path::PathBuf::from(zfp_dir);
                    cache_file.push(format!("{}.bin", frame));

                    match File::create(cache_file) {
                        Ok(f) => {
                            let mut buffer = std::io::BufWriter::new(f);
                            match encode_into_std_write(&zfp_frame, &mut buffer, config::legacy()) {
                                Ok(_) => {
                                    //flush the buffer, check for any errors
                                    match buffer.into_inner() {
                                        Ok(_) => {}
                                        Err(err) => {
                                            println!("error flushing a zfp stream: {}", err);
                                            res = false;
                                        }
                                    };
                                }
                                Err(err) => {
                                    println!("error serializing a zfp stream: {}", err);
                                    res = false;
                                }
                            }
                        }
                        Err(err) => {
                            println!("{}", err);
                            res = false;
                        }
                    };
                }

                res
            })
            //.reduce(|| true, |acc, res| acc && res);
            .fold(true, |acc, res| acc && res);

        println!("{}: zfp compression success: {}", self.dataset_id, success);

        //create a hidden ok file upon success
        if success {
            #[cfg(not(feature = "raid"))]
            {
                let filename = format!("{}/{}.zfp", FITSCACHE, self.dataset_id.replace("/", "_"));
                let zfp_dir = std::path::Path::new(&filename);

                let mut ok_file = std::path::PathBuf::from(zfp_dir);
                ok_file.push(".ok");

                match File::create(ok_file) {
                    Ok(_) => {}
                    Err(err) => println!("{}", err),
                }
            }

            #[cfg(feature = "raid")]
            {
                for raid_volume in 0..RAID_COUNT {
                    let filename = format!(
                        "{}{}/{}/{}.zfp",
                        RAID_PREFIX,
                        raid_volume,
                        FITSCACHE,
                        self.dataset_id.replace("/", "_")
                    );
                    let zfp_dir = std::path::Path::new(&filename);

                    let mut ok_file = std::path::PathBuf::from(zfp_dir);
                    ok_file.push(".ok");

                    match File::create(ok_file) {
                        Ok(_) => {}
                        Err(err) => println!("{}", err),
                    }
                }
            }
        }

        success
    }

    #[cfg(feature = "opencl")]
    fn rbf_compress(&self) {
        //check if the RBF file already exists in the FITSCACHE
        let filename = format!("{}/{}.rbf", FITSCACHE, self.dataset_id.replace("/", "_"));
        let filepath = std::path::Path::new(&filename);

        if filepath.exists() {
            return;
        }

        println!(
            "{}: compressing a FITS data cube with Radial Basis Functions",
            self.dataset_id
        );

        let XCLUST = self.width / 16;
        let YCLUST = self.height / 16;
        let NCLUST = XCLUST * YCLUST;

        println!(
            "plane dimensions: {}x{}, RBF clusters: {}x{}, total = {}",
            self.width, self.height, XCLUST, YCLUST, NCLUST
        );

        //define NCLUST dynamically
        let mut ocl_src = format!("#define NCLUST {}", NCLUST);

        ocl_src.push_str(r#"
            inline void atomicAdd_l_f(volatile __local float *addr, float val)
            {
                union {
                    unsigned int u32;
                    float        f32;
                } next, expected, current;
   	            current.f32    = *addr;
                do {
   	                expected.f32 = current.f32;
                    next.f32     = expected.f32 + val;
   		            current.u32  = atomic_cmpxchg( (volatile __local unsigned int *)addr, 
                               expected.u32, next.u32);
                } while( current.u32 != expected.u32 );
            }

            inline void atomicAdd_g_f(volatile __global float *addr, float val)
            {
                union {
                    unsigned int u32;
                    float        f32;
                } next, expected, current;
   	            current.f32    = *addr;
                do {
   	                expected.f32 = current.f32;
                    next.f32     = expected.f32 + val;
   		            current.u32  = atomic_cmpxchg( (volatile __global unsigned int *)addr, 
                               expected.u32, next.u32);
                } while( current.u32 != expected.u32 );
            }

            __kernel void rbf_forward_pass(__global float* _x1, __global float* _x2, __global float* _y, __global float* _data, __global float* _e, __constant float* c1, __constant float* c2, __constant float* p0, __constant float* p1, __constant float* p2, __constant float* w, __global float* _grad_w) {              

                __local float tid_grad_w[NCLUST+1];
                float grad_w[NCLUST+1];

                size_t local_index = get_local_id(0);

                if(local_index == 0)
                {
                    for(int i=0;i<NCLUST+1;i++)
                        tid_grad_w[i]=0.0;
                };
                barrier(CLK_LOCAL_MEM_FENCE);

                size_t index = get_global_id(0);

                float x1 = _x1[index];
                float x2 = _x2[index];

                float tmp = w[NCLUST];
                grad_w[NCLUST] = 1.0;//bias

                for(int i=0;i<NCLUST;i++)
                {
                    float a = native_exp(p0[i]) ;
		            float b = p1[i] ;
		            float c = native_exp(p2[i]) ;

		            float tmp1 = (x1 - c1[i]) ;
		            float tmp2 = (x2 - c2[i]) ;
		            float dist = a*tmp1*tmp1 - 2.0f*b*tmp1*tmp2 + c*tmp2*tmp2 ;
		            float act = native_exp( - dist ) ;

		            tmp += w[i] * act ;
                    grad_w[i] = act ;
                }

                float e = tmp - _data[index] ;
                _y[index] = tmp ;                
	            _e[index] = e ;

                //gradients
	            for(int i=0;i<NCLUST+1;i++)
		            atomicAdd_l_f(&(tid_grad_w[i]), e * grad_w[i]) ;

                barrier(CLK_LOCAL_MEM_FENCE);
                if(local_index == (get_local_size(0)-1))
                {
                    for(int i=0;i<NCLUST+1;i++)
                        atomicAdd_g_f(&(_grad_w[i]), tid_grad_w[i]);
                }
            }
        "#);

        for frame in 0..1
        /*self.depth*/
        {
            //prepare the training data
            let capacity = self.width * self.height;
            let mut data: Vec<f32> = Vec::with_capacity(capacity);
            let mut x1: Vec<f32> = Vec::with_capacity(capacity);
            let mut x2: Vec<f32> = Vec::with_capacity(capacity);
            let mut y: Vec<f32> = Vec::with_capacity(capacity);
            let mut e: Vec<f32> = Vec::with_capacity(capacity);

            let frame_min = self.frame_min[frame];
            let frame_max = self.frame_max[frame];

            let mut x1min = std::f32::MAX;
            let mut x1max = std::f32::MIN;
            let mut x2min = std::f32::MAX;
            let mut x2max = std::f32::MIN;

            let mut offset: usize = 0;

            //log-transform the data
            match self.bitpix {
                -32 => {
                    let vec = &self.data_f16[frame];

                    for iy in 0..self.height {
                        for ix in 0..self.width {
                            let tmp = self.bzero + self.bscale * vec[offset].to_f32(); //convert from half to f32
                            offset = offset + 1;

                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                let pixel = 0.5_f32 + (tmp - frame_min) / (frame_max - frame_min);
                                data.push(pixel.ln());

                                let val1 = (ix as f32) / ((self.width - 1) as f32);
                                let val2 = (iy as f32) / ((self.height - 1) as f32);

                                if val1 < x1min {
                                    x1min = val1;
                                }

                                if val1 > x1max {
                                    x1max = val1;
                                }

                                if val2 < x2min {
                                    x2min = val2;
                                }

                                if val2 > x2max {
                                    x2max = val2;
                                }

                                x1.push(val1);
                                x2.push(val2);
                                y.push(0.0);
                                e.push(0.0);
                            }
                        }
                    }
                }
                _ => println!("unsupported bitpix: {}", self.bitpix),
            };

            println!(
                "frame {}, min: {}, max: {}, capacity: {}, length: {}/{}/{}/{}/{}",
                frame,
                frame_min,
                frame_max,
                capacity,
                data.len(),
                x1.len(),
                x2.len(),
                y.len(),
                e.len()
            );

            //init RBF clusters
            println!(
                "x1min: {}, x1max: {}, x2min: {}, x2max: {}",
                x1min, x1max, x2min, x2max
            );

            let mut c1: Vec<f32> = Vec::with_capacity(NCLUST);
            let mut c2: Vec<f32> = Vec::with_capacity(NCLUST);
            let mut p0: Vec<f32> = Vec::with_capacity(NCLUST);
            let mut p1: Vec<f32> = Vec::with_capacity(NCLUST);
            let mut p2: Vec<f32> = Vec::with_capacity(NCLUST);

            let mut rng = rand::thread_rng();
            let angle = Uniform::new(0.0f32, 2.0 * std::f32::consts::PI);

            for i in 0..XCLUST {
                for j in 0..YCLUST {
                    let i = (i as f32) / ((XCLUST - 1) as f32);
                    let j = (j as f32) / ((YCLUST - 1) as f32);

                    c1.push(i * (x1max - x1min));
                    c2.push(j * (x2max - x2min));

                    let sigmaX = 0.1f32 / ((XCLUST - 1) as f32);
                    let sigmaY = 0.1f32 / ((YCLUST - 1) as f32);
                    let theta = angle.sample(&mut rng);

                    let a = 0.5 * theta.cos() * theta.cos() / (sigmaX * sigmaX)
                        + 0.5 * theta.sin() * theta.sin() / (sigmaY * sigmaY);
                    let b = -0.25 * (2.0 * theta).sin() / (sigmaX * sigmaX)
                        + 0.25 * (2.0 * theta).sin() / (sigmaY * sigmaY);
                    let c = 0.5 * theta.sin() * theta.sin() / (sigmaX * sigmaX)
                        + 0.5 * theta.cos() * theta.cos() / (sigmaY * sigmaY);

                    p0.push(a.ln());
                    p1.push(b);
                    p2.push(c.ln());
                }
            }

            let normal = StandardNormal;
            //no. clusters plus a bias term
            let mut w: Vec<f32> = (0..NCLUST + 1)
                .map(|_| normal.sample(&mut rng) as f32)
                .collect();

            //gradients
            let mut grad_c1: Vec<f32> = vec![0.0; NCLUST];
            let mut grad_c2: Vec<f32> = vec![0.0; NCLUST];
            let mut grad_p0: Vec<f32> = vec![0.0; NCLUST];
            let mut grad_p1: Vec<f32> = vec![0.0; NCLUST];
            let mut grad_p2: Vec<f32> = vec![0.0; NCLUST];
            let mut grad_w: Vec<f32> = vec![0.0; NCLUST + 1];

            //init OpenCL buffers
            let len = data.len();
            let pro_que = ProQue::builder()
                //.cmplr_def("NCLUST", NCLUST)
                .src(ocl_src.clone())
                .dims(len)
                .build()
                .unwrap();
            /*let mut pro_que = ProQue::builder().src(ocl_src).build().unwrap();
            pro_que.set_dims(len);*/

            let ocl_data = pro_que.create_buffer::<f32>().unwrap();
            ocl_data.write(&data).enq().unwrap();

            let ocl_x1 = pro_que.create_buffer::<f32>().unwrap();
            ocl_x1.write(&x1).enq().unwrap();

            let ocl_x2 = pro_que.create_buffer::<f32>().unwrap();
            ocl_x2.write(&x2).enq().unwrap();

            let ocl_y = pro_que.create_buffer::<f32>().unwrap();
            let ocl_e = pro_que.create_buffer::<f32>().unwrap();

            let ocl_c1 = pro_que.buffer_builder::<f32>().len(NCLUST).build().unwrap();
            ocl_c1.write(&c1).enq().unwrap();

            let ocl_c2 = pro_que.buffer_builder::<f32>().len(NCLUST).build().unwrap();
            ocl_c2.write(&c2).enq().unwrap();

            let ocl_p0 = pro_que.buffer_builder::<f32>().len(NCLUST).build().unwrap();
            ocl_p0.write(&p0).enq().unwrap();

            let ocl_p1 = pro_que.buffer_builder::<f32>().len(NCLUST).build().unwrap();
            ocl_p1.write(&p1).enq().unwrap();

            let ocl_p2 = pro_que.buffer_builder::<f32>().len(NCLUST).build().unwrap();
            ocl_p2.write(&p2).enq().unwrap();

            let ocl_w = pro_que
                .buffer_builder::<f32>()
                .len(NCLUST + 1)
                .build()
                .unwrap();
            ocl_w.write(&w).enq().unwrap();

            let ocl_grad_w = pro_que
                .buffer_builder::<f32>()
                .len(NCLUST + 1)
                .build()
                .unwrap();

            let kernel = pro_que
                .kernel_builder("rbf_forward_pass")
                .arg(&ocl_x1)
                .arg(&ocl_x2)
                .arg(&ocl_y)
                .arg(&ocl_data)
                .arg(&ocl_e)
                .arg(&ocl_c1)
                .arg(&ocl_c2)
                .arg(&ocl_p0)
                .arg(&ocl_p1)
                .arg(&ocl_p2)
                .arg(&ocl_w)
                .arg(&ocl_grad_w)
                //.arg(NCLUST as i32)
                //.arg_local::<f32>(NCLUST + 1)
                .build()
                .unwrap();

            //batch training
            let NITER = 1;

            for iter in 0..NITER {
                print!("{} ", iter);

                unsafe {
                    kernel.enq().unwrap();
                }

                ocl_y.read(&mut y).enq().unwrap();
                ocl_e.read(&mut e).enq().unwrap();
                ocl_grad_w.read(&mut grad_w).enq().unwrap();
            }

            println!("grad_w: {:?}", grad_w);
        }
    }

    pub fn drop_to_cache(&self) {
        if self.has_data {
            #[cfg(not(feature = "zfp"))]
            let write_to_zfp = false;

            #[cfg(feature = "zfp")]
            let write_to_zfp = {
                if self.bitpix == -32 && self.data_f16.len() > 0 && self.depth > 1 {
                    //skip compression for really small files
                    let total_size =
                        self.width * self.height * self.depth * ((self.bitpix.abs() / 8) as usize);

                    //>1MB
                    if total_size > 1024 * 1024 {
                        self.zfp_compress()
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if !write_to_zfp {
                if self.bitpix == -32 && self.data_f16.len() > 0 {
                    //check if the binary file already exists in the FITSCACHE
                    let filename =
                        format!("{}/{}.bin", FITSCACHE, self.dataset_id.replace("/", "_"));
                    let filepath = std::path::Path::new(&filename);

                    if !filepath.exists() {
                        println!("{}: writing half-float f16 data to cache", self.dataset_id);

                        let tmp_filename = format!(
                            "{}/{}.bin.tmp",
                            FITSCACHE,
                            self.dataset_id.replace("/", "_")
                        );
                        let tmp_filepath = std::path::Path::new(&tmp_filename);

                        let mut buffer = match File::create(tmp_filepath) {
                            Ok(f) => f,
                            Err(err) => {
                                println!("{}", err);
                                return;
                            }
                        };

                        for v16 in &self.data_f16 {
                            let ptr = v16.as_ptr() as *mut u8;
                            let len = v16.len();

                            unsafe {
                                let raw = slice::from_raw_parts(ptr, 2 * len);

                                match buffer.write_all(&raw) {
                                    Ok(()) => {}
                                    Err(err) => {
                                        println!(
                                            "binary cache write error: {}, removing the temporary file",
                                            err
                                        );

                                        let _ = std::fs::remove_file(tmp_filepath);

                                        return;
                                    }
                                };
                            };
                        }

                        let _ = std::fs::rename(tmp_filepath, filepath);
                    }
                }
            }

            #[cfg(feature = "opencl")]
            {
                if self.depth > 1 {
                    self.rbf_compress();
                }
            }
        }
    }
}

impl Clone for FITS {
    fn clone(&self) -> FITS {
        let mut fits = FITS::new(&self.dataset_id, &self.url, &self.flux);

        //only a limited clone (fields needed by get_frequency_range())

        fits.has_header = self.has_header;
        fits.has_frequency = self.has_frequency;
        fits.has_velocity = self.has_velocity;

        fits.crval3 = self.crval3;
        fits.cdelt3 = self.cdelt3;
        fits.crpix3 = self.crpix3;
        fits.cunit3 = self.cunit3.clone();
        fits.ctype3 = self.ctype3.clone();
        fits.frame_multiplier = self.frame_multiplier;

        fits.is_optical = self.is_optical;
        fits.is_xray = self.is_xray;
        fits.width = self.width;
        fits.height = self.height;
        fits.depth = self.depth;
        fits.polarisation = self.polarisation;

        fits
    }
}

impl Drop for FITS {
    fn drop(&mut self) {
        println!("deleting {}", self.dataset_id);
        self.drop_to_cache();

        if self.has_data {
            //remove a symbolic link
            let filename = format!("{}/{}.fits", FITSCACHE, self.dataset_id.replace("/", "_"));
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
    }
}

fn is_gzip_compressed(f: &mut File) -> bool {
    let mut header = [0; 10];
    match f.read_exact(&mut header) {
        Ok(()) => {
            //reset the file
            if let Err(err) = f.seek(SeekFrom::Start(0)) {
                println!("CRITICAL ERROR seeking within the FITS file: {}", err);
            };

            if header[0] == 0x1f && header[1] == 0x8b && header[2] == 0x08 {
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

fn is_bzip2_compressed(f: &mut File) -> bool {
    let mut header = [0; 4];
    match f.read_exact(&mut header) {
        Ok(()) => {
            //reset the file
            if let Err(err) = f.seek(SeekFrom::Start(0)) {
                println!("CRITICAL ERROR seeking within the FITS file: {}", err);
            };

            if header[0] == 0x42 && header[1] == 0x5a && header[2] == 0x68 {
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

fn logistic_regression_classifier(slot: &Vec<f64>) -> i32 {
    let legacy = 443.6170837772559
        - 0.008793892019758082 * slot[1 - 1]
        - 0.05060583821958265 * slot[2 - 1]
        - 0.07060929424871956 * slot[3 - 1]
        - 0.07458549722846479 * slot[4 - 1]
        - 0.07872086383369215 * slot[5 - 1]
        - 0.07838234461963924 * slot[6 - 1]
        - 0.07036569793244277 * slot[7 - 1]
        - 0.05817571617720529 * slot[8 - 1]
        - 0.0432668804603716 * slot[9 - 1]
        - 0.03451400986790043 * slot[10 - 1]
        - 0.028448750051392184 * slot[11 - 1]
        - 0.023848416061757093 * slot[12 - 1]
        - 0.02149025645261006 * slot[13 - 1]
        - 0.020785782492790963 * slot[14 - 1]
        - 0.021522770699430458 * slot[15 - 1]
        - 0.02308194255152165 * slot[16 - 1]
        - 0.028285162976156276 * slot[17 - 1]
        - 0.029766367267853214 * slot[18 - 1]
        - 0.030813480444413486 * slot[19 - 1]
        - 0.03169508313725471 * slot[20 - 1]
        - 0.03246788218945292 * slot[21 - 1]
        - 0.033461508405307504 * slot[22 - 1]
        - 0.03680829162885007 * slot[23 - 1]
        - 0.03844555030418408 * slot[24 - 1]
        - 0.03958187332073853 * slot[25 - 1]
        - 0.040645927662603824 * slot[26 - 1]
        - 0.04387228450213452 * slot[27 - 1]
        - 0.0448303571058308 * slot[28 - 1]
        - 0.04579970542092778 * slot[29 - 1]
        - 0.04681403768373626 * slot[30 - 1]
        - 0.04794528106118201 * slot[31 - 1]
        - 0.048997896610131236 * slot[32 - 1]
        - 0.0500346695456061 * slot[33 - 1]
        - 0.05101105444768723 * slot[34 - 1]
        - 0.05205773779717132 * slot[35 - 1]
        - 0.053323697703242356 * slot[36 - 1]
        - 0.0545756611497103 * slot[37 - 1]
        - 0.05759138423279407 * slot[38 - 1]
        - 0.058773811284816756 * slot[39 - 1]
        - 0.05959043219992909 * slot[40 - 1]
        - 0.06021756246932851 * slot[41 - 1]
        - 0.06053584049969081 * slot[42 - 1]
        - 0.05946864884726697 * slot[43 - 1]
        - 0.058017806161462064 * slot[44 - 1]
        - 0.05649042170826382 * slot[45 - 1]
        - 0.05487187762107259 * slot[46 - 1]
        - 0.05323837344000212 * slot[47 - 1]
        - 0.03172480272464783 * slot[48 - 1]
        - 0.03012374668992967 * slot[49 - 1]
        - 0.02868679296520774 * slot[50 - 1]
        - 0.027536968182540023 * slot[51 - 1]
        - 0.026451768722341187 * slot[52 - 1]
        - 0.025633828181934704 * slot[53 - 1]
        - 0.030772974646868397 * slot[54 - 1]
        - 0.03022627306369872 * slot[55 - 1]
        - 0.02977046716738917 * slot[56 - 1]
        - 0.029394409350553447 * slot[57 - 1]
        - 0.029120021564382308 * slot[58 - 1]
        - 0.028860121764409774 * slot[59 - 1]
        - 0.028564539818413802 * slot[60 - 1]
        - 0.04151173003384049 * slot[61 - 1]
        - 0.04135331115372854 * slot[62 - 1]
        - 0.04123852147328032 * slot[63 - 1]
        - 0.04115917899163619 * slot[64 - 1]
        - 0.04096367765617342 * slot[65 - 1]
        - 0.040706092146348384 * slot[66 - 1]
        - 0.040519863947365495 * slot[67 - 1]
        - 0.04021055390260678 * slot[68 - 1]
        - 0.039800451780115376 * slot[69 - 1]
        - 0.03939821994161991 * slot[70 - 1]
        - 0.03885113255806612 * slot[71 - 1]
        - 0.03830351478890804 * slot[72 - 1]
        - 0.022363463780130245 * slot[73 - 1]
        - 0.021817323388618724 * slot[74 - 1]
        - 0.02114118726934881 * slot[75 - 1]
        - 0.020543384948290146 * slot[76 - 1]
        - 0.019892296007862006 * slot[77 - 1]
        - 0.01914921056549216 * slot[78 - 1]
        - 0.01828034751168774 * slot[79 - 1]
        - 0.017360874782837148 * slot[80 - 1]
        - 0.016217902907801743 * slot[81 - 1]
        - 0.014847060215524044 * slot[82 - 1]
        - 0.013390129429479342 * slot[83 - 1]
        - 0.011876801304111407 * slot[84 - 1]
        - 0.010140738345286676 * slot[85 - 1]
        - 0.00846906007191739 * slot[86 - 1]
        - 0.006862820232582459 * slot[87 - 1]
        - 0.005308650169401543 * slot[88 - 1]
        - 0.0038472283059190608 * slot[89 - 1]
        - 0.0024194873256139587 * slot[90 - 1]
        - 0.0010992844263096931 * slot[91 - 1]
        + 0.00005389604430364895 * slot[92 - 1]
        + 0.0014110915510820398 * slot[93 - 1]
        + 0.004179273981723788 * slot[94 - 1]
        + 0.006635849765220059 * slot[95 - 1]
        + 0.013958486425786704 * slot[96 - 1]
        + 0.015015881191807887 * slot[97 - 1]
        + 0.015950669792735774 * slot[98 - 1]
        + 0.01678380444351064 * slot[99 - 1]
        + 0.017576626787672592 * slot[100 - 1]
        + 0.018290499784349042 * slot[101 - 1]
        + 0.019040803148975792 * slot[102 - 1]
        + 0.019841379210530318 * slot[103 - 1]
        + 0.02055086992083401 * slot[104 - 1]
        + 0.021344588583473496 * slot[105 - 1]
        + 0.02207955765174099 * slot[106 - 1]
        - 0.024205066065540824 * slot[107 - 1]
        - 0.023393220046100855 * slot[108 - 1]
        - 0.022601411257901077 * slot[109 - 1]
        - 0.021855813944655262 * slot[110 - 1]
        - 0.02114844314883402 * slot[111 - 1]
        - 0.020434960181834494 * slot[112 - 1]
        - 0.01973327824885937 * slot[113 - 1]
        - 0.018936455524012314 * slot[114 - 1]
        - 0.018157024181075596 * slot[115 - 1]
        - 0.0174129834032178 * slot[116 - 1]
        - 0.01680849955470401 * slot[117 - 1]
        - 0.016211926474499996 * slot[118 - 1]
        - 0.015608554929332849 * slot[119 - 1]
        - 0.01500710762545931 * slot[120 - 1]
        - 0.014446820792379985 * slot[121 - 1]
        - 0.013907437050889721 * slot[122 - 1]
        - 0.013370641413709156 * slot[123 - 1]
        - 0.012793933584261796 * slot[124 - 1]
        - 0.012273962385522088 * slot[125 - 1]
        - 0.011799725976415569 * slot[126 - 1]
        - 0.01133338561369326 * slot[127 - 1]
        - 0.010929600569301909 * slot[128 - 1]
        - 0.01051669039425422 * slot[129 - 1]
        - 0.010029195791935668 * slot[130 - 1]
        - 0.00967522380244091 * slot[131 - 1]
        - 0.009246602259858133 * slot[132 - 1]
        - 0.008883502245755646 * slot[133 - 1]
        - 0.008494331282444692 * slot[134 - 1]
        - 0.008236482150351 * slot[135 - 1]
        - 0.007844102941266098 * slot[136 - 1]
        - 0.0020638338892650194 * slot[137 - 1]
        - 0.001759582415899818 * slot[138 - 1]
        - 0.0014836208837400071 * slot[139 - 1]
        - 0.001145492449522322 * slot[140 - 1]
        + 0.005898238323622205 * slot[141 - 1]
        + 0.006282081115722431 * slot[142 - 1]
        + 0.0065798673732572295 * slot[143 - 1]
        + 0.006793752339693942 * slot[144 - 1]
        + 0.006950686156881262 * slot[145 - 1]
        + 0.0071229230965212074 * slot[146 - 1]
        + 0.007280230722201209 * slot[147 - 1]
        + 0.0073759539295827035 * slot[148 - 1]
        + 0.007402477622496995 * slot[149 - 1]
        + 0.007470433981802484 * slot[150 - 1]
        + 0.007466672066791723 * slot[151 - 1]
        + 0.007478706860143925 * slot[152 - 1]
        + 0.007477977954886849 * slot[153 - 1]
        + 0.007384452557667273 * slot[154 - 1]
        + 0.00734744563375555 * slot[155 - 1]
        + 0.00722699976733004 * slot[156 - 1]
        + 0.0071769869414632925 * slot[157 - 1]
        - 0.018538378131074937 * slot[158 - 1]
        - 0.018620115557513126 * slot[159 - 1]
        - 0.01864579248404537 * slot[160 - 1]
        - 0.018719112731700394 * slot[161 - 1]
        - 0.018803659847477793 * slot[162 - 1]
        - 0.018918026760284444 * slot[163 - 1]
        - 0.019025328578160066 * slot[164 - 1]
        - 0.019164247925215223 * slot[165 - 1]
        - 0.019209043771030748 * slot[166 - 1]
        - 0.019200055281257095 * slot[167 - 1]
        - 0.019139808127311253 * slot[168 - 1]
        - 0.01920540961651061 * slot[169 - 1]
        - 0.01931242261534546 * slot[170 - 1]
        - 0.01930349842935479 * slot[171 - 1]
        - 0.019255829966651483 * slot[172 - 1]
        - 0.019239978939067565 * slot[173 - 1]
        - 0.01924260915140854 * slot[174 - 1]
        - 0.019167778729110272 * slot[175 - 1]
        - 0.019186856344186536 * slot[176 - 1]
        - 0.019054458924967838 * slot[177 - 1]
        - 0.018994205344602482 * slot[178 - 1]
        - 0.01894458066212672 * slot[179 - 1]
        - 0.018882844344789148 * slot[180 - 1]
        - 0.01872290183894547 * slot[181 - 1]
        - 0.018597157046677342 * slot[182 - 1]
        - 0.018440177800817463 * slot[183 - 1]
        - 0.01829069579968281 * slot[184 - 1]
        - 0.018154609252103406 * slot[185 - 1]
        - 0.017879768860855336 * slot[186 - 1]
        - 0.017728423569157603 * slot[187 - 1]
        - 0.0143909425676989 * slot[188 - 1]
        - 0.01416983564454204 * slot[189 - 1]
        - 0.014028389393451962 * slot[190 - 1]
        - 0.013900223814470092 * slot[191 - 1]
        - 0.013717215824821221 * slot[192 - 1]
        - 0.013523753234602797 * slot[193 - 1]
        - 0.013381644130502045 * slot[194 - 1]
        - 0.013242806537567858 * slot[195 - 1]
        - 0.013157538872753845 * slot[196 - 1]
        - 0.01292828377958559 * slot[197 - 1]
        - 0.012773888740065135 * slot[198 - 1]
        - 0.01265924840845493 * slot[199 - 1]
        - 0.012571120276711756 * slot[200 - 1]
        - 0.01246025086893503 * slot[201 - 1]
        - 0.012283554979621171 * slot[202 - 1]
        - 0.012192234329296857 * slot[203 - 1]
        - 0.012137942066590773 * slot[204 - 1]
        - 0.01199328830972549 * slot[205 - 1]
        - 0.00993208906691401 * slot[206 - 1]
        - 0.00981227475076372 * slot[207 - 1]
        - 0.009681034036177923 * slot[208 - 1]
        - 0.009486297188966297 * slot[209 - 1]
        - 0.009304973414864453 * slot[210 - 1]
        - 0.009080672609714997 * slot[211 - 1]
        - 0.008926538673392348 * slot[212 - 1]
        - 0.008747526727768021 * slot[213 - 1]
        - 0.008628507026836848 * slot[214 - 1]
        - 0.00855862562376535 * slot[215 - 1]
        - 0.008461806404263765 * slot[216 - 1]
        - 0.008373561957271392 * slot[217 - 1]
        - 0.008151279909348564 * slot[218 - 1]
        - 0.008035833559007048 * slot[219 - 1]
        - 0.007878074639398311 * slot[220 - 1]
        - 0.007731384424551718 * slot[221 - 1]
        - 0.007674247883862302 * slot[222 - 1]
        - 0.007480581027701517 * slot[223 - 1]
        - 0.00739989722193016 * slot[224 - 1]
        - 0.0073020321040572975 * slot[225 - 1]
        - 0.007223966098206321 * slot[226 - 1]
        - 0.007088628047783925 * slot[227 - 1]
        - 0.006964230343408242 * slot[228 - 1]
        - 0.00690598766979579 * slot[229 - 1]
        - 0.006784051898778949 * slot[230 - 1]
        - 0.0066711756590074195 * slot[231 - 1]
        - 0.006610264212219908 * slot[232 - 1]
        - 0.006521893352198157 * slot[233 - 1]
        - 0.006387249941607781 * slot[234 - 1]
        - 0.006375079866441397 * slot[235 - 1]
        - 0.006219182896120065 * slot[236 - 1]
        - 0.006147716126916939 * slot[237 - 1]
        - 0.006065868517189078 * slot[238 - 1]
        - 0.005846469412000385 * slot[239 - 1]
        - 0.005743867811174836 * slot[240 - 1]
        - 0.005685995730813601 * slot[241 - 1]
        - 0.005640182867342912 * slot[242 - 1]
        - 0.005548139801071938 * slot[243 - 1]
        - 0.005418644913924256 * slot[244 - 1]
        - 0.005331810367641753 * slot[245 - 1]
        - 0.0052731297400840605 * slot[246 - 1]
        - 0.005152946453426777 * slot[247 - 1]
        - 0.00519776193348362 * slot[248 - 1]
        - 0.0051491441999874914 * slot[249 - 1]
        - 0.005072857091411818 * slot[250 - 1]
        - 0.004946480614036325 * slot[251 - 1]
        - 0.004849208448898196 * slot[252 - 1]
        - 0.004746251797004135 * slot[253 - 1]
        - 0.004706004113410831 * slot[254 - 1]
        - 0.004571284455003158 * slot[255 - 1]
        - 0.004463253292269082 * slot[256 - 1]
        - 0.0044455423658506694 * slot[257 - 1]
        - 0.004366307534310734 * slot[258 - 1]
        - 0.004336983622214118 * slot[259 - 1]
        - 0.004245934522100252 * slot[260 - 1]
        - 0.003272098635276185 * slot[261 - 1]
        - 0.0032120305858740227 * slot[262 - 1]
        - 0.0030648535840805155 * slot[263 - 1]
        - 0.0030346140546285588 * slot[264 - 1]
        - 0.002981755125164063 * slot[265 - 1]
        - 0.0028883092715222213 * slot[266 - 1]
        - 0.0029143933427026847 * slot[267 - 1]
        - 0.0028795347771758354 * slot[268 - 1]
        - 0.0028247673248653756 * slot[269 - 1]
        - 0.002369767513451682 * slot[270 - 1]
        - 0.0022480268400335846 * slot[271 - 1]
        - 0.002261264530483066 * slot[272 - 1]
        - 0.001212903583670368 * slot[273 - 1]
        - 0.001133503954576572 * slot[274 - 1]
        - 0.001145414753569322 * slot[275 - 1]
        - 0.0010989877875704221 * slot[276 - 1]
        - 0.001053317228487255 * slot[277 - 1]
        - 0.0010529082768351902 * slot[278 - 1]
        - 0.00037259598260261975 * slot[279 - 1]
        - 0.00027114179323008586 * slot[280 - 1]
        - 0.0002557589016344403 * slot[281 - 1]
        - 0.00022783803138445017 * slot[282 - 1]
        - 0.0002897706572790688 * slot[283 - 1]
        - 0.0002514406332667067 * slot[284 - 1]
        - 0.00004966151072005591 * slot[285 - 1]
        - 0.00006541619903660358 * slot[286 - 1]
        - 0.0000187694665332772 * slot[287 - 1]
        + 0.00010977863890993955 * slot[288 - 1]
        + 0.001638852913454785 * slot[289 - 1]
        + 0.0017043048816274518 * slot[290 - 1]
        + 0.001736089408557071 * slot[291 - 1]
        + 0.0018402506074991847 * slot[292 - 1]
        + 0.0018550626930139021 * slot[293 - 1]
        + 0.0018656702281824274 * slot[294 - 1]
        + 0.0019174366009964262 * slot[295 - 1]
        + 0.0019170840854498936 * slot[296 - 1]
        + 0.001979719157031033 * slot[297 - 1]
        + 0.0020387033204285927 * slot[298 - 1]
        + 0.002030305975070475 * slot[299 - 1]
        + 0.00206864649040917 * slot[300 - 1]
        + 0.0019998175565474796 * slot[301 - 1]
        + 0.001968427297088867 * slot[302 - 1]
        + 0.002009029294716233 * slot[303 - 1]
        + 0.002046092710211492 * slot[304 - 1]
        + 0.0019469043288166182 * slot[305 - 1]
        + 0.001938927484758406 * slot[306 - 1]
        + 0.001944399301893716 * slot[307 - 1]
        + 0.0020355190427796396 * slot[308 - 1]
        + 0.002043429531151754 * slot[309 - 1]
        + 0.001995619003538394 * slot[310 - 1]
        + 0.0020533787002623965 * slot[311 - 1]
        + 0.0020989897906476644 * slot[312 - 1]
        + 0.0020681577404398667 * slot[313 - 1]
        + 0.0020628970466793034 * slot[314 - 1]
        + 0.0020676996834844345 * slot[315 - 1]
        + 0.0021016407455823906 * slot[316 - 1]
        + 0.0020930720634306575 * slot[317 - 1]
        + 0.002089220382473654 * slot[318 - 1]
        + 0.0019619593119369603 * slot[319 - 1]
        + 0.0019830935676354855 * slot[320 - 1]
        + 0.0018601708183261165 * slot[321 - 1]
        + 0.0018291607809111984 * slot[322 - 1]
        + 0.001822691820530474 * slot[323 - 1]
        + 0.0017836547528532235 * slot[324 - 1]
        + 0.0017290993609852586 * slot[325 - 1]
        + 0.0015573554422180344 * slot[326 - 1]
        + 0.0014874936168074882 * slot[327 - 1]
        + 0.0013592864447346897 * slot[328 - 1]
        + 0.00289011549401065 * slot[329 - 1]
        + 0.0027434043186425573 * slot[330 - 1]
        + 0.0026605764590683016 * slot[331 - 1]
        + 0.0025416146969946883 * slot[332 - 1]
        + 0.0023719971132788757 * slot[333 - 1]
        + 0.002220361951707135 * slot[334 - 1]
        + 0.002105288735770232 * slot[335 - 1]
        + 0.007470111429222376 * slot[336 - 1]
        + 0.0073752988409037734 * slot[337 - 1]
        + 0.0072203241703273505 * slot[338 - 1]
        + 0.007010395839187631 * slot[339 - 1]
        + 0.006846593379434731 * slot[340 - 1]
        + 0.006807676036465687 * slot[341 - 1]
        + 0.006512217015569277 * slot[342 - 1]
        + 0.0062701102022552704 * slot[343 - 1]
        + 0.0060548711327892794 * slot[344 - 1]
        + 0.005977839433169882 * slot[345 - 1]
        + 0.005781020619676601 * slot[346 - 1]
        + 0.005634673493746294 * slot[347 - 1]
        + 0.005428119353910702 * slot[348 - 1]
        + 0.0051640756812471305 * slot[349 - 1]
        + 0.004877757880216183 * slot[350 - 1]
        + 0.004824636622467335 * slot[351 - 1]
        + 0.004560315297148898 * slot[352 - 1]
        + 0.004321586453344959 * slot[353 - 1]
        + 0.004048523628481312 * slot[354 - 1]
        + 0.003823006556207921 * slot[355 - 1]
        + 0.0036172429154814334 * slot[356 - 1]
        + 0.0033302891298672404 * slot[357 - 1]
        + 0.003053376169471997 * slot[358 - 1]
        + 0.0030070097150126174 * slot[359 - 1]
        + 0.0026910202735092796 * slot[360 - 1]
        + 0.0023771468974857013 * slot[361 - 1]
        + 0.0021007740371102633 * slot[362 - 1]
        + 0.0017598527437646335 * slot[363 - 1]
        + 0.0015701520410368614 * slot[364 - 1]
        + 0.0012338150727229493 * slot[365 - 1]
        + 0.0009736826616956688 * slot[366 - 1]
        + 0.0006839650837589377 * slot[367 - 1]
        + 0.00035220632278036324 * slot[368 - 1]
        + 0.00004545568226837442 * slot[369 - 1]
        - 0.0002579392134419548 * slot[370 - 1]
        - 0.0005582726399098054 * slot[371 - 1]
        - 0.0008269045848061201 * slot[372 - 1]
        - 0.001263213011513308 * slot[373 - 1]
        - 0.0016670293788975702 * slot[374 - 1]
        + 0.0055550503118719525 * slot[375 - 1]
        + 0.005201526731039338 * slot[376 - 1]
        + 0.004890106093817272 * slot[377 - 1]
        + 0.004638953268868018 * slot[378 - 1]
        + 0.004235865340620671 * slot[379 - 1]
        + 0.003900764032367516 * slot[380 - 1]
        + 0.003530259429607158 * slot[381 - 1]
        + 0.003193844296296421 * slot[382 - 1]
        + 0.002865828138860879 * slot[383 - 1]
        + 0.002426070490896507 * slot[384 - 1]
        + 0.001955608668977498 * slot[385 - 1]
        + 0.001527880827098704 * slot[386 - 1]
        + 0.0010295527397849794 * slot[387 - 1]
        + 0.0005477683362929348 * slot[388 - 1]
        + 0.00014559103412698722 * slot[389 - 1]
        - 0.0003282546802283396 * slot[390 - 1]
        + 0.012064888383267455 * slot[391 - 1]
        + 0.01171426858498267 * slot[392 - 1]
        + 0.011311201767479554 * slot[393 - 1]
        + 0.010848845131078845 * slot[394 - 1]
        + 0.010452658981353183 * slot[395 - 1]
        + 0.010079697349525678 * slot[396 - 1]
        + 0.009658525588601306 * slot[397 - 1]
        + 0.009384423337961093 * slot[398 - 1]
        + 0.008984242310086689 * slot[399 - 1]
        + 0.008706409073012551 * slot[400 - 1]
        + 0.008024453765561696 * slot[401 - 1]
        + 0.007641409781854658 * slot[402 - 1]
        + 0.0072666027226250905 * slot[403 - 1]
        + 0.006874819526560467 * slot[404 - 1]
        + 0.020427592518986074 * slot[405 - 1]
        + 0.020119793246230765 * slot[406 - 1]
        + 0.01975100041330767 * slot[407 - 1]
        + 0.01943858860312621 * slot[408 - 1]
        + 0.0189959077226651 * slot[409 - 1]
        + 0.014328016620512828 * slot[410 - 1]
        + 0.01396073223115299 * slot[411 - 1]
        + 0.013518390637148256 * slot[412 - 1]
        + 0.013066570530321196 * slot[413 - 1]
        + 0.012673872939753831 * slot[414 - 1]
        + 0.012129859891534088 * slot[415 - 1]
        + 0.011730670956667886 * slot[416 - 1]
        + 0.01137028875982159 * slot[417 - 1]
        + 0.01089344945086196 * slot[418 - 1]
        + 0.010348525708846267 * slot[419 - 1]
        + 0.00995764152443439 * slot[420 - 1]
        + 0.009378171368750084 * slot[421 - 1]
        + 0.008847103672708133 * slot[422 - 1]
        + 0.008279742903563208 * slot[423 - 1]
        + 0.007613371268617059 * slot[424 - 1]
        + 0.007013518410940574 * slot[425 - 1]
        + 0.0063708226230497055 * slot[426 - 1]
        + 0.005662887255634348 * slot[427 - 1]
        + 0.00502631210844587 * slot[428 - 1]
        + 0.0043306458368406586 * slot[429 - 1]
        + 0.0036343656953684938 * slot[430 - 1]
        + 0.0028972514849317558 * slot[431 - 1]
        + 0.002183680388275169 * slot[432 - 1]
        + 0.001357236122086758 * slot[433 - 1]
        + 0.03596578437244394 * slot[434 - 1]
        + 0.03526800084859375 * slot[435 - 1]
        + 0.034544792824788575 * slot[436 - 1]
        + 0.033693959817760744 * slot[437 - 1]
        + 0.03281113455073867 * slot[438 - 1]
        + 0.03196679742578616 * slot[439 - 1]
        + 0.01504341101726387 * slot[440 - 1]
        + 0.014059925610857636 * slot[441 - 1]
        + 0.013136993048709578 * slot[442 - 1]
        - 0.010362658092329675 * slot[443 - 1]
        - 0.011373669248345533 * slot[444 - 1]
        - 0.031015472292486804 * slot[445 - 1]
        - 0.032096093266893395 * slot[446 - 1]
        - 0.03316779369365395 * slot[447 - 1]
        - 0.03418764536229714 * slot[448 - 1]
        - 0.03530223640337386 * slot[449 - 1]
        - 0.012680325096672572 * slot[450 - 1]
        - 0.01366892943793114 * slot[451 - 1]
        - 0.01461222340607793 * slot[452 - 1]
        - 0.015541620614692681 * slot[453 - 1]
        - 0.016517524692201378 * slot[454 - 1]
        + 0.001871640265640495 * slot[455 - 1]
        + 0.0009402188434140935 * slot[456 - 1]
        - 0.000021303469749149825 * slot[457 - 1]
        - 0.024166927935949652 * slot[458 - 1]
        - 0.025078921971124695 * slot[459 - 1]
        - 0.02605758225806709 * slot[460 - 1]
        - 0.027138819830272096 * slot[461 - 1]
        - 0.028137217439501355 * slot[462 - 1]
        - 0.029096335354170548 * slot[463 - 1]
        - 0.03019643628018956 * slot[464 - 1]
        - 0.03115697155441504 * slot[465 - 1]
        - 0.03220607060556952 * slot[466 - 1]
        - 0.033200498536482013 * slot[467 - 1]
        - 0.034165524586651355 * slot[468 - 1]
        - 0.035111573890241964 * slot[469 - 1]
        - 0.005152245936366961 * slot[470 - 1]
        - 0.006102914020054382 * slot[471 - 1]
        - 0.00714875692360184 * slot[472 - 1]
        - 0.008021596901536306 * slot[473 - 1]
        - 0.00909381697318954 * slot[474 - 1]
        - 0.00993260470204889 * slot[475 - 1]
        - 0.010972926199017569 * slot[476 - 1]
        - 0.011975537409739117 * slot[477 - 1]
        - 0.012944624909943962 * slot[478 - 1]
        - 0.013862612076645516 * slot[479 - 1]
        - 0.014728078787086192 * slot[480 - 1]
        - 0.01582330830837902 * slot[481 - 1]
        - 0.016761773160571664 * slot[482 - 1]
        - 0.017804100986393838 * slot[483 - 1]
        + 0.07901493676178804 * slot[484 - 1]
        + 0.07844300301586239 * slot[485 - 1]
        + 0.0778523291837327 * slot[486 - 1]
        + 0.0772213564595285 * slot[487 - 1]
        + 0.07668877325696044 * slot[488 - 1]
        + 0.07632282612102131 * slot[489 - 1]
        + 0.07570321138677889 * slot[490 - 1]
        + 0.07517256752727362 * slot[491 - 1]
        + 0.07471379226529706 * slot[492 - 1]
        + 0.07412017599901714 * slot[493 - 1]
        + 0.07351557841441562 * slot[494 - 1]
        + 0.07303161662466856 * slot[495 - 1]
        + 0.07292207925221175 * slot[496 - 1]
        + 0.07252688400111408 * slot[497 - 1]
        + 0.07198337313305846 * slot[498 - 1]
        + 0.04220055670435057 * slot[499 - 1]
        + 0.03465284297375505 * slot[500 - 1]
        + 0.034147685731074164 * slot[501 - 1]
        + 0.0334882778047835 * slot[502 - 1]
        + 0.032987728970790595 * slot[503 - 1]
        + 0.03233357511819123 * slot[504 - 1]
        + 0.03155154194199775 * slot[505 - 1]
        + 0.030992163088886863 * slot[506 - 1]
        + 0.031067330938169293 * slot[507 - 1]
        + 0.030510961098656877 * slot[508 - 1]
        + 0.02587998146972715 * slot[509 - 1]
        + 0.02538185250235444 * slot[510 - 1]
        + 0.02449878728975566 * slot[511 - 1]
        + 0.023943294347399113 * slot[512 - 1]
        + 0.010509744403974211 * slot[513 - 1]
        - 0.006428837030931561 * slot[514 - 1]
        - 0.025140619051728475 * slot[515 - 1]
        - 0.038582151027232704 * slot[516 - 1]
        - 0.0399562283464851 * slot[517 - 1]
        - 0.041142760168883734 * slot[518 - 1]
        - 0.042257610061678684 * slot[519 - 1]
        - 0.043278785597266035 * slot[520 - 1]
        - 0.08335725125414836 * slot[521 - 1]
        - 0.08497653893500431 * slot[522 - 1]
        - 0.08659111921812936 * slot[523 - 1]
        - 0.08834874898658557 * slot[524 - 1]
        - 0.006102877522245729 * slot[525 - 1]
        + 0.0507049532273698 * slot[526 - 1]
        + 0.04966443803375119 * slot[527 - 1]
        + 0.04819440958569264 * slot[528 - 1]
        + 0.047221917636104616 * slot[529 - 1]
        + 0.04602366166843579 * slot[530 - 1]
        + 0.044798376767210664 * slot[531 - 1]
        + 0.04353896838916662 * slot[532 - 1]
        + 0.04209704355532128 * slot[533 - 1]
        + 0.04068214948435204 * slot[534 - 1]
        + 0.038389793606239626 * slot[535 - 1]
        + 0.037081126074602865 * slot[536 - 1]
        + 0.03573264059861542 * slot[537 - 1]
        + 0.03425013116747351 * slot[538 - 1]
        + 0.032840731838057184 * slot[539 - 1]
        + 0.0315469160435194 * slot[540 - 1]
        + 0.030227619040786892 * slot[541 - 1]
        + 0.02894663408540119 * slot[542 - 1]
        + 0.02763516984272008 * slot[543 - 1]
        + 0.026240293544519882 * slot[544 - 1]
        + 0.025114314762883228 * slot[545 - 1]
        + 0.023592108213287586 * slot[546 - 1]
        + 0.02210640881548693 * slot[547 - 1]
        + 0.0202831539172544 * slot[548 - 1]
        - 0.0041574098951797835 * slot[549 - 1]
        - 0.006231175370023931 * slot[550 - 1]
        - 0.007927797040440542 * slot[551 - 1]
        - 0.009593722063980454 * slot[552 - 1]
        - 0.011442256273313701 * slot[553 - 1]
        - 0.013060892043288058 * slot[554 - 1]
        + 0.0718792179709698 * slot[555 - 1]
        + 0.07022437327048221 * slot[556 - 1]
        + 0.06902337486104315 * slot[557 - 1]
        + 0.06768980365108854 * slot[558 - 1]
        + 0.06636238268594942 * slot[559 - 1]
        + 0.06507905622962198 * slot[560 - 1]
        + 0.06391877333653996 * slot[561 - 1]
        + 0.06267863741480238 * slot[562 - 1]
        + 0.061383804215856035 * slot[563 - 1]
        + 0.060006818247331804 * slot[564 - 1]
        + 0.05866845900350387 * slot[565 - 1]
        + 0.05745935623100776 * slot[566 - 1]
        + 0.05652745386657043 * slot[567 - 1]
        + 0.055265397968056525 * slot[568 - 1]
        + 0.054175021137955075 * slot[569 - 1]
        + 0.05302552374249188 * slot[570 - 1]
        + 0.05193202762248718 * slot[571 - 1]
        + 0.05058396553889085 * slot[572 - 1]
        + 0.04927134504246701 * slot[573 - 1]
        + 0.04801248947896223 * slot[574 - 1]
        + 0.04236361883153131 * slot[575 - 1]
        + 0.04128400809302487 * slot[576 - 1]
        + 0.040272990980570224 * slot[577 - 1]
        + 0.0389503498360928 * slot[578 - 1]
        + 0.03770757404818641 * slot[579 - 1]
        + 0.03660911520036586 * slot[580 - 1]
        + 0.03571764959953599 * slot[581 - 1]
        + 0.034658012151699995 * slot[582 - 1]
        + 0.033573675019295784 * slot[583 - 1]
        + 0.03245352155997396 * slot[584 - 1]
        + 0.03144411603633158 * slot[585 - 1]
        + 0.030496641367390883 * slot[586 - 1]
        + 0.02985022577899095 * slot[587 - 1]
        + 0.029195822036861767 * slot[588 - 1]
        + 0.028541964208472295 * slot[589 - 1]
        + 0.027732967493456835 * slot[590 - 1]
        + 0.02682294676603588 * slot[591 - 1]
        + 0.025779177706463964 * slot[592 - 1]
        + 0.025238990175946552 * slot[593 - 1]
        + 0.02327666786280709 * slot[594 - 1]
        + 0.023035201139073 * slot[595 - 1]
        + 0.02267639200169454 * slot[596 - 1]
        + 0.021944298002045916 * slot[597 - 1]
        + 0.02120641443072345 * slot[598 - 1]
        + 0.020411281802290713 * slot[599 - 1]
        + 0.020066370036603286 * slot[600 - 1]
        + 0.019461988595477795 * slot[601 - 1]
        + 0.019298480056093796 * slot[602 - 1]
        + 0.018957658090251073 * slot[603 - 1]
        + 0.01859873253566968 * slot[604 - 1]
        + 0.01827518266766587 * slot[605 - 1]
        + 0.018272711265424915 * slot[606 - 1]
        + 0.017993373413299645 * slot[607 - 1]
        + 0.017773496710462466 * slot[608 - 1]
        + 0.017645261441106466 * slot[609 - 1]
        + 0.01760007231286385 * slot[610 - 1]
        + 0.017407298914435352 * slot[611 - 1]
        + 0.017767734737990697 * slot[612 - 1]
        + 0.017797009798344726 * slot[613 - 1]
        + 0.004462320561501542 * slot[614 - 1]
        + 0.004382918996820509 * slot[615 - 1]
        + 0.004387089942767948 * slot[616 - 1]
        + 0.004741318614410219 * slot[617 - 1]
        + 0.00445813012150073 * slot[618 - 1]
        + 0.004764354282367543 * slot[619 - 1]
        + 0.004912783840022146 * slot[620 - 1]
        + 0.005193923560374699 * slot[621 - 1]
        + 0.005498570332747771 * slot[622 - 1]
        + 0.005455303691664602 * slot[623 - 1]
        + 0.005498721063160796 * slot[624 - 1]
        + 0.07489765603273205 * slot[625 - 1]
        + 0.07510899115613087 * slot[626 - 1]
        + 0.07567327077868276 * slot[627 - 1]
        + 0.07628986297793953 * slot[628 - 1]
        + 0.07685025866587515 * slot[629 - 1]
        + 0.0774482468746512 * slot[630 - 1]
        + 0.07845158831842873 * slot[631 - 1]
        + 0.07907191171267446 * slot[632 - 1]
        + 0.07988428689652982 * slot[633 - 1]
        + 0.08042657444117435 * slot[634 - 1]
        + 0.08108139766285644 * slot[635 - 1]
        + 0.08155447824696284 * slot[636 - 1]
        + 0.08201204703791654 * slot[637 - 1]
        + 0.08264526592281302 * slot[638 - 1]
        + 0.08347266472710624 * slot[639 - 1]
        + 0.08418149013871049 * slot[640 - 1]
        + 0.08497016859963016 * slot[641 - 1]
        + 0.0858223286693648 * slot[642 - 1]
        + 0.08659638179325051 * slot[643 - 1]
        + 0.08735200601554421 * slot[644 - 1]
        + 0.08818212041918973 * slot[645 - 1]
        + 0.08854180470984888 * slot[646 - 1]
        + 0.0893312283670712 * slot[647 - 1]
        + 0.09012493792635341 * slot[648 - 1]
        + 0.09058437074324428 * slot[649 - 1]
        + 0.09096461956964046 * slot[650 - 1]
        + 0.0917316512415347 * slot[651 - 1]
        + 0.0924936667766341 * slot[652 - 1]
        + 0.09310103856078213 * slot[653 - 1]
        + 0.09420146656309507 * slot[654 - 1]
        + 0.09452388072908643 * slot[655 - 1]
        + 0.09527018203079428 * slot[656 - 1]
        + 0.09602795305709655 * slot[657 - 1]
        + 0.09654657426641614 * slot[658 - 1]
        + 0.09718535958155992 * slot[659 - 1]
        + 0.09813589189938518 * slot[660 - 1]
        + 0.09903618253263684 * slot[661 - 1]
        + 0.0997945783241565 * slot[662 - 1]
        + 0.1005037227372786 * slot[663 - 1]
        + 0.10126329717205342 * slot[664 - 1]
        + 0.10218526484710391 * slot[665 - 1]
        + 0.10312699801309479 * slot[666 - 1]
        + 0.10341561969784961 * slot[667 - 1]
        + 0.10388302557183983 * slot[668 - 1]
        + 0.1046899574157171 * slot[669 - 1]
        + 0.10550609706401268 * slot[670 - 1]
        + 0.10583497015641738 * slot[671 - 1]
        + 0.10601019445554044 * slot[672 - 1]
        + 0.10639263569201902 * slot[673 - 1]
        + 0.10694949577678359 * slot[674 - 1]
        + 0.10779478001081333 * slot[675 - 1]
        + 0.10846308029881287 * slot[676 - 1]
        + 0.10879530415985145 * slot[677 - 1]
        + 0.10911642500853946 * slot[678 - 1]
        + 0.11003189807648908 * slot[679 - 1]
        + 0.11024150294405265 * slot[680 - 1]
        + 0.11070537284562885 * slot[681 - 1]
        + 0.11132564317942706 * slot[682 - 1]
        + 0.11181477686930014 * slot[683 - 1]
        + 0.11256257666435053 * slot[684 - 1]
        + 0.11297365786198095 * slot[685 - 1]
        + 0.11368851513337555 * slot[686 - 1]
        + 0.11470308709438938 * slot[687 - 1]
        + 0.11490499615121344 * slot[688 - 1]
        + 0.11549320368249825 * slot[689 - 1]
        + 0.11628006642712131 * slot[690 - 1]
        + 0.11675192967316544 * slot[691 - 1]
        + 0.11731609400703283 * slot[692 - 1]
        + 0.11769299743834255 * slot[693 - 1]
        + 0.11793384889913791 * slot[694 - 1]
        + 0.11842831619194424 * slot[695 - 1]
        + 0.11900080389544955 * slot[696 - 1]
        + 0.11917209487480346 * slot[697 - 1]
        + 0.12002030416200582 * slot[698 - 1]
        + 0.12040232929110711 * slot[699 - 1]
        + 0.12090291031725496 * slot[700 - 1]
        + 0.12142219009981697 * slot[701 - 1]
        + 0.12185435751071688 * slot[702 - 1]
        + 0.12212712371790382 * slot[703 - 1]
        + 0.12293987573651823 * slot[704 - 1]
        + 0.12337362392584306 * slot[705 - 1]
        + 0.12399427081550767 * slot[706 - 1]
        + 0.12414348714541606 * slot[707 - 1]
        + 0.12518538182074837 * slot[708 - 1]
        + 0.12563150355923083 * slot[709 - 1]
        + 0.12649879353290538 * slot[710 - 1]
        + 0.1269927805215211 * slot[711 - 1]
        + 0.1274327274135965 * slot[712 - 1]
        + 0.12827266553341263 * slot[713 - 1]
        + 0.1285033941778977 * slot[714 - 1]
        + 0.12881378688718528 * slot[715 - 1]
        + 0.1291716792666163 * slot[716 - 1]
        + 0.12958521809217327 * slot[717 - 1]
        + 0.1298140190175291 * slot[718 - 1]
        + 0.13045361665691585 * slot[719 - 1]
        + 0.13097400774953077 * slot[720 - 1]
        + 0.13114577630868143 * slot[721 - 1]
        + 0.13178352446022795 * slot[722 - 1]
        + 0.132321813564816 * slot[723 - 1]
        + 0.132631495691995 * slot[724 - 1]
        + 0.13319367453550077 * slot[725 - 1]
        + 0.13375452918692526 * slot[726 - 1]
        + 0.1341679707319012 * slot[727 - 1]
        + 0.1351780897696142 * slot[728 - 1]
        + 0.13572784844812708 * slot[729 - 1]
        + 0.1364292149298885 * slot[730 - 1]
        + 0.1370218360772729 * slot[731 - 1]
        + 0.13766686918089713 * slot[732 - 1]
        + 0.1382107738079067 * slot[733 - 1]
        + 0.1391290415949284 * slot[734 - 1]
        + 0.13956921231857916 * slot[735 - 1]
        + 0.14022462349222023 * slot[736 - 1]
        + 0.14055101588003413 * slot[737 - 1]
        + 0.14147150740901474 * slot[738 - 1]
        + 0.1422332252913789 * slot[739 - 1]
        + 0.14313643139309867 * slot[740 - 1]
        + 0.14397934106788607 * slot[741 - 1]
        + 0.14491601120798558 * slot[742 - 1]
        + 0.14603925434906675 * slot[743 - 1]
        + 0.14659692427220858 * slot[744 - 1]
        + 0.14747634633816437 * slot[745 - 1]
        + 0.1480450258460289 * slot[746 - 1]
        + 0.1486352199711739 * slot[747 - 1]
        + 0.14959429927014528 * slot[748 - 1]
        + 0.15063070902082817 * slot[749 - 1]
        + 0.15178872469461513 * slot[750 - 1]
        + 0.15334939254792362 * slot[751 - 1]
        + 0.15468836048046794 * slot[752 - 1]
        + 0.1566144209847916 * slot[753 - 1]
        + 0.1721061715222415 * slot[754 - 1]
        + 0.17331139431895226 * slot[755 - 1]
        + 0.17455927472013494 * slot[756 - 1]
        + 0.17568772000714578 * slot[757 - 1]
        + 0.17684704330136528 * slot[758 - 1]
        + 0.17798219035983806 * slot[759 - 1]
        + 0.17913361549225443 * slot[760 - 1]
        + 0.18044548249027154 * slot[761 - 1]
        + 0.18195482798972662 * slot[762 - 1]
        + 0.18334246059392706 * slot[763 - 1]
        + 0.1846509507087981 * slot[764 - 1]
        + 0.1856776645930925 * slot[765 - 1]
        + 0.1864232362373791 * slot[766 - 1]
        + 0.18752563718968662 * slot[767 - 1]
        + 0.18883777394538614 * slot[768 - 1]
        + 0.18982043043126579 * slot[769 - 1]
        + 0.1911977246881641 * slot[770 - 1]
        + 0.1921002838477575 * slot[771 - 1]
        + 0.19352536771758844 * slot[772 - 1]
        + 0.19485091322494835 * slot[773 - 1]
        + 0.19605088944421464 * slot[774 - 1]
        + 0.19694564246141225 * slot[775 - 1]
        + 0.197875540525203 * slot[776 - 1]
        + 0.19977952074492414 * slot[777 - 1]
        + 0.20047708770572886 * slot[778 - 1]
        + 0.20230568589743184 * slot[779 - 1]
        + 0.20289210169411637 * slot[780 - 1]
        + 0.2043386818499466 * slot[781 - 1]
        + 0.20460637103641183 * slot[782 - 1]
        + 0.20571733019586388 * slot[783 - 1]
        + 0.20680411884815625 * slot[784 - 1]
        + 0.207285120135408 * slot[785 - 1]
        + 0.2081967968469581 * slot[786 - 1]
        + 0.20899361728266086 * slot[787 - 1]
        + 0.20982218481179685 * slot[788 - 1]
        + 0.20999576623967753 * slot[789 - 1]
        + 0.21092903815501018 * slot[790 - 1]
        + 0.2113864729777917 * slot[791 - 1]
        + 0.2115257685775915 * slot[792 - 1]
        + 0.21211993159504758 * slot[793 - 1]
        + 0.21239141285402444 * slot[794 - 1]
        + 0.21317481161096036 * slot[795 - 1]
        + 0.21362944761082484 * slot[796 - 1]
        + 0.21397038911032512 * slot[797 - 1]
        + 0.21435729331951628 * slot[798 - 1]
        + 0.21514304039572973 * slot[799 - 1]
        + 0.21503259944685124 * slot[800 - 1]
        + 0.21532777490760757 * slot[801 - 1]
        + 0.21618169883218855 * slot[802 - 1]
        + 0.21635601093739695 * slot[803 - 1]
        + 0.21715267005556074 * slot[804 - 1]
        + 0.21820447622341407 * slot[805 - 1]
        + 0.21954936694686883 * slot[806 - 1]
        + 0.21983439652810963 * slot[807 - 1]
        + 0.22079798060804007 * slot[808 - 1]
        + 0.22375623596480673 * slot[809 - 1]
        + 0.22462273302862895 * slot[810 - 1]
        + 0.22536474949341306 * slot[811 - 1]
        + 0.2259530267125207 * slot[812 - 1]
        + 0.22678909350618695 * slot[813 - 1]
        + 0.22747142205853632 * slot[814 - 1]
        + 0.22733601900965714 * slot[815 - 1]
        + 0.22833508616812212 * slot[816 - 1]
        + 0.22868732717007953 * slot[817 - 1]
        + 0.22945645805734738 * slot[818 - 1]
        + 0.22955063975577095 * slot[819 - 1]
        + 0.23051141659628926 * slot[820 - 1]
        + 0.23033362253150588 * slot[821 - 1]
        + 0.23171982174125827 * slot[822 - 1]
        + 0.23215659211945916 * slot[823 - 1]
        + 0.23274152564399908 * slot[824 - 1]
        + 0.23349729438091074 * slot[825 - 1]
        + 0.23372684250301828 * slot[826 - 1]
        + 0.2349377340469927 * slot[827 - 1]
        + 0.23526958467589312 * slot[828 - 1]
        + 0.23580330121940327 * slot[829 - 1]
        + 0.23620510658239946 * slot[830 - 1]
        + 0.23568539273360328 * slot[831 - 1]
        + 0.2368527999372308 * slot[832 - 1]
        + 0.2369351762142354 * slot[833 - 1]
        + 0.2385774995746872 * slot[834 - 1]
        + 0.23988752026256188 * slot[835 - 1]
        + 0.2417261588364106 * slot[836 - 1]
        + 0.24220849666961677 * slot[837 - 1]
        + 0.24233057256322776 * slot[838 - 1]
        + 0.24415515813376937 * slot[839 - 1]
        + 0.24300036040006429 * slot[840 - 1]
        + 0.24593485051173022 * slot[841 - 1]
        + 0.2461500884627499 * slot[842 - 1]
        + 0.245518205737506 * slot[843 - 1]
        + 0.24643535415917484 * slot[844 - 1]
        + 0.24925328635158403 * slot[845 - 1]
        + 0.2510887126551754 * slot[846 - 1]
        + 0.2510401530966881 * slot[847 - 1]
        + 0.2514291171214713 * slot[848 - 1]
        + 0.25169932156028724 * slot[849 - 1]
        + 0.2559528059904543 * slot[850 - 1]
        + 0.2555383763138127 * slot[851 - 1]
        + 0.25437148956275435 * slot[852 - 1]
        + 0.2555231742950719 * slot[853 - 1]
        + 0.25457823655766104 * slot[854 - 1]
        + 0.25744369593237143 * slot[855 - 1]
        + 0.25612993482538954 * slot[856 - 1]
        + 0.25617675826658337 * slot[857 - 1]
        + 0.2567769253300378 * slot[858 - 1]
        + 0.26152657782848737 * slot[859 - 1]
        + 0.2617067952075105 * slot[860 - 1]
        + 0.26128749047273614 * slot[861 - 1]
        + 0.2609597065405411 * slot[862 - 1]
        + 0.26157492306786817 * slot[863 - 1]
        + 0.2659762630282364 * slot[864 - 1]
        + 0.266660436063571 * slot[865 - 1]
        + 0.26711190491294273 * slot[866 - 1]
        + 0.26730212179538193 * slot[867 - 1]
        + 0.27204281631107374 * slot[868 - 1]
        + 0.27211151902050457 * slot[869 - 1]
        + 0.2707538035332852 * slot[870 - 1]
        + 0.27034354893910684 * slot[871 - 1]
        + 0.27021197143024367 * slot[872 - 1]
        + 0.27359310253764835 * slot[873 - 1]
        + 0.2719791954003321 * slot[874 - 1]
        + 0.27118678820407605 * slot[875 - 1]
        + 0.2706289309399426 * slot[876 - 1]
        + 0.26954494965239983 * slot[877 - 1]
        + 0.27192959443325726 * slot[878 - 1]
        + 0.27004552098977663 * slot[879 - 1]
        + 0.2700594260863881 * slot[880 - 1]
        + 0.2678194038224915 * slot[881 - 1]
        + 0.2713818455497455 * slot[882 - 1]
        + 0.26923518046868417 * slot[883 - 1]
        + 0.26774620840441 * slot[884 - 1]
        + 0.26481244733263903 * slot[885 - 1]
        + 0.2623647439796307 * slot[886 - 1]
        + 0.2653773661986149 * slot[887 - 1]
        + 0.26222163031295964 * slot[888 - 1]
        + 0.2605144774158493 * slot[889 - 1]
        + 0.25758952380555006 * slot[890 - 1]
        + 0.26100837457223736 * slot[891 - 1]
        + 0.25682440375140925 * slot[892 - 1]
        + 0.25534607670867726 * slot[893 - 1]
        + 0.2531633103215483 * slot[894 - 1]
        + 0.2515936130785372 * slot[895 - 1]
        + 0.2514728982427409 * slot[896 - 1]
        + 0.2511488867065527 * slot[897 - 1]
        + 0.2510116161064974 * slot[898 - 1]
        + 0.24815003280609835 * slot[899 - 1]
        + 0.2481801646845387 * slot[900 - 1]
        + 0.2484754698391067 * slot[901 - 1]
        + 0.25285856314349403 * slot[902 - 1]
        + 0.25120689964781723 * slot[903 - 1]
        + 0.2523916009231172 * slot[904 - 1]
        + 0.2548517796561108 * slot[905 - 1]
        + 0.256891657534119 * slot[906 - 1]
        + 0.25977884247139976 * slot[907 - 1]
        + 0.2598585803032042 * slot[908 - 1]
        + 0.2652108973679878 * slot[909 - 1]
        + 0.27055560170443044 * slot[910 - 1]
        + 0.2747068796553872 * slot[911 - 1]
        + 0.2772534381984023 * slot[912 - 1]
        + 0.28524578268969697 * slot[913 - 1]
        + 0.29422238816840457 * slot[914 - 1]
        + 0.3046760588096132 * slot[915 - 1]
        + 0.3157776521281605 * slot[916 - 1]
        + 0.32708503172681425 * slot[917 - 1]
        + 0.3433205243976533 * slot[918 - 1]
        + 0.36375447296833785 * slot[919 - 1]
        + 0.3849034543302204 * slot[920 - 1]
        + 0.3953123948787572 * slot[921 - 1]
        + 0.4076205468760475 * slot[922 - 1]
        + 0.4352628345325459 * slot[923 - 1]
        + 0.464534463597515 * slot[924 - 1]
        + 0.49780246110905274 * slot[925 - 1]
        + 0.5094779528701895 * slot[926 - 1]
        + 0.5419074561341719 * slot[927 - 1]
        + 0.594578500752554 * slot[928 - 1]
        + 0.6287621854694081 * slot[929 - 1]
        + 0.6310089549149326 * slot[930 - 1]
        + 0.6650116663244451 * slot[931 - 1]
        + 0.6776901758414212 * slot[932 - 1]
        + 0.7873208516985344 * slot[933 - 1]
        + 0.8388921867341012 * slot[934 - 1]
        + 0.8445568605247658 * slot[935 - 1]
        + 0.8884828320705112 * slot[936 - 1]
        + 0.9868092852107635 * slot[937 - 1]
        + 1.0331895170418826 * slot[938 - 1]
        + 1.061301635535256 * slot[939 - 1]
        + 1.1085398466041307 * slot[940 - 1]
        + 1.1334062888889105 * slot[941 - 1]
        + 1.273761153612758 * slot[942 - 1]
        + 1.3407562182493966 * slot[943 - 1]
        + 1.3807188852114984 * slot[944 - 1]
        + 1.3709295372147108 * slot[945 - 1]
        + 1.4738732220295674 * slot[946 - 1]
        + 1.5180567918999057 * slot[947 - 1]
        + 1.4569040467180574 * slot[948 - 1]
        + 1.4180241819080377 * slot[949 - 1]
        + 1.4663470660303992 * slot[950 - 1]
        + 1.7665855694637111 * slot[951 - 1]
        + 1.7324721091776627 * slot[952 - 1]
        + 1.7090008391476235 * slot[953 - 1]
        + 1.690416492014695 * slot[954 - 1]
        + 1.7028355601720586 * slot[955 - 1]
        + 1.9040957989885263 * slot[956 - 1]
        + 1.6867678996932165 * slot[957 - 1]
        + 1.7197696436040217 * slot[958 - 1]
        + 1.5015889254349046 * slot[959 - 1]
        + 1.9060055459973368 * slot[960 - 1]
        + 2.066718875709478 * slot[961 - 1]
        + 1.881737709505516 * slot[962 - 1]
        + 1.9182714102453375 * slot[963 - 1]
        + 1.8668770010382927 * slot[964 - 1]
        + 2.767737393965599 * slot[965 - 1]
        + 2.9857125437663092 * slot[966 - 1]
        + 3.357945754318952 * slot[967 - 1]
        + 3.229639148654132 * slot[968 - 1]
        + 4.545958029194712 * slot[969 - 1]
        + 4.580546559013673 * slot[970 - 1]
        + 4.325435524163781 * slot[971 - 1]
        + 4.511177556474724 * slot[972 - 1]
        + 3.6381541423767234 * slot[973 - 1]
        + 3.748570406849703 * slot[974 - 1]
        + 3.4134908971754294 * slot[975 - 1]
        + 3.060248629847451 * slot[976 - 1]
        + 2.279645798461712 * slot[977 - 1]
        + 1.4704965654899163 * slot[978 - 1]
        + 1.694152312795732 * slot[979 - 1]
        + 1.218336274306771 * slot[980 - 1]
        + 1.8472963591715534 * slot[981 - 1]
        + 0.9772746216748442 * slot[982 - 1]
        + 0.6766722011291689 * slot[983 - 1]
        + 0.22267662152283588 * slot[984 - 1]
        - 0.3377178924185821 * slot[985 - 1]
        + 0.8328313913037121 * slot[986 - 1]
        + 0.33321824152329543 * slot[987 - 1]
        + 0.39865353452121643 * slot[988 - 1]
        - 1.042827042021106 * slot[989 - 1]
        - 2.0523654127324504 * slot[990 - 1]
        - 3.0381303095653407 * slot[991 - 1]
        - 2.7901882200030506 * slot[992 - 1]
        - 1.342609154855977 * slot[993 - 1]
        - 3.4179702154250116 * slot[994 - 1]
        - 1.1576918860303866 * slot[995 - 1]
        - 2.070805435571488 * slot[996 - 1]
        - 2.955040917921543 * slot[997 - 1]
        - 3.2898706992324898 * slot[998 - 1]
        - 4.739362323850019 * slot[999 - 1]
        - 5.668885000261006 * slot[1000 - 1]
        - 4.5845992700839195 * slot[1001 - 1]
        - 5.850498363226426 * slot[1002 - 1]
        - 8.270082844757171 * slot[1003 - 1]
        - 10.555603097059318 * slot[1004 - 1]
        - 8.295110360950408 * slot[1005 - 1]
        - 7.598677518504715 * slot[1006 - 1]
        - 4.037904582394127 * slot[1007 - 1]
        - 5.886961483976815 * slot[1008 - 1]
        - 8.013981829397464 * slot[1009 - 1]
        - 12.267281185549667 * slot[1010 - 1]
        - 16.35988715874757 * slot[1011 - 1]
        - 16.36761709874582 * slot[1012 - 1]
        - 4.308481040508021 * slot[1013 - 1]
        - 12.410597106977546 * slot[1014 - 1]
        - 10.835904600756184 * slot[1015 - 1]
        - 14.899482768866228 * slot[1016 - 1]
        - 14.023681021118861 * slot[1017 - 1]
        - 16.338252289483385 * slot[1018 - 1]
        - 25.270317452035627 * slot[1019 - 1]
        - 27.70645291705413 * slot[1020 - 1]
        - 46.34742082070532 * slot[1021 - 1]
        - 95.41336847906656 * slot[1022 - 1]
        - 197.4918671655324 * slot[1023 - 1];

    let linear = 276.75896124573
        - 0.059031699424137704 * slot[1 - 1]
        - 0.024409341568345844 * slot[2 - 1]
        - 0.010180197610123743 * slot[3 - 1]
        - 0.007608954119320594 * slot[4 - 1]
        - 0.004975108876915551 * slot[5 - 1]
        - 0.0025445737367902435 * slot[6 - 1]
        - 0.0009812607768172582 * slot[7 - 1]
        + 0.0008840931080856374 * slot[8 - 1]
        + 0.0026031117370026438 * slot[9 - 1]
        + 0.005053863869733737 * slot[10 - 1]
        + 0.005822710992217723 * slot[11 - 1]
        + 0.005654589128341836 * slot[12 - 1]
        + 0.0036480812978335305 * slot[13 - 1]
        + 0.00176032915226042 * slot[14 - 1]
        - 4.880831076303089e-7 * slot[15 - 1]
        - 0.001483700142458999 * slot[16 - 1]
        - 0.002217359921515263 * slot[17 - 1]
        - 0.003332527592384323 * slot[18 - 1]
        - 0.004303257308840225 * slot[19 - 1]
        - 0.005190033663427947 * slot[20 - 1]
        - 0.0059555145459226876 * slot[21 - 1]
        - 0.006568544144919845 * slot[22 - 1]
        - 0.006731600430904218 * slot[23 - 1]
        - 0.007024482035265646 * slot[24 - 1]
        - 0.007367679904067879 * slot[25 - 1]
        - 0.007650946144186294 * slot[26 - 1]
        - 0.0075433645477366905 * slot[27 - 1]
        - 0.007692615572131248 * slot[28 - 1]
        - 0.007794335828638299 * slot[29 - 1]
        - 0.007842811411177323 * slot[30 - 1]
        - 0.007846197734578548 * slot[31 - 1]
        - 0.007819741735835842 * slot[32 - 1]
        - 0.007762870008441003 * slot[33 - 1]
        - 0.007686348981153611 * slot[34 - 1]
        - 0.007577973464786598 * slot[35 - 1]
        - 0.00743512936806839 * slot[36 - 1]
        - 0.007277088398010361 * slot[37 - 1]
        - 0.00694766618553959 * slot[38 - 1]
        - 0.006736258103047596 * slot[39 - 1]
        - 0.006540841327361399 * slot[40 - 1]
        - 0.006348275108871576 * slot[41 - 1]
        - 0.00616502381194533 * slot[42 - 1]
        - 0.005957374332399188 * slot[43 - 1]
        - 0.005755105068512353 * slot[44 - 1]
        - 0.005547780447149306 * slot[45 - 1]
        - 0.005337939711950447 * slot[46 - 1]
        - 0.0051296506144949275 * slot[47 - 1]
        - 0.004681076911507185 * slot[48 - 1]
        - 0.004468639824854334 * slot[49 - 1]
        - 0.0042658827569925575 * slot[50 - 1]
        - 0.004065417221154683 * slot[51 - 1]
        - 0.003873939149172303 * slot[52 - 1]
        - 0.003692836696767826 * slot[53 - 1]
        - 0.003352815795741069 * slot[54 - 1]
        - 0.00317996868227064 * slot[55 - 1]
        - 0.0030069561637978116 * slot[56 - 1]
        - 0.0028312465853308722 * slot[57 - 1]
        - 0.0026649571034896274 * slot[58 - 1]
        - 0.0025072224825450247 * slot[59 - 1]
        - 0.0023453351533478943 * slot[60 - 1]
        - 0.0019102675940173936 * slot[61 - 1]
        - 0.0017675573297466312 * slot[62 - 1]
        - 0.0016211876606255662 * slot[63 - 1]
        - 0.0014722552822036001 * slot[64 - 1]
        - 0.0013495401687539709 * slot[65 - 1]
        - 0.0012170552022323785 * slot[66 - 1]
        - 0.0010805133262480617 * slot[67 - 1]
        - 0.0009622710853533335 * slot[68 - 1]
        - 0.000835473130666558 * slot[69 - 1]
        - 0.0007066212587367365 * slot[70 - 1]
        - 0.0005886074369864893 * slot[71 - 1]
        - 0.0004675419149010408 * slot[72 - 1]
        - 0.0000953331059060884 * slot[73 - 1]
        + 2.803541482348135e-6 * slot[74 - 1]
        + 0.00010012826547517125 * slot[75 - 1]
        + 0.00020940945717895236 * slot[76 - 1]
        + 0.0003053734388655798 * slot[77 - 1]
        + 0.0003951169339480053 * slot[78 - 1]
        + 0.0005066013766289484 * slot[79 - 1]
        + 0.0005913886684856569 * slot[80 - 1]
        + 0.0007020706251116636 * slot[81 - 1]
        + 0.0008184681613540623 * slot[82 - 1]
        + 0.0009395394517395116 * slot[83 - 1]
        + 0.0010441597654379238 * slot[84 - 1]
        + 0.001166582379987374 * slot[85 - 1]
        + 0.0012878231216281039 * slot[86 - 1]
        + 0.001384855888066904 * slot[87 - 1]
        + 0.001494264046414702 * slot[88 - 1]
        + 0.0016176777315026509 * slot[89 - 1]
        + 0.001759018933199392 * slot[90 - 1]
        + 0.0019343204407134434 * slot[91 - 1]
        + 0.0022204909029633446 * slot[92 - 1]
        + 0.003251919753299099 * slot[93 - 1]
        + 0.007714838430816919 * slot[94 - 1]
        + 0.011609588909083392 * slot[95 - 1]
        + 0.013274847051745019 * slot[96 - 1]
        + 0.01404322472043978 * slot[97 - 1]
        + 0.014511798198793687 * slot[98 - 1]
        + 0.014927911740511652 * slot[99 - 1]
        + 0.015306762109127924 * slot[100 - 1]
        + 0.015651397477969706 * slot[101 - 1]
        + 0.015953139305744518 * slot[102 - 1]
        + 0.016274117962472096 * slot[103 - 1]
        + 0.016625399703602075 * slot[104 - 1]
        + 0.016968377554139876 * slot[105 - 1]
        + 0.017371383901656694 * slot[106 - 1]
        - 0.014065161351216512 * slot[107 - 1]
        - 0.013716531030489417 * slot[108 - 1]
        - 0.013287829724807854 * slot[109 - 1]
        - 0.012828605039989649 * slot[110 - 1]
        - 0.012411009839419776 * slot[111 - 1]
        - 0.011939856790132874 * slot[112 - 1]
        - 0.011464067268305135 * slot[113 - 1]
        - 0.010926545286231305 * slot[114 - 1]
        - 0.010335710583937372 * slot[115 - 1]
        - 0.009830809241649549 * slot[116 - 1]
        - 0.009384736931046331 * slot[117 - 1]
        - 0.009010330520376196 * slot[118 - 1]
        - 0.008725128241283233 * slot[119 - 1]
        - 0.00840349129518876 * slot[120 - 1]
        - 0.008106874568966273 * slot[121 - 1]
        - 0.007810159450478887 * slot[122 - 1]
        - 0.00755546288146252 * slot[123 - 1]
        - 0.007349109515544764 * slot[124 - 1]
        - 0.0071479510656048345 * slot[125 - 1]
        - 0.006941907014169927 * slot[126 - 1]
        - 0.006778899837315713 * slot[127 - 1]
        - 0.006574240917712399 * slot[128 - 1]
        - 0.006410036651174396 * slot[129 - 1]
        - 0.006284685107983011 * slot[130 - 1]
        - 0.006102561331301155 * slot[131 - 1]
        - 0.005914827256702084 * slot[132 - 1]
        - 0.00572977978242851 * slot[133 - 1]
        - 0.00553231662329828 * slot[134 - 1]
        - 0.005376698689869832 * slot[135 - 1]
        - 0.0052409033954758134 * slot[136 - 1]
        - 0.004733871262786501 * slot[137 - 1]
        - 0.0046333444072460305 * slot[138 - 1]
        - 0.004571568099861528 * slot[139 - 1]
        - 0.004478489564365604 * slot[140 - 1]
        - 0.004237466104469657 * slot[141 - 1]
        - 0.004216534887539923 * slot[142 - 1]
        - 0.0041527890723227386 * slot[143 - 1]
        - 0.0041010667254611465 * slot[144 - 1]
        - 0.004098326446195365 * slot[145 - 1]
        - 0.0040555360423535185 * slot[146 - 1]
        - 0.004003533234225943 * slot[147 - 1]
        - 0.003997842546469698 * slot[148 - 1]
        - 0.0039534762848313105 * slot[149 - 1]
        - 0.003909488339505926 * slot[150 - 1]
        - 0.003899314354440708 * slot[151 - 1]
        - 0.003861714462371291 * slot[152 - 1]
        - 0.0038137189164745887 * slot[153 - 1]
        - 0.0038096066147265497 * slot[154 - 1]
        - 0.003775962030734047 * slot[155 - 1]
        - 0.0037355954667646013 * slot[156 - 1]
        - 0.003742520444586683 * slot[157 - 1]
        - 0.0036267085770943855 * slot[158 - 1]
        - 0.003611320163017759 * slot[159 - 1]
        - 0.003632710016492884 * slot[160 - 1]
        - 0.0035989181668576514 * slot[161 - 1]
        - 0.0035845389838173813 * slot[162 - 1]
        - 0.0036183754188407182 * slot[163 - 1]
        - 0.0036089049025228507 * slot[164 - 1]
        - 0.0036182955858400047 * slot[165 - 1]
        - 0.0035934559820306306 * slot[166 - 1]
        - 0.0036142959349699756 * slot[167 - 1]
        - 0.0036224615257148484 * slot[168 - 1]
        - 0.00358702183364097 * slot[169 - 1]
        - 0.0036046892088893658 * slot[170 - 1]
        - 0.0035674647771661667 * slot[171 - 1]
        - 0.0035463754888110212 * slot[172 - 1]
        - 0.0035866657135764485 * slot[173 - 1]
        - 0.0035529158360269836 * slot[174 - 1]
        - 0.003540189931217341 * slot[175 - 1]
        - 0.003538840859093029 * slot[176 - 1]
        - 0.00351232749259077 * slot[177 - 1]
        - 0.0035235410740344264 * slot[178 - 1]
        - 0.003532097267531759 * slot[179 - 1]
        - 0.003509504167761195 * slot[180 - 1]
        - 0.0035231530440308118 * slot[181 - 1]
        - 0.0035481811641335146 * slot[182 - 1]
        - 0.0035481933959450296 * slot[183 - 1]
        - 0.003534428058613641 * slot[184 - 1]
        - 0.0035579445876562383 * slot[185 - 1]
        - 0.0035598314610182915 * slot[186 - 1]
        - 0.003538752580490781 * slot[187 - 1]
        - 0.003533513118987604 * slot[188 - 1]
        - 0.0035363152227690826 * slot[189 - 1]
        - 0.003505078081042798 * slot[190 - 1]
        - 0.0035243812278629295 * slot[191 - 1]
        - 0.0034753960490142765 * slot[192 - 1]
        - 0.0034715744037873363 * slot[193 - 1]
        - 0.0035067366648011424 * slot[194 - 1]
        - 0.003488467675474064 * slot[195 - 1]
        - 0.003448795629011882 * slot[196 - 1]
        - 0.0034977262482724734 * slot[197 - 1]
        - 0.003487806338681125 * slot[198 - 1]
        - 0.0034737864972819677 * slot[199 - 1]
        - 0.0035142307900501183 * slot[200 - 1]
        - 0.0035457369817963593 * slot[201 - 1]
        - 0.0035146339967026307 * slot[202 - 1]
        - 0.0035068156147922535 * slot[203 - 1]
        - 0.0034840180013303683 * slot[204 - 1]
        - 0.0034353911897942397 * slot[205 - 1]
        - 0.003442580901497893 * slot[206 - 1]
        - 0.0034630162357541386 * slot[207 - 1]
        - 0.0034656716376016174 * slot[208 - 1]
        - 0.0034672188298294814 * slot[209 - 1]
        - 0.0035280193112569797 * slot[210 - 1]
        - 0.003522908472150006 * slot[211 - 1]
        - 0.003521708124642371 * slot[212 - 1]
        - 0.0036049727830366764 * slot[213 - 1]
        - 0.003577784555135391 * slot[214 - 1]
        - 0.003550769954890345 * slot[215 - 1]
        - 0.0035807304393145516 * slot[216 - 1]
        - 0.0035601214704421044 * slot[217 - 1]
        - 0.003553468955193781 * slot[218 - 1]
        - 0.0035779652578386252 * slot[219 - 1]
        - 0.003591486891299504 * slot[220 - 1]
        - 0.0035951437517061195 * slot[221 - 1]
        - 0.0036149695419619428 * slot[222 - 1]
        - 0.003637765771785309 * slot[223 - 1]
        - 0.003645050391420718 * slot[224 - 1]
        - 0.0036679502311562037 * slot[225 - 1]
        - 0.00368175099913531 * slot[226 - 1]
        - 0.0036747634858298383 * slot[227 - 1]
        - 0.003695867799870366 * slot[228 - 1]
        - 0.0036788718026768605 * slot[229 - 1]
        - 0.003669967992637713 * slot[230 - 1]
        - 0.003711077228453461 * slot[231 - 1]
        - 0.003712872957591804 * slot[232 - 1]
        - 0.0036738018890794945 * slot[233 - 1]
        - 0.003692902339540649 * slot[234 - 1]
        - 0.003668262338336359 * slot[235 - 1]
        - 0.003637693973313271 * slot[236 - 1]
        - 0.003686297459354776 * slot[237 - 1]
        - 0.0036486050397809577 * slot[238 - 1]
        - 0.003673552427569915 * slot[239 - 1]
        - 0.0036891533352769387 * slot[240 - 1]
        - 0.00362834935705614 * slot[241 - 1]
        - 0.003596932808813948 * slot[242 - 1]
        - 0.003617654247663206 * slot[243 - 1]
        - 0.0036525094943879346 * slot[244 - 1]
        - 0.0036005994244242877 * slot[245 - 1]
        - 0.0035949256793355307 * slot[246 - 1]
        - 0.0036186609570929484 * slot[247 - 1]
        - 0.0035790640363874593 * slot[248 - 1]
        - 0.0035043642537493356 * slot[249 - 1]
        - 0.003497058133941114 * slot[250 - 1]
        - 0.003458924174701875 * slot[251 - 1]
        - 0.003428048212267376 * slot[252 - 1]
        - 0.0034653685200741334 * slot[253 - 1]
        - 0.003409380493390231 * slot[254 - 1]
        - 0.003389179029891659 * slot[255 - 1]
        - 0.003376494159700437 * slot[256 - 1]
        - 0.003371188089373348 * slot[257 - 1]
        - 0.003344967132137794 * slot[258 - 1]
        - 0.003339584326300446 * slot[259 - 1]
        - 0.003311985894816274 * slot[260 - 1]
        - 0.0034077731348627818 * slot[261 - 1]
        - 0.0034133627839849252 * slot[262 - 1]
        - 0.0034197070858333523 * slot[263 - 1]
        - 0.003379829018544754 * slot[264 - 1]
        - 0.0033937924439837995 * slot[265 - 1]
        - 0.0033784998330141282 * slot[266 - 1]
        - 0.0033302661840644687 * slot[267 - 1]
        - 0.003365171870552582 * slot[268 - 1]
        - 0.0033704387542164865 * slot[269 - 1]
        - 0.003493841340696833 * slot[270 - 1]
        - 0.0035487416860019013 * slot[271 - 1]
        - 0.0035311962318049298 * slot[272 - 1]
        - 0.0037019147696155615 * slot[273 - 1]
        - 0.003754012167659744 * slot[274 - 1]
        - 0.003744821761420805 * slot[275 - 1]
        - 0.003707588383058819 * slot[276 - 1]
        - 0.0037336312976944917 * slot[277 - 1]
        - 0.0037132675292981007 * slot[278 - 1]
        - 0.0037603547847632245 * slot[279 - 1]
        - 0.0037894398576550146 * slot[280 - 1]
        - 0.00374999352900192 * slot[281 - 1]
        - 0.0036921648048434713 * slot[282 - 1]
        - 0.00368668390127378 * slot[283 - 1]
        - 0.0036356340813940013 * slot[284 - 1]
        - 0.0036171390423334927 * slot[285 - 1]
        - 0.003621744154788278 * slot[286 - 1]
        - 0.0035601790247797066 * slot[287 - 1]
        - 0.003540587163349425 * slot[288 - 1]
        - 0.003731319565976294 * slot[289 - 1]
        - 0.003753844413330469 * slot[290 - 1]
        - 0.003695032969536591 * slot[291 - 1]
        - 0.0036607548231229674 * slot[292 - 1]
        - 0.0036379590149929536 * slot[293 - 1]
        - 0.003624768020586372 * slot[294 - 1]
        - 0.003549822565734157 * slot[295 - 1]
        - 0.0035724123207825694 * slot[296 - 1]
        - 0.0035601228751061804 * slot[297 - 1]
        - 0.0034931739581764774 * slot[298 - 1]
        - 0.0034652800156198104 * slot[299 - 1]
        - 0.0034418806693850873 * slot[300 - 1]
        - 0.0033321524232001144 * slot[301 - 1]
        - 0.0033466401853822006 * slot[302 - 1]
        - 0.0032635233913959946 * slot[303 - 1]
        - 0.003199341578807124 * slot[304 - 1]
        - 0.0031931082632174943 * slot[305 - 1]
        - 0.0031472120176856563 * slot[306 - 1]
        - 0.0031039498369315396 * slot[307 - 1]
        - 0.003152685011825976 * slot[308 - 1]
        - 0.0030874109059442767 * slot[309 - 1]
        - 0.0030695216702202426 * slot[310 - 1]
        - 0.0031070483577397786 * slot[311 - 1]
        - 0.0030761036155802393 * slot[312 - 1]
        - 0.0030373703518035947 * slot[313 - 1]
        - 0.003045550182522781 * slot[314 - 1]
        - 0.0029711395710798763 * slot[315 - 1]
        - 0.0029475135728526812 * slot[316 - 1]
        - 0.002945437559806681 * slot[317 - 1]
        - 0.002866509942097078 * slot[318 - 1]
        - 0.002820970763080409 * slot[319 - 1]
        - 0.0028694934529526392 * slot[320 - 1]
        - 0.002772686062953354 * slot[321 - 1]
        - 0.002750716321345488 * slot[322 - 1]
        - 0.002771039236751248 * slot[323 - 1]
        - 0.002715456727362237 * slot[324 - 1]
        - 0.0026266204882592695 * slot[325 - 1]
        - 0.0026152094195057833 * slot[326 - 1]
        - 0.002534989218267813 * slot[327 - 1]
        - 0.0024963279111845055 * slot[328 - 1]
        - 0.002623217067910486 * slot[329 - 1]
        - 0.002634176821235665 * slot[330 - 1]
        - 0.002533426573580339 * slot[331 - 1]
        - 0.0024618613180307095 * slot[332 - 1]
        - 0.0024619838559224064 * slot[333 - 1]
        - 0.002363974378033582 * slot[334 - 1]
        - 0.002277498453043412 * slot[335 - 1]
        - 0.002837152060096485 * slot[336 - 1]
        - 0.002770189746636145 * slot[337 - 1]
        - 0.0026970290495182177 * slot[338 - 1]
        - 0.002644672926669433 * slot[339 - 1]
        - 0.0025493216804827012 * slot[340 - 1]
        - 0.0024952815174323563 * slot[341 - 1]
        - 0.00246200058597735 * slot[342 - 1]
        - 0.002350487104660283 * slot[343 - 1]
        - 0.0022619727309687795 * slot[344 - 1]
        - 0.0022237503699073253 * slot[345 - 1]
        - 0.002134696779768516 * slot[346 - 1]
        - 0.002027333223125436 * slot[347 - 1]
        - 0.0020372705517830683 * slot[348 - 1]
        - 0.0019387752170638198 * slot[349 - 1]
        - 0.0018502498274803937 * slot[350 - 1]
        - 0.001846220226303727 * slot[351 - 1]
        - 0.0017626497029114578 * slot[352 - 1]
        - 0.0016663369944091737 * slot[353 - 1]
        - 0.0016926165491143916 * slot[354 - 1]
        - 0.0015824128083111102 * slot[355 - 1]
        - 0.0015060833442516407 * slot[356 - 1]
        - 0.0014859299229372817 * slot[357 - 1]
        - 0.0013509171647360764 * slot[358 - 1]
        - 0.0012939898815834386 * slot[359 - 1]
        - 0.0012564762276239194 * slot[360 - 1]
        - 0.0011057387189028222 * slot[361 - 1]
        - 0.0009978305495495679 * slot[362 - 1]
        - 0.000972414156717597 * slot[363 - 1]
        - 0.0008497245949464264 * slot[364 - 1]
        - 0.0006840356054204203 * slot[365 - 1]
        - 0.0006811862431113189 * slot[366 - 1]
        - 0.0005894201654490895 * slot[367 - 1]
        - 0.0004937410079282852 * slot[368 - 1]
        - 0.0003762495058122642 * slot[369 - 1]
        - 0.0003491058847380638 * slot[370 - 1]
        - 0.00022784294827449657 * slot[371 - 1]
        - 0.00008552788068703184 * slot[372 - 1]
        - 0.00006385246009320138 * slot[373 - 1]
        + 0.00009299693514960202 * slot[374 - 1]
        - 0.00021186144220492945 * slot[375 - 1]
        - 0.00018124286441511728 * slot[376 - 1]
        - 0.00007065959958674255 * slot[377 - 1]
        - 0.00002349117895831438 * slot[378 - 1]
        - 0.00003443982232159076 * slot[379 - 1]
        + 0.0001161969319476714 * slot[380 - 1]
        + 0.00027625710651614857 * slot[381 - 1]
        + 0.00026300050102008647 * slot[382 - 1]
        + 0.00039839835646753794 * slot[383 - 1]
        + 0.0005273813722459691 * slot[384 - 1]
        + 0.0005915080748369488 * slot[385 - 1]
        + 0.0007286704601868882 * slot[386 - 1]
        + 0.0009284325149359212 * slot[387 - 1]
        + 0.0009424759528658492 * slot[388 - 1]
        + 0.0010738002923255186 * slot[389 - 1]
        + 0.0012044299775402886 * slot[390 - 1]
        + 0.0005119935573615855 * slot[391 - 1]
        + 0.0006282459633762227 * slot[392 - 1]
        + 0.0007711601153148877 * slot[393 - 1]
        + 0.0007382241323656225 * slot[394 - 1]
        + 0.0008791098643268637 * slot[395 - 1]
        + 0.001046255741534716 * slot[396 - 1]
        + 0.0011177083694235296 * slot[397 - 1]
        + 0.001269548039223823 * slot[398 - 1]
        + 0.0014748108020671649 * slot[399 - 1]
        + 0.001471331702803415 * slot[400 - 1]
        + 0.0012064186804023338 * slot[401 - 1]
        + 0.0013440445008991151 * slot[402 - 1]
        + 0.001418677805894286 * slot[403 - 1]
        + 0.0016043194426212763 * slot[404 - 1]
        + 0.0012120022113904724 * slot[405 - 1]
        + 0.0012396830683325851 * slot[406 - 1]
        + 0.0014312188537067864 * slot[407 - 1]
        + 0.0015722659149609494 * slot[408 - 1]
        + 0.0015787313966258265 * slot[409 - 1]
        + 0.0016216795870142731 * slot[410 - 1]
        + 0.0017989711662380569 * slot[411 - 1]
        + 0.0019352715086812173 * slot[412 - 1]
        + 0.001993463821178319 * slot[413 - 1]
        + 0.0021104804076988574 * slot[414 - 1]
        + 0.002289041701829933 * slot[415 - 1]
        + 0.0023292131968800857 * slot[416 - 1]
        + 0.0024933096182050895 * slot[417 - 1]
        + 0.0026425346885916245 * slot[418 - 1]
        + 0.0026291031660582967 * slot[419 - 1]
        + 0.002752685168519807 * slot[420 - 1]
        + 0.0028873301732563828 * slot[421 - 1]
        + 0.0029622192289409516 * slot[422 - 1]
        + 0.00303380454341575 * slot[423 - 1]
        + 0.003145831431205846 * slot[424 - 1]
        + 0.003159546503078267 * slot[425 - 1]
        + 0.0033103100579316386 * slot[426 - 1]
        + 0.0034507325521291234 * slot[427 - 1]
        + 0.0034240695211602266 * slot[428 - 1]
        + 0.0035414783234936145 * slot[429 - 1]
        + 0.0036486308300491344 * slot[430 - 1]
        + 0.0036788482777433368 * slot[431 - 1]
        + 0.0038451484039150855 * slot[432 - 1]
        + 0.003982389264971661 * slot[433 - 1]
        + 0.0031751390274690375 * slot[434 - 1]
        + 0.0033195332561105 * slot[435 - 1]
        + 0.0034391259902866213 * slot[436 - 1]
        + 0.0034539989927793733 * slot[437 - 1]
        + 0.0035940070497285526 * slot[438 - 1]
        + 0.003684640607205329 * slot[439 - 1]
        + 0.0034275961701638815 * slot[440 - 1]
        + 0.003518609427512831 * slot[441 - 1]
        + 0.0036248951462059544 * slot[442 - 1]
        + 0.003246289947716976 * slot[443 - 1]
        + 0.003383989956667084 * slot[444 - 1]
        + 0.0031835624687794367 * slot[445 - 1]
        + 0.003211216975567872 * slot[446 - 1]
        + 0.003385211207426506 * slot[447 - 1]
        + 0.0035374075268792783 * slot[448 - 1]
        + 0.0036090460427622632 * slot[449 - 1]
        + 0.0034674454541358197 * slot[450 - 1]
        + 0.0036543702227008253 * slot[451 - 1]
        + 0.0037623652110998426 * slot[452 - 1]
        + 0.0037532014648455937 * slot[453 - 1]
        + 0.00387923026375528 * slot[454 - 1]
        + 0.003552942020075882 * slot[455 - 1]
        + 0.0036261846634750333 * slot[456 - 1]
        + 0.003783759558791873 * slot[457 - 1]
        + 0.0036080875091853287 * slot[458 - 1]
        + 0.0035452266264665673 * slot[459 - 1]
        + 0.0037465325598671963 * slot[460 - 1]
        + 0.003959220102862522 * slot[461 - 1]
        + 0.00403311582810242 * slot[462 - 1]
        + 0.004238975665793194 * slot[463 - 1]
        + 0.0044067437572395006 * slot[464 - 1]
        + 0.004432068348475626 * slot[465 - 1]
        + 0.004676227427261212 * slot[466 - 1]
        + 0.004786441550058285 * slot[467 - 1]
        + 0.004765404334422112 * slot[468 - 1]
        + 0.0049325392535906456 * slot[469 - 1]
        + 0.004429531499521799 * slot[470 - 1]
        + 0.004467659128113693 * slot[471 - 1]
        + 0.004718638572041515 * slot[472 - 1]
        + 0.004913510121622318 * slot[473 - 1]
        + 0.0048828088193924625 * slot[474 - 1]
        + 0.0050039118262468546 * slot[475 - 1]
        + 0.005078728112813268 * slot[476 - 1]
        + 0.00509753028587499 * slot[477 - 1]
        + 0.005252121193840125 * slot[478 - 1]
        + 0.005366339833613078 * slot[479 - 1]
        + 0.005354215207774659 * slot[480 - 1]
        + 0.00554082819603967 * slot[481 - 1]
        + 0.005610196499030463 * slot[482 - 1]
        + 0.005515079076203249 * slot[483 - 1]
        + 0.004278100875858948 * slot[484 - 1]
        + 0.004433834274363917 * slot[485 - 1]
        + 0.004338341321166157 * slot[486 - 1]
        + 0.0044303133515282336 * slot[487 - 1]
        + 0.004541180624493676 * slot[488 - 1]
        + 0.004519802331937812 * slot[489 - 1]
        + 0.004674684568908046 * slot[490 - 1]
        + 0.004837560780071337 * slot[491 - 1]
        + 0.005037524254743791 * slot[492 - 1]
        + 0.004961735603122789 * slot[493 - 1]
        + 0.005094932102949915 * slot[494 - 1]
        + 0.005255562073493127 * slot[495 - 1]
        + 0.00516135647404347 * slot[496 - 1]
        + 0.005284963618925748 * slot[497 - 1]
        + 0.0054364032897519735 * slot[498 - 1]
        + 0.005346420204406649 * slot[499 - 1]
        + 0.005271605656079424 * slot[500 - 1]
        + 0.005350781269380526 * slot[501 - 1]
        + 0.005186002295432084 * slot[502 - 1]
        + 0.00531726491861485 * slot[503 - 1]
        + 0.005350294993844942 * slot[504 - 1]
        + 0.0051404784921012 * slot[505 - 1]
        + 0.005267956294092258 * slot[506 - 1]
        + 0.005245094615300835 * slot[507 - 1]
        + 0.0050088631458617945 * slot[508 - 1]
        + 0.0049284258069929605 * slot[509 - 1]
        + 0.0048991521496996315 * slot[510 - 1]
        + 0.004793242052308566 * slot[511 - 1]
        + 0.004901513632494768 * slot[512 - 1]
        + 0.004998818909200495 * slot[513 - 1]
        + 0.004808967083783169 * slot[514 - 1]
        + 0.004691733778478276 * slot[515 - 1]
        + 0.004629782151635283 * slot[516 - 1]
        + 0.004383101531884513 * slot[517 - 1]
        + 0.004413870567962053 * slot[518 - 1]
        + 0.0044308914909967426 * slot[519 - 1]
        + 0.004240319712115543 * slot[520 - 1]
        + 0.004087697608478026 * slot[521 - 1]
        + 0.0041167078233132455 * slot[522 - 1]
        + 0.003909346487410925 * slot[523 - 1]
        + 0.003860932105959918 * slot[524 - 1]
        + 0.003815035276654144 * slot[525 - 1]
        + 0.003499227594730329 * slot[526 - 1]
        + 0.003607204431790143 * slot[527 - 1]
        + 0.003434129419178155 * slot[528 - 1]
        + 0.003287345854039091 * slot[529 - 1]
        + 0.0034202739061408413 * slot[530 - 1]
        + 0.0032620196191444524 * slot[531 - 1]
        + 0.0029730946988623303 * slot[532 - 1]
        + 0.0029055723975341748 * slot[533 - 1]
        + 0.0028309729142221786 * slot[534 - 1]
        + 0.0027892250257038976 * slot[535 - 1]
        + 0.0024584861896071275 * slot[536 - 1]
        + 0.0024652985776903663 * slot[537 - 1]
        + 0.0024977130121527167 * slot[538 - 1]
        + 0.002271483776965744 * slot[539 - 1]
        + 0.0023444105293385416 * slot[540 - 1]
        + 0.002419686722126758 * slot[541 - 1]
        + 0.0021693377864810113 * slot[542 - 1]
        + 0.0022360138344531947 * slot[543 - 1]
        + 0.0022782204156670937 * slot[544 - 1]
        + 0.0020240295285648673 * slot[545 - 1]
        + 0.002013932754816633 * slot[546 - 1]
        + 0.001976265043441355 * slot[547 - 1]
        + 0.001798472380962586 * slot[548 - 1]
        + 0.0018494403273702832 * slot[549 - 1]
        + 0.001690451282148248 * slot[550 - 1]
        + 0.0013820359151387742 * slot[551 - 1]
        + 0.0013894694932412666 * slot[552 - 1]
        + 0.0013252321338376228 * slot[553 - 1]
        + 0.0010258153270249885 * slot[554 - 1]
        + 0.0009369626980120708 * slot[555 - 1]
        + 0.000833286688762415 * slot[556 - 1]
        + 0.0005519954120170326 * slot[557 - 1]
        + 0.0004891736727220739 * slot[558 - 1]
        + 0.0005905923742953138 * slot[559 - 1]
        + 0.00016267129796191507 * slot[560 - 1]
        + 0.00017910164509634386 * slot[561 - 1]
        - 3.92032875007673e-6 * slot[562 - 1]
        - 0.00041705635242892286 * slot[563 - 1]
        - 0.0004691655649625272 * slot[564 - 1]
        - 0.0006604290247701904 * slot[565 - 1]
        - 0.0009361732429822687 * slot[566 - 1]
        - 0.0009493923945644167 * slot[567 - 1]
        - 0.0010478287269264168 * slot[568 - 1]
        - 0.001375047246837329 * slot[569 - 1]
        - 0.0015523581002715521 * slot[570 - 1]
        - 0.001689578379950757 * slot[571 - 1]
        - 0.00223765032895993 * slot[572 - 1]
        - 0.0024329366820232887 * slot[573 - 1]
        - 0.002604453860415875 * slot[574 - 1]
        - 0.003091979525595979 * slot[575 - 1]
        - 0.00351321050104054 * slot[576 - 1]
        - 0.0035788755769299366 * slot[577 - 1]
        - 0.003940367557283985 * slot[578 - 1]
        - 0.004661357192782453 * slot[579 - 1]
        - 0.004739629350278927 * slot[580 - 1]
        - 0.0047936572410743755 * slot[581 - 1]
        - 0.00508868619610607 * slot[582 - 1]
        - 0.0052447764027279 * slot[583 - 1]
        - 0.005456042145298731 * slot[584 - 1]
        - 0.005843693081611112 * slot[585 - 1]
        - 0.005930804276265399 * slot[586 - 1]
        - 0.00616596867910669 * slot[587 - 1]
        - 0.006595869896641344 * slot[588 - 1]
        - 0.006614018128264312 * slot[589 - 1]
        - 0.006900609875216179 * slot[590 - 1]
        - 0.007387846787358723 * slot[591 - 1]
        - 0.007506777503135057 * slot[592 - 1]
        - 0.007464545691527228 * slot[593 - 1]
        - 0.008517473584973496 * slot[594 - 1]
        - 0.008555478814215362 * slot[595 - 1]
        - 0.008445249662143252 * slot[596 - 1]
        - 0.008854790001304048 * slot[597 - 1]
        - 0.008921890357833952 * slot[598 - 1]
        - 0.009036335252818856 * slot[599 - 1]
        - 0.009476076445450947 * slot[600 - 1]
        - 0.009510691647533726 * slot[601 - 1]
        - 0.009559560173040875 * slot[602 - 1]
        - 0.010013060659625973 * slot[603 - 1]
        - 0.010028925114036776 * slot[604 - 1]
        - 0.0100548068903577 * slot[605 - 1]
        - 0.010409261445871315 * slot[606 - 1]
        - 0.010436051830960191 * slot[607 - 1]
        - 0.010395412955791347 * slot[608 - 1]
        - 0.010823926828949861 * slot[609 - 1]
        - 0.010735422672649266 * slot[610 - 1]
        - 0.010661185599820528 * slot[611 - 1]
        - 0.010913635812242334 * slot[612 - 1]
        - 0.010776331056186113 * slot[613 - 1]
        - 0.014613042508576141 * slot[614 - 1]
        - 0.01439224230777131 * slot[615 - 1]
        - 0.01485810984684616 * slot[616 - 1]
        - 0.014679576227679542 * slot[617 - 1]
        - 0.014628702667911844 * slot[618 - 1]
        - 0.014885262930012104 * slot[619 - 1]
        - 0.014969417311609085 * slot[620 - 1]
        - 0.014666671105787354 * slot[621 - 1]
        - 0.015024419303265977 * slot[622 - 1]
        - 0.014996079174437436 * slot[623 - 1]
        - 0.015004126467550661 * slot[624 - 1]
        - 0.017461244073356363 * slot[625 - 1]
        - 0.01735813238894153 * slot[626 - 1]
        - 0.017251008692328606 * slot[627 - 1]
        - 0.017725202748801204 * slot[628 - 1]
        - 0.01757693545793532 * slot[629 - 1]
        - 0.017342927116401816 * slot[630 - 1]
        - 0.017586477779539936 * slot[631 - 1]
        - 0.017314517247621918 * slot[632 - 1]
        - 0.01705603746453006 * slot[633 - 1]
        - 0.01769137936018406 * slot[634 - 1]
        - 0.01741488921767916 * slot[635 - 1]
        - 0.0170843540208291 * slot[636 - 1]
        - 0.01756181763890598 * slot[637 - 1]
        - 0.017181931996329617 * slot[638 - 1]
        - 0.01662987657493087 * slot[639 - 1]
        - 0.01685890304628015 * slot[640 - 1]
        - 0.016322096220417727 * slot[641 - 1]
        - 0.0159913785121797 * slot[642 - 1]
        - 0.01638804315841168 * slot[643 - 1]
        - 0.01608561712195935 * slot[644 - 1]
        - 0.015582133124125588 * slot[645 - 1]
        - 0.015938005390333767 * slot[646 - 1]
        - 0.015496983827934551 * slot[647 - 1]
        - 0.015290428621966242 * slot[648 - 1]
        - 0.01565373911646426 * slot[649 - 1]
        - 0.015308014456260375 * slot[650 - 1]
        - 0.01484609388382446 * slot[651 - 1]
        - 0.015267115066978391 * slot[652 - 1]
        - 0.014788218778751657 * slot[653 - 1]
        - 0.01412061396251755 * slot[654 - 1]
        - 0.014583303569615132 * slot[655 - 1]
        - 0.01391738365197523 * slot[656 - 1]
        - 0.013235512512096195 * slot[657 - 1]
        - 0.01901670993036844 * slot[658 - 1]
        - 0.01931456578445832 * slot[659 - 1]
        - 0.018423499673834182 * slot[660 - 1]
        - 0.017756430421224614 * slot[661 - 1]
        - 0.01795213220874234 * slot[662 - 1]
        - 0.017361070169760878 * slot[663 - 1]
        - 0.01652375945683497 * slot[664 - 1]
        - 0.016570034997846202 * slot[665 - 1]
        - 0.015804969861618008 * slot[666 - 1]
        - 0.015216010908834416 * slot[667 - 1]
        - 0.015443254204889761 * slot[668 - 1]
        - 0.014759932296888269 * slot[669 - 1]
        - 0.014163194338004177 * slot[670 - 1]
        - 0.01436201339682149 * slot[671 - 1]
        - 0.013913020727157322 * slot[672 - 1]
        - 0.013071442603035065 * slot[673 - 1]
        - 0.01334084727011017 * slot[674 - 1]
        - 0.012391767169697641 * slot[675 - 1]
        - 0.011530767932209652 * slot[676 - 1]
        - 0.011843545340830222 * slot[677 - 1]
        - 0.01065132745462877 * slot[678 - 1]
        - 0.010870120982089003 * slot[679 - 1]
        - 0.011166217554813103 * slot[680 - 1]
        - 0.009478066002867743 * slot[681 - 1]
        - 0.00953608288520548 * slot[682 - 1]
        - 0.009817562688453182 * slot[683 - 1]
        - 0.008082792343363239 * slot[684 - 1]
        - 0.008304801951592116 * slot[685 - 1]
        - 0.008520292458103361 * slot[686 - 1]
        - 0.006987004538819664 * slot[687 - 1]
        - 0.007129733402264622 * slot[688 - 1]
        - 0.007271805007253511 * slot[689 - 1]
        - 0.0054799363525336355 * slot[690 - 1]
        - 0.005573049347541618 * slot[691 - 1]
        - 0.0055799178759297794 * slot[692 - 1]
        - 0.0038368925645691746 * slot[693 - 1]
        - 0.004049442696489585 * slot[694 - 1]
        - 0.00436498433342933 * slot[695 - 1]
        - 0.002765956708291876 * slot[696 - 1]
        - 0.0029464905523573118 * slot[697 - 1]
        - 0.0008569840698878153 * slot[698 - 1]
        - 0.0009073335128051179 * slot[699 - 1]
        - 0.0011341001435744406 * slot[700 - 1]
        + 0.0006741028541369901 * slot[701 - 1]
        + 0.0006924541828925907 * slot[702 - 1]
        + 0.0005389314285400071 * slot[703 - 1]
        + 0.0022853053404915274 * slot[704 - 1]
        + 0.0022655082428155913 * slot[705 - 1]
        + 0.0019368203563564766 * slot[706 - 1]
        + 0.0033605166521508733 * slot[707 - 1]
        + 0.003269969684049964 * slot[708 - 1]
        + 0.002888503324785371 * slot[709 - 1]
        + 0.004693238705689186 * slot[710 - 1]
        + 0.004420908852187625 * slot[711 - 1]
        + 0.004411350130920516 * slot[712 - 1]
        + 0.005979988517872355 * slot[713 - 1]
        + 0.005876288208151021 * slot[714 - 1]
        + 0.005773300306306956 * slot[715 - 1]
        + 0.007486744335757311 * slot[716 - 1]
        + 0.007277221631550818 * slot[717 - 1]
        + 0.006861757840396987 * slot[718 - 1]
        + 0.00873836715469257 * slot[719 - 1]
        + 0.008659211388119354 * slot[720 - 1]
        + 0.008324809745573347 * slot[721 - 1]
        + 0.010139120468210477 * slot[722 - 1]
        + 0.01026602962324211 * slot[723 - 1]
        + 0.009960069107951088 * slot[724 - 1]
        + 0.011550417662342868 * slot[725 - 1]
        + 0.011589181184121217 * slot[726 - 1]
        + 0.011619229205580069 * slot[727 - 1]
        + 0.01366212549289178 * slot[728 - 1]
        + 0.013589255390674385 * slot[729 - 1]
        + 0.01359695007196303 * slot[730 - 1]
        + 0.015713356970944455 * slot[731 - 1]
        + 0.01576682128324226 * slot[732 - 1]
        + 0.01569275906425916 * slot[733 - 1]
        + 0.01779373756086293 * slot[734 - 1]
        + 0.017823979918279314 * slot[735 - 1]
        + 0.017857363080041964 * slot[736 - 1]
        + 0.019684440798619272 * slot[737 - 1]
        + 0.019792621957296582 * slot[738 - 1]
        + 0.019685498443179777 * slot[739 - 1]
        + 0.021832218337202072 * slot[740 - 1]
        + 0.021945696840737463 * slot[741 - 1]
        + 0.022411147670038208 * slot[742 - 1]
        + 0.024648990899948434 * slot[743 - 1]
        + 0.024549739200819463 * slot[744 - 1]
        + 0.024682540361748332 * slot[745 - 1]
        + 0.02639137411488361 * slot[746 - 1]
        + 0.026182920544081358 * slot[747 - 1]
        + 0.026057163454397857 * slot[748 - 1]
        + 0.028112884840185787 * slot[749 - 1]
        + 0.028258800439412816 * slot[750 - 1]
        + 0.028250875967597167 * slot[751 - 1]
        + 0.03029066671402905 * slot[752 - 1]
        + 0.03047466616089626 * slot[753 - 1]
        + 0.021878342884465066 * slot[754 - 1]
        + 0.02423328434348731 * slot[755 - 1]
        + 0.024258378999363756 * slot[756 - 1]
        + 0.024751962850145023 * slot[757 - 1]
        + 0.027250054723301095 * slot[758 - 1]
        + 0.02750652789031235 * slot[759 - 1]
        + 0.028098504122851416 * slot[760 - 1]
        + 0.030772952851885093 * slot[761 - 1]
        + 0.0308273664088596 * slot[762 - 1]
        + 0.03132443490601491 * slot[763 - 1]
        + 0.03403692227805526 * slot[764 - 1]
        + 0.03422331162082694 * slot[765 - 1]
        + 0.03436864518984271 * slot[766 - 1]
        + 0.0369066978242097 * slot[767 - 1]
        + 0.03726998247442224 * slot[768 - 1]
        + 0.03761456908318663 * slot[769 - 1]
        + 0.04020038734170174 * slot[770 - 1]
        + 0.04044592507878655 * slot[771 - 1]
        + 0.040721470267824036 * slot[772 - 1]
        + 0.04302857935880493 * slot[773 - 1]
        + 0.04333402205142614 * slot[774 - 1]
        + 0.04368275049915984 * slot[775 - 1]
        + 0.04602771781051275 * slot[776 - 1]
        + 0.045780635158899005 * slot[777 - 1]
        + 0.04568139778952744 * slot[778 - 1]
        + 0.04828189374606534 * slot[779 - 1]
        + 0.04809611114965655 * slot[780 - 1]
        + 0.05034400630245878 * slot[781 - 1]
        + 0.05009014634814587 * slot[782 - 1]
        + 0.050395953907483616 * slot[783 - 1]
        + 0.05261161020450826 * slot[784 - 1]
        + 0.052217314736897724 * slot[785 - 1]
        + 0.05236493436101623 * slot[786 - 1]
        + 0.054541847064166926 * slot[787 - 1]
        + 0.05426609213340616 * slot[788 - 1]
        + 0.05359586542188792 * slot[789 - 1]
        + 0.05606779352544944 * slot[790 - 1]
        + 0.0555194162752574 * slot[791 - 1]
        + 0.05469298296341688 * slot[792 - 1]
        + 0.05660514856285781 * slot[793 - 1]
        + 0.05607016200744563 * slot[794 - 1]
        + 0.05550498915782603 * slot[795 - 1]
        + 0.057359384145442206 * slot[796 - 1]
        + 0.0567491373744254 * slot[797 - 1]
        + 0.056382719494373415 * slot[798 - 1]
        + 0.05780069190764426 * slot[799 - 1]
        + 0.056942090622542296 * slot[800 - 1]
        + 0.056584731004211214 * slot[801 - 1]
        + 0.058604144397332025 * slot[802 - 1]
        + 0.057795268224091034 * slot[803 - 1]
        + 0.057378652635801504 * slot[804 - 1]
        + 0.05987681831526848 * slot[805 - 1]
        + 0.05990502746653965 * slot[806 - 1]
        + 0.05913254133279674 * slot[807 - 1]
        + 0.06046173412981725 * slot[808 - 1]
        + 0.06171198248368732 * slot[809 - 1]
        + 0.061474274297139565 * slot[810 - 1]
        + 0.06292455694728916 * slot[811 - 1]
        + 0.0627785786807144 * slot[812 - 1]
        + 0.06236879943680397 * slot[813 - 1]
        + 0.0638279494444423 * slot[814 - 1]
        + 0.06272148141799333 * slot[815 - 1]
        + 0.06258624564810365 * slot[816 - 1]
        + 0.0640241519152247 * slot[817 - 1]
        + 0.06339596514464446 * slot[818 - 1]
        + 0.062472058380187614 * slot[819 - 1]
        + 0.064492339414269 * slot[820 - 1]
        + 0.06317628844175627 * slot[821 - 1]
        + 0.06266447202229465 * slot[822 - 1]
        + 0.06369865627205616 * slot[823 - 1]
        + 0.06370972596510575 * slot[824 - 1]
        + 0.06277473934045238 * slot[825 - 1]
        + 0.06410800190351139 * slot[826 - 1]
        + 0.06428542789260862 * slot[827 - 1]
        + 0.06294357352828471 * slot[828 - 1]
        + 0.06392838059046838 * slot[829 - 1]
        + 0.06277495318795556 * slot[830 - 1]
        + 0.06161671232621509 * slot[831 - 1]
        + 0.06396997526396982 * slot[832 - 1]
        + 0.06299649720056573 * slot[833 - 1]
        + 0.06347438132851964 * slot[834 - 1]
        + 0.06692607115693683 * slot[835 - 1]
        + 0.06746776255911731 * slot[836 - 1]
        + 0.06693424266013863 * slot[837 - 1]
        + 0.06897612015164135 * slot[838 - 1]
        + 0.06966468012499237 * slot[839 - 1]
        + 0.06758153986946128 * slot[840 - 1]
        + 0.07131367679391888 * slot[841 - 1]
        + 0.07164435821459293 * slot[842 - 1]
        + 0.06988124841060188 * slot[843 - 1]
        + 0.07165789225874433 * slot[844 - 1]
        + 0.0734385720261025 * slot[845 - 1]
        + 0.07524300115624685 * slot[846 - 1]
        + 0.07576919321433621 * slot[847 - 1]
        + 0.07539887273854097 * slot[848 - 1]
        + 0.0743780846811407 * slot[849 - 1]
        + 0.08035639906186062 * slot[850 - 1]
        + 0.07940725033451786 * slot[851 - 1]
        + 0.0776003593867786 * slot[852 - 1]
        + 0.08098648900812379 * slot[853 - 1]
        + 0.07921804625313413 * slot[854 - 1]
        + 0.08137670266397935 * slot[855 - 1]
        + 0.08136013215515307 * slot[856 - 1]
        + 0.08179898990301797 * slot[857 - 1]
        + 0.08130765624980082 * slot[858 - 1]
        + 0.08729227162033723 * slot[859 - 1]
        + 0.08673019456474251 * slot[860 - 1]
        + 0.08865812099639292 * slot[861 - 1]
        + 0.0881833787059046 * slot[862 - 1]
        + 0.0881520872067063 * slot[863 - 1]
        + 0.09358018221886234 * slot[864 - 1]
        + 0.0942749854702939 * slot[865 - 1]
        + 0.09451929789972145 * slot[866 - 1]
        + 0.0949994465065178 * slot[867 - 1]
        + 0.10052294027696992 * slot[868 - 1]
        + 0.10053817884960775 * slot[869 - 1]
        + 0.10054049667040889 * slot[870 - 1]
        + 0.09995881886060168 * slot[871 - 1]
        + 0.09983178238507753 * slot[872 - 1]
        + 0.10434192286206334 * slot[873 - 1]
        + 0.10220617052991737 * slot[874 - 1]
        + 0.10103755810365923 * slot[875 - 1]
        + 0.10262134816945416 * slot[876 - 1]
        + 0.10098215605785765 * slot[877 - 1]
        + 0.1027772373267039 * slot[878 - 1]
        + 0.10283854734780096 * slot[879 - 1]
        + 0.10219811881474075 * slot[880 - 1]
        + 0.09860598484899083 * slot[881 - 1]
        + 0.1021698332523638 * slot[882 - 1]
        + 0.09988996222333921 * slot[883 - 1]
        + 0.09659164650821654 * slot[884 - 1]
        + 0.09205350290297781 * slot[885 - 1]
        + 0.0883795496344015 * slot[886 - 1]
        + 0.09014499057804376 * slot[887 - 1]
        + 0.08554971499170042 * slot[888 - 1]
        + 0.0823702706761294 * slot[889 - 1]
        + 0.07758755932127345 * slot[890 - 1]
        + 0.0812686704127264 * slot[891 - 1]
        + 0.07429238931896326 * slot[892 - 1]
        + 0.0706674205591141 * slot[893 - 1]
        + 0.06923767802223899 * slot[894 - 1]
        + 0.0655575829896718 * slot[895 - 1]
        + 0.06240651835898718 * slot[896 - 1]
        + 0.060837941817179615 * slot[897 - 1]
        + 0.059960522495926184 * slot[898 - 1]
        + 0.053435706649781735 * slot[899 - 1]
        + 0.05075462387612855 * slot[900 - 1]
        + 0.047223280017622146 * slot[901 - 1]
        + 0.04695383237800724 * slot[902 - 1]
        + 0.04203612158376615 * slot[903 - 1]
        + 0.03781306577756626 * slot[904 - 1]
        + 0.03566030077991958 * slot[905 - 1]
        + 0.033250806335877 * slot[906 - 1]
        + 0.02914416308786271 * slot[907 - 1]
        + 0.020721313599837116 * slot[908 - 1]
        + 0.021622637217432805 * slot[909 - 1]
        + 0.01691403768908182 * slot[910 - 1]
        + 0.00906299987673098 * slot[911 - 1]
        + 0.0019569403131039437 * slot[912 - 1]
        - 0.00269886075280798 * slot[913 - 1]
        - 0.008067651806461365 * slot[914 - 1]
        - 0.01122446451074 * slot[915 - 1]
        - 0.016757557924295813 * slot[916 - 1]
        - 0.021332276007355162 * slot[917 - 1]
        - 0.023840073676748372 * slot[918 - 1]
        - 0.02882208788227289 * slot[919 - 1]
        - 0.02982276225612884 * slot[920 - 1]
        - 0.04419457049851092 * slot[921 - 1]
        - 0.06313742676630366 * slot[922 - 1]
        - 0.06953713520147951 * slot[923 - 1]
        - 0.06631782697468257 * slot[924 - 1]
        - 0.0785299938242505 * slot[925 - 1]
        - 0.11332360046649935 * slot[926 - 1]
        - 0.12269690150120063 * slot[927 - 1]
        - 0.12095996523858918 * slot[928 - 1]
        - 0.13861423346889995 * slot[929 - 1]
        - 0.18891853551377458 * slot[930 - 1]
        - 0.19148801650251476 * slot[931 - 1]
        - 0.24400859132038405 * slot[932 - 1]
        - 0.23056887923615632 * slot[933 - 1]
        - 0.24442684856962757 * slot[934 - 1]
        - 0.29580974254664544 * slot[935 - 1]
        - 0.32582268326627717 * slot[936 - 1]
        - 0.30856332557894733 * slot[937 - 1]
        - 0.33865592940061945 * slot[938 - 1]
        - 0.3987309201869484 * slot[939 - 1]
        - 0.4277927277582649 * slot[940 - 1]
        - 0.5485966779440987 * slot[941 - 1]
        - 0.4763215178123918 * slot[942 - 1]
        - 0.4446606656419231 * slot[943 - 1]
        - 0.5317725124311695 * slot[944 - 1]
        - 0.5846683002050745 * slot[945 - 1]
        - 0.5537279405210447 * slot[946 - 1]
        - 0.5767533015864837 * slot[947 - 1]
        - 0.7582348499435883 * slot[948 - 1]
        - 0.9164139927105954 * slot[949 - 1]
        - 0.9983035029483198 * slot[950 - 1]
        - 0.9505152520491106 * slot[951 - 1]
        - 1.0695182366310085 * slot[952 - 1]
        - 1.327767083509664 * slot[953 - 1]
        - 1.4055127987582796 * slot[954 - 1]
        - 1.7442376523536225 * slot[955 - 1]
        - 1.7546916383614661 * slot[956 - 1]
        - 2.1722261624568655 * slot[957 - 1]
        - 2.2276647352454484 * slot[958 - 1]
        - 2.614266591132327 * slot[959 - 1]
        - 2.4154932498956407 * slot[960 - 1]
        - 2.231459242358749 * slot[961 - 1]
        - 2.451445329690307 * slot[962 - 1]
        - 2.45565114411348 * slot[963 - 1]
        - 2.6294250450251906 * slot[964 - 1]
        - 1.7202931006436928 * slot[965 - 1]
        - 1.7158979100985239 * slot[966 - 1]
        - 1.5400246175898822 * slot[967 - 1]
        - 1.7765957116096904 * slot[968 - 1]
        - 1.0796499483800055 * slot[969 - 1]
        - 1.1466410862055074 * slot[970 - 1]
        - 1.296251769273003 * slot[971 - 1]
        - 0.9128786596202034 * slot[972 - 1]
        - 1.8782347851000556 * slot[973 - 1]
        - 1.4575256057187387 * slot[974 - 1]
        - 1.6858786548809932 * slot[975 - 1]
        - 1.8019609501998586 * slot[976 - 1]
        - 2.2486511389599433 * slot[977 - 1]
        - 2.97061157621015 * slot[978 - 1]
        - 2.6009539655887832 * slot[979 - 1]
        - 1.6129191819949793 * slot[980 - 1]
        - 2.1163337835289053 * slot[981 - 1]
        - 2.7711817420740643 * slot[982 - 1]
        - 2.214293428068915 * slot[983 - 1]
        - 2.26937064278655 * slot[984 - 1]
        - 2.6345937101191828 * slot[985 - 1]
        - 2.743779709831267 * slot[986 - 1]
        - 1.5326164832715479 * slot[987 - 1]
        - 0.8904361152297388 * slot[988 - 1]
        - 0.9977155855970793 * slot[989 - 1]
        - 1.1972352225198137 * slot[990 - 1]
        - 1.307410565443168 * slot[991 - 1]
        + 0.6593275570539362 * slot[992 - 1]
        + 0.4396354117141449 * slot[993 - 1]
        - 1.0608749723562583 * slot[994 - 1]
        + 0.15766609468158074 * slot[995 - 1]
        + 0.16753834024545827 * slot[996 - 1]
        + 0.9659860108330698 * slot[997 - 1]
        + 2.46941653716066 * slot[998 - 1]
        + 1.7294178069082293 * slot[999 - 1]
        + 2.190022593387508 * slot[1000 - 1]
        + 0.5445859382137798 * slot[1001 - 1]
        + 1.6868163697414884 * slot[1002 - 1]
        + 0.812184394328638 * slot[1003 - 1]
        + 2.7974057899045857 * slot[1004 - 1]
        - 1.2492626878558815 * slot[1005 - 1]
        + 0.4252003064647748 * slot[1006 - 1]
        - 1.8678498292628885 * slot[1007 - 1]
        - 2.1885136238490777 * slot[1008 - 1]
        - 2.175721462708496 * slot[1009 - 1]
        - 3.2055059499970757 * slot[1010 - 1]
        - 5.772911023060903 * slot[1011 - 1]
        - 6.400827631534423 * slot[1012 - 1]
        - 13.048109038552125 * slot[1013 - 1]
        - 22.129522031362555 * slot[1014 - 1]
        - 19.909936228944105 * slot[1015 - 1]
        - 22.099001375125358 * slot[1016 - 1]
        - 16.36642978467885 * slot[1017 - 1]
        - 15.164579619967688 * slot[1018 - 1]
        - 16.574413298885737 * slot[1019 - 1]
        - 10.806712300721342 * slot[1020 - 1]
        - 15.296911823785557 * slot[1021 - 1]
        - 35.21692264644518 * slot[1022 - 1]
        - 4.305146893646584 * slot[1023 - 1];

    let logistic = -89.5787087847014
        - 0.11490119217090321 * slot[1 - 1]
        - 0.05333087496202682 * slot[2 - 1]
        - 0.028473277934414464 * slot[3 - 1]
        - 0.023943941936957813 * slot[4 - 1]
        - 0.019550086224781712 * slot[5 - 1]
        - 0.015512770077387565 * slot[6 - 1]
        - 0.01300523350219427 * slot[7 - 1]
        - 0.010158747751370796 * slot[8 - 1]
        - 0.007687598577991992 * slot[9 - 1]
        - 0.003930930596534944 * slot[10 - 1]
        - 0.0018700028233219364 * slot[11 - 1]
        - 0.0011272855118020853 * slot[12 - 1]
        - 0.002871790369427081 * slot[13 - 1]
        - 0.004635474367787656 * slot[14 - 1]
        - 0.006435279860449549 * slot[15 - 1]
        - 0.008004512397366534 * slot[16 - 1]
        - 0.008573919813411508 * slot[17 - 1]
        - 0.009774346148761161 * slot[18 - 1]
        - 0.010842059201915475 * slot[19 - 1]
        - 0.011839549503048651 * slot[20 - 1]
        - 0.012708972735096236 * slot[21 - 1]
        - 0.01340506016779232 * slot[22 - 1]
        - 0.0135773350372876 * slot[23 - 1]
        - 0.013863293725122863 * slot[24 - 1]
        - 0.014212973970837648 * slot[25 - 1]
        - 0.01449667594300818 * slot[26 - 1]
        - 0.01426795080722656 * slot[27 - 1]
        - 0.0143780389749932 * slot[28 - 1]
        - 0.014429619507910656 * slot[29 - 1]
        - 0.014415118373604917 * slot[30 - 1]
        - 0.014345903189324745 * slot[31 - 1]
        - 0.01423433720174558 * slot[32 - 1]
        - 0.014085920803407037 * slot[33 - 1]
        - 0.013909982303705638 * slot[34 - 1]
        - 0.013695912291682863 * slot[35 - 1]
        - 0.013440068816806575 * slot[36 - 1]
        - 0.013147856074391147 * slot[37 - 1]
        - 0.012633251249335412 * slot[38 - 1]
        - 0.012272969052611403 * slot[39 - 1]
        - 0.011940004441356105 * slot[40 - 1]
        - 0.011615019585025815 * slot[41 - 1]
        - 0.011299874949671813 * slot[42 - 1]
        - 0.010926590476474301 * slot[43 - 1]
        - 0.010543424266546137 * slot[44 - 1]
        - 0.010169532820908916 * slot[45 - 1]
        - 0.009779288172394377 * slot[46 - 1]
        - 0.009386026789534194 * slot[47 - 1]
        - 0.008574897224241474 * slot[48 - 1]
        - 0.008193741532803394 * slot[49 - 1]
        - 0.007812612069571982 * slot[50 - 1]
        - 0.007450236576529801 * slot[51 - 1]
        - 0.00709992885044475 * slot[52 - 1]
        - 0.006753462840607841 * slot[53 - 1]
        - 0.0060139755026809285 * slot[54 - 1]
        - 0.005692779147129688 * slot[55 - 1]
        - 0.005363862322619211 * slot[56 - 1]
        - 0.005043605103858742 * slot[57 - 1]
        - 0.004725185077742543 * slot[58 - 1]
        - 0.004417422686692312 * slot[59 - 1]
        - 0.004111766126905789 * slot[60 - 1]
        - 0.003127989486777108 * slot[61 - 1]
        - 0.002837588166692064 * slot[62 - 1]
        - 0.00256043453830377 * slot[63 - 1]
        - 0.002284763267659464 * slot[64 - 1]
        - 0.002017264665643684 * slot[65 - 1]
        - 0.001761946248777305 * slot[66 - 1]
        - 0.0015093946078645112 * slot[67 - 1]
        - 0.0012534337854386577 * slot[68 - 1]
        - 0.0010042542788704523 * slot[69 - 1]
        - 0.0007703375857861482 * slot[70 - 1]
        - 0.0005225195796636632 * slot[71 - 1]
        - 0.0002969370206186312 * slot[72 - 1]
        + 0.0005060678320858863 * slot[73 - 1]
        + 0.0007271541797123698 * slot[74 - 1]
        + 0.0009337640300255267 * slot[75 - 1]
        + 0.0011346826974670012 * slot[76 - 1]
        + 0.0013397058261573741 * slot[77 - 1]
        + 0.0015261585208776536 * slot[78 - 1]
        + 0.0017300139205919808 * slot[79 - 1]
        + 0.001938294407276801 * slot[80 - 1]
        + 0.002131268141761864 * slot[81 - 1]
        + 0.0023354813068939243 * slot[82 - 1]
        + 0.002545948958722304 * slot[83 - 1]
        + 0.0027588517800188174 * slot[84 - 1]
        + 0.0029701715944631 * slot[85 - 1]
        + 0.003162902440762817 * slot[86 - 1]
        + 0.003366365756613969 * slot[87 - 1]
        + 0.0035441820318280297 * slot[88 - 1]
        + 0.003715197157612295 * slot[89 - 1]
        + 0.003909459483465713 * slot[90 - 1]
        + 0.0040851603378905055 * slot[91 - 1]
        + 0.004302921555451793 * slot[92 - 1]
        + 0.004820377125696104 * slot[93 - 1]
        + 0.006646887816461802 * slot[94 - 1]
        + 0.008236352994113726 * slot[95 - 1]
        + 0.009357037572843714 * slot[96 - 1]
        + 0.009736069411924973 * slot[97 - 1]
        + 0.009996018387062613 * slot[98 - 1]
        + 0.010251244066551424 * slot[99 - 1]
        + 0.010466467152363487 * slot[100 - 1]
        + 0.01065571734770243 * slot[101 - 1]
        + 0.010857682520409697 * slot[102 - 1]
        + 0.011048103824998275 * slot[103 - 1]
        + 0.011240355629140034 * slot[104 - 1]
        + 0.011453946437434573 * slot[105 - 1]
        + 0.011661513070848895 * slot[106 - 1]
        - 0.01925771051483423 * slot[107 - 1]
        - 0.019045007298892178 * slot[108 - 1]
        - 0.018827115468134804 * slot[109 - 1]
        - 0.0185998618661425 * slot[110 - 1]
        - 0.018373264136835224 * slot[111 - 1]
        - 0.018143016531938186 * slot[112 - 1]
        - 0.017915561868775957 * slot[113 - 1]
        - 0.01764211931804023 * slot[114 - 1]
        - 0.01737321061869198 * slot[115 - 1]
        - 0.017135058719376782 * slot[116 - 1]
        - 0.016908736568342766 * slot[117 - 1]
        - 0.01672939861762699 * slot[118 - 1]
        - 0.016575763622337182 * slot[119 - 1]
        - 0.01640183397047852 * slot[120 - 1]
        - 0.016260275656240837 * slot[121 - 1]
        - 0.016119031636758523 * slot[122 - 1]
        - 0.015993998014146315 * slot[123 - 1]
        - 0.01586389399872598 * slot[124 - 1]
        - 0.01576511034007226 * slot[125 - 1]
        - 0.01566904482983965 * slot[126 - 1]
        - 0.015578258246438015 * slot[127 - 1]
        - 0.015481342581424145 * slot[128 - 1]
        - 0.015415351097636226 * slot[129 - 1]
        - 0.015329801272902543 * slot[130 - 1]
        - 0.015250383146005322 * slot[131 - 1]
        - 0.015161193901654432 * slot[132 - 1]
        - 0.01505876801398558 * slot[133 - 1]
        - 0.014982278084469176 * slot[134 - 1]
        - 0.014926475348737448 * slot[135 - 1]
        - 0.014856312469577537 * slot[136 - 1]
        - 0.014417427390756852 * slot[137 - 1]
        - 0.014382009994835026 * slot[138 - 1]
        - 0.014324548083927066 * slot[139 - 1]
        - 0.014285119603981896 * slot[140 - 1]
        - 0.013818180170188888 * slot[141 - 1]
        - 0.013774762997635075 * slot[142 - 1]
        - 0.013748826830771926 * slot[143 - 1]
        - 0.013726351243945743 * slot[144 - 1]
        - 0.013700826332939904 * slot[145 - 1]
        - 0.013690023737730584 * slot[146 - 1]
        - 0.013688340496091337 * slot[147 - 1]
        - 0.013661593083933797 * slot[148 - 1]
        - 0.013674159435582228 * slot[149 - 1]
        - 0.013664065391729182 * slot[150 - 1]
        - 0.01363770574486681 * slot[151 - 1]
        - 0.01363236751451939 * slot[152 - 1]
        - 0.013638417097531937 * slot[153 - 1]
        - 0.013625802485388793 * slot[154 - 1]
        - 0.013642480359164297 * slot[155 - 1]
        - 0.013662032671294783 * slot[156 - 1]
        - 0.013644888609735834 * slot[157 - 1]
        - 0.01327655732077232 * slot[158 - 1]
        - 0.013298831397534254 * slot[159 - 1]
        - 0.013291817348090715 * slot[160 - 1]
        - 0.013321822810710109 * slot[161 - 1]
        - 0.013357346302146056 * slot[162 - 1]
        - 0.013380420546464033 * slot[163 - 1]
        - 0.013435447699283443 * slot[164 - 1]
        - 0.013481519653608725 * slot[165 - 1]
        - 0.013515064911257426 * slot[166 - 1]
        - 0.013513017079263705 * slot[167 - 1]
        - 0.013548788158118786 * slot[168 - 1]
        - 0.013577558246194294 * slot[169 - 1]
        - 0.013586078924015461 * slot[170 - 1]
        - 0.013613630951083703 * slot[171 - 1]
        - 0.013654916005736007 * slot[172 - 1]
        - 0.013680384629178031 * slot[173 - 1]
        - 0.013721461032156674 * slot[174 - 1]
        - 0.013765650999681175 * slot[175 - 1]
        - 0.013764176544091627 * slot[176 - 1]
        - 0.013790826815348446 * slot[177 - 1]
        - 0.0138567580358286 * slot[178 - 1]
        - 0.013861781604822506 * slot[179 - 1]
        - 0.01390714497901981 * slot[180 - 1]
        - 0.013954338332318576 * slot[181 - 1]
        - 0.013952975076597016 * slot[182 - 1]
        - 0.014004880956229862 * slot[183 - 1]
        - 0.01404306512703514 * slot[184 - 1]
        - 0.01404981038930807 * slot[185 - 1]
        - 0.014083089612881761 * slot[186 - 1]
        - 0.014124291846081325 * slot[187 - 1]
        - 0.01407284869065143 * slot[188 - 1]
        - 0.014122031597214741 * slot[189 - 1]
        - 0.014157900564985105 * slot[190 - 1]
        - 0.014173567352941036 * slot[191 - 1]
        - 0.014195224836376252 * slot[192 - 1]
        - 0.014248711621419437 * slot[193 - 1]
        - 0.014261528418999721 * slot[194 - 1]
        - 0.014308281880788897 * slot[195 - 1]
        - 0.01435488719450908 * slot[196 - 1]
        - 0.014372938031715486 * slot[197 - 1]
        - 0.014438543016085851 * slot[198 - 1]
        - 0.014501025067312994 * slot[199 - 1]
        - 0.014544979932545483 * slot[200 - 1]
        - 0.014641844402375679 * slot[201 - 1]
        - 0.014681818881861796 * slot[202 - 1]
        - 0.01469342809429222 * slot[203 - 1]
        - 0.014751992977077513 * slot[204 - 1]
        - 0.014793506945542457 * slot[205 - 1]
        - 0.014966726098994838 * slot[206 - 1]
        - 0.014993145468306159 * slot[207 - 1]
        - 0.015057815100343173 * slot[208 - 1]
        - 0.015105755105826204 * slot[209 - 1]
        - 0.015152152019265211 * slot[210 - 1]
        - 0.015200820606651143 * slot[211 - 1]
        - 0.015255535679373433 * slot[212 - 1]
        - 0.015283203731642799 * slot[213 - 1]
        - 0.015319247545945842 * slot[214 - 1]
        - 0.015363822414108072 * slot[215 - 1]
        - 0.015374518341239674 * slot[216 - 1]
        - 0.015421873830264473 * slot[217 - 1]
        - 0.015456297043158049 * slot[218 - 1]
        - 0.015459079325866378 * slot[219 - 1]
        - 0.01552234637394955 * slot[220 - 1]
        - 0.01558408289050442 * slot[221 - 1]
        - 0.015580067995335156 * slot[222 - 1]
        - 0.015625733490713913 * slot[223 - 1]
        - 0.01567542866333932 * slot[224 - 1]
        - 0.015660103303530758 * slot[225 - 1]
        - 0.01572352111617812 * slot[226 - 1]
        - 0.015772524786875883 * slot[227 - 1]
        - 0.015762210540304303 * slot[228 - 1]
        - 0.015809404716208286 * slot[229 - 1]
        - 0.01583605573836272 * slot[230 - 1]
        - 0.015836177687403773 * slot[231 - 1]
        - 0.015870996268108947 * slot[232 - 1]
        - 0.01589910260856927 * slot[233 - 1]
        - 0.01588284533172545 * slot[234 - 1]
        - 0.015916868438380024 * slot[235 - 1]
        - 0.015925482785475616 * slot[236 - 1]
        - 0.015937586749807896 * slot[237 - 1]
        - 0.015953598450218136 * slot[238 - 1]
        - 0.015991524945247432 * slot[239 - 1]
        - 0.01597133186006477 * slot[240 - 1]
        - 0.015971856115420933 * slot[241 - 1]
        - 0.016008933046578525 * slot[242 - 1]
        - 0.015990059148612665 * slot[243 - 1]
        - 0.016028003251843965 * slot[244 - 1]
        - 0.016066177883864288 * slot[245 - 1]
        - 0.01610401382168215 * slot[246 - 1]
        - 0.01611116439819683 * slot[247 - 1]
        - 0.016145351650295958 * slot[248 - 1]
        - 0.01615876500949562 * slot[249 - 1]
        - 0.01613669301514437 * slot[250 - 1]
        - 0.016166654685116418 * slot[251 - 1]
        - 0.016215175949805566 * slot[252 - 1]
        - 0.016217144828187386 * slot[253 - 1]
        - 0.016253631448454035 * slot[254 - 1]
        - 0.016305356927121768 * slot[255 - 1]
        - 0.016260788722604835 * slot[256 - 1]
        - 0.016291956623291456 * slot[257 - 1]
        - 0.016321843220056125 * slot[258 - 1]
        - 0.01628556179762563 * slot[259 - 1]
        - 0.01630152866339144 * slot[260 - 1]
        - 0.0170015698831001 * slot[261 - 1]
        - 0.016966507993156285 * slot[262 - 1]
        - 0.017007634860795744 * slot[263 - 1]
        - 0.017013114744320712 * slot[264 - 1]
        - 0.016982572208924216 * slot[265 - 1]
        - 0.01702211774720593 * slot[266 - 1]
        - 0.01705322219114611 * slot[267 - 1]
        - 0.01703207701689882 * slot[268 - 1]
        - 0.017057617027573153 * slot[269 - 1]
        - 0.017675341799793406 * slot[270 - 1]
        - 0.017662288418849866 * slot[271 - 1]
        - 0.017648902405410175 * slot[272 - 1]
        - 0.018415232652522876 * slot[273 - 1]
        - 0.018407071317767764 * slot[274 - 1]
        - 0.01843540450421555 * slot[275 - 1]
        - 0.0184543345703936 * slot[276 - 1]
        - 0.018424110780059594 * slot[277 - 1]
        - 0.018442718890998192 * slot[278 - 1]
        - 0.018879134150006558 * slot[279 - 1]
        - 0.01881415711061039 * slot[280 - 1]
        - 0.018790801318233134 * slot[281 - 1]
        - 0.018820081560133754 * slot[282 - 1]
        - 0.018766196568539752 * slot[283 - 1]
        - 0.018747355451284867 * slot[284 - 1]
        - 0.018751182543925517 * slot[285 - 1]
        - 0.018696826366221136 * slot[286 - 1]
        - 0.0186925734784373 * slot[287 - 1]
        - 0.018675268465139623 * slot[288 - 1]
        - 0.019449014064244985 * slot[289 - 1]
        - 0.01942276899411006 * slot[290 - 1]
        - 0.01936541427349244 * slot[291 - 1]
        - 0.019360576914395154 * slot[292 - 1]
        - 0.019282020034533432 * slot[293 - 1]
        - 0.0192767771085092 * slot[294 - 1]
        - 0.019238196104347408 * slot[295 - 1]
        - 0.019191141597579547 * slot[296 - 1]
        - 0.019213888980332345 * slot[297 - 1]
        - 0.019200778556042124 * slot[298 - 1]
        - 0.01909786493820174 * slot[299 - 1]
        - 0.019058205221227798 * slot[300 - 1]
        - 0.01902115327300107 * slot[301 - 1]
        - 0.018920701349404608 * slot[302 - 1]
        - 0.018884167372047463 * slot[303 - 1]
        - 0.0188430477026652 * slot[304 - 1]
        - 0.018745496067448235 * slot[305 - 1]
        - 0.01873158981623399 * slot[306 - 1]
        - 0.018632370907563332 * slot[307 - 1]
        - 0.018534936418016533 * slot[308 - 1]
        - 0.018475744492725703 * slot[309 - 1]
        - 0.018422275615520966 * slot[310 - 1]
        - 0.018341425717928853 * slot[311 - 1]
        - 0.018295405112288412 * slot[312 - 1]
        - 0.018238207695049895 * slot[313 - 1]
        - 0.01813059303524592 * slot[314 - 1]
        - 0.018051858803249724 * slot[315 - 1]
        - 0.01799694464461973 * slot[316 - 1]
        - 0.017860830035317 * slot[317 - 1]
        - 0.01774417677928323 * slot[318 - 1]
        - 0.017693133230827586 * slot[319 - 1]
        - 0.017549565771701685 * slot[320 - 1]
        - 0.017434593073162825 * slot[321 - 1]
        - 0.01737170321605622 * slot[322 - 1]
        - 0.017230098639931135 * slot[323 - 1]
        - 0.017154816721735924 * slot[324 - 1]
        - 0.01704212041354589 * slot[325 - 1]
        - 0.016877165825990754 * slot[326 - 1]
        - 0.0168039662204337 * slot[327 - 1]
        - 0.01669249888577107 * slot[328 - 1]
        - 0.018368045896729794 * slot[329 - 1]
        - 0.018206232176436802 * slot[330 - 1]
        - 0.018081371064669043 * slot[331 - 1]
        - 0.017971826946400128 * slot[332 - 1]
        - 0.01774896295661779 * slot[333 - 1]
        - 0.017589361157102582 * slot[334 - 1]
        - 0.017491933984673812 * slot[335 - 1]
        - 0.02181250343298047 * slot[336 - 1]
        - 0.021682290770364255 * slot[337 - 1]
        - 0.021477487238609615 * slot[338 - 1]
        - 0.021280491850615032 * slot[339 - 1]
        - 0.021148742191834824 * slot[340 - 1]
        - 0.0209913314881646 * slot[341 - 1]
        - 0.020759690727983237 * slot[342 - 1]
        - 0.020618442356241884 * slot[343 - 1]
        - 0.020413480254652203 * slot[344 - 1]
        - 0.02021208606181211 * slot[345 - 1]
        - 0.020034826310004177 * slot[346 - 1]
        - 0.019825149147586877 * slot[347 - 1]
        - 0.019584778941381747 * slot[348 - 1]
        - 0.019350698390547306 * slot[349 - 1]
        - 0.01912242494067047 * slot[350 - 1]
        - 0.018862854574887265 * slot[351 - 1]
        - 0.018630517726706895 * slot[352 - 1]
        - 0.018459287840721293 * slot[353 - 1]
        - 0.018144871436121722 * slot[354 - 1]
        - 0.017936243760369404 * slot[355 - 1]
        - 0.017717797667357774 * slot[356 - 1]
        - 0.01741342510462024 * slot[357 - 1]
        - 0.0171387028907377 * slot[358 - 1]
        - 0.01689806926494717 * slot[359 - 1]
        - 0.016532557186838725 * slot[360 - 1]
        - 0.016316452492082783 * slot[361 - 1]
        - 0.015992167958895468 * slot[362 - 1]
        - 0.015649694731549195 * slot[363 - 1]
        - 0.015365777451358309 * slot[364 - 1]
        - 0.015103594818600638 * slot[365 - 1]
        - 0.01471595802260403 * slot[366 - 1]
        - 0.014445617019569153 * slot[367 - 1]
        - 0.014175656645654677 * slot[368 - 1]
        - 0.013904285900847899 * slot[369 - 1]
        - 0.013529010799380629 * slot[370 - 1]
        - 0.013181604686293799 * slot[371 - 1]
        - 0.012872545159596415 * slot[372 - 1]
        - 0.01245458668741084 * slot[373 - 1]
        - 0.012103746925424751 * slot[374 - 1]
        - 0.01876850527683017 * slot[375 - 1]
        - 0.01838765149737329 * slot[376 - 1]
        - 0.018061295571975645 * slot[377 - 1]
        - 0.0177219796595559 * slot[378 - 1]
        - 0.017315687254509583 * slot[379 - 1]
        - 0.017004045233786007 * slot[380 - 1]
        - 0.01667408965707264 * slot[381 - 1]
        - 0.01621809519385579 * slot[382 - 1]
        - 0.015853796970639555 * slot[383 - 1]
        - 0.015454924745825917 * slot[384 - 1]
        - 0.014988872791277228 * slot[385 - 1]
        - 0.014576339197943766 * slot[386 - 1]
        - 0.014167148974263952 * slot[387 - 1]
        - 0.013759446717702448 * slot[388 - 1]
        - 0.013345609160657449 * slot[389 - 1]
        - 0.012964839666079125 * slot[390 - 1]
        - 0.022964419135439226 * slot[391 - 1]
        - 0.02257507510793611 * slot[392 - 1]
        - 0.022197377047956605 * slot[393 - 1]
        - 0.021823727753279662 * slot[394 - 1]
        - 0.02142688876342791 * slot[395 - 1]
        - 0.021067027744311496 * slot[396 - 1]
        - 0.020574403511430004 * slot[397 - 1]
        - 0.020258396255962367 * slot[398 - 1]
        - 0.019938998670149255 * slot[399 - 1]
        - 0.019459157967173072 * slot[400 - 1]
        - 0.01845187204834128 * slot[401 - 1]
        - 0.01804259105475711 * slot[402 - 1]
        - 0.01762356128485341 * slot[403 - 1]
        - 0.017311000298506476 * slot[404 - 1]
        - 0.02969035355066874 * slot[405 - 1]
        - 0.02925074278886619 * slot[406 - 1]
        - 0.028926155920282557 * slot[407 - 1]
        - 0.02858160792518716 * slot[408 - 1]
        - 0.02814734827261804 * slot[409 - 1]
        - 0.02329978915763047 * slot[410 - 1]
        - 0.022954086556304727 * slot[411 - 1]
        - 0.02261124175918236 * slot[412 - 1]
        - 0.02212771619277849 * slot[413 - 1]
        - 0.02179851570686742 * slot[414 - 1]
        - 0.02142168304903878 * slot[415 - 1]
        - 0.020878090724441526 * slot[416 - 1]
        - 0.020457325195841432 * slot[417 - 1]
        - 0.02000009318047136 * slot[418 - 1]
        - 0.019491262013942953 * slot[419 - 1]
        - 0.018970600405346247 * slot[420 - 1]
        - 0.018434397326546684 * slot[421 - 1]
        - 0.01785265220828377 * slot[422 - 1]
        - 0.017307106318686143 * slot[423 - 1]
        - 0.016746695708267714 * slot[424 - 1]
        - 0.016066542388076168 * slot[425 - 1]
        - 0.01539586602882376 * slot[426 - 1]
        - 0.014812805428940158 * slot[427 - 1]
        - 0.014046052140430582 * slot[428 - 1]
        - 0.013252099878376711 * slot[429 - 1]
        - 0.012544803528913401 * slot[430 - 1]
        - 0.011753818325224801 * slot[431 - 1]
        - 0.011029200324888026 * slot[432 - 1]
        - 0.010209592853261415 * slot[433 - 1]
        - 0.04126253823553597 * slot[434 - 1]
        - 0.0405699857242855 * slot[435 - 1]
        - 0.039805841765220104 * slot[436 - 1]
        - 0.03895050038276486 * slot[437 - 1]
        - 0.03813419745304397 * slot[438 - 1]
        - 0.037404088727552444 * slot[439 - 1]
        - 0.020358334093735938 * slot[440 - 1]
        - 0.019488485530004037 * slot[441 - 1]
        - 0.01868354122099695 * slot[442 - 1]
        + 0.005725559073514994 * slot[443 - 1]
        + 0.006665272060557821 * slot[444 - 1]
        + 0.026825758303626497 * slot[445 - 1]
        + 0.02788958438864928 * slot[446 - 1]
        + 0.028964410638310845 * slot[447 - 1]
        + 0.02998148919493318 * slot[448 - 1]
        + 0.031046340638535248 * slot[449 - 1]
        + 0.008364142246702762 * slot[450 - 1]
        + 0.009266817139872268 * slot[451 - 1]
        + 0.010247968606240773 * slot[452 - 1]
        + 0.011303463701809777 * slot[453 - 1]
        + 0.012265882462329484 * slot[454 - 1]
        - 0.004959245388789642 * slot[455 - 1]
        - 0.00391713245405257 * slot[456 - 1]
        - 0.002992517212487379 * slot[457 - 1]
        + 0.021681086446408915 * slot[458 - 1]
        + 0.022736665462939704 * slot[459 - 1]
        + 0.023857873751043687 * slot[460 - 1]
        + 0.02491313323467117 * slot[461 - 1]
        + 0.026004153995706766 * slot[462 - 1]
        + 0.027047968103466127 * slot[463 - 1]
        + 0.028071096435303172 * slot[464 - 1]
        + 0.029093426865910658 * slot[465 - 1]
        + 0.03012838503151849 * slot[466 - 1]
        + 0.031254661283562066 * slot[467 - 1]
        + 0.03235756812529709 * slot[468 - 1]
        + 0.03333740211386057 * slot[469 - 1]
        + 0.00762142254547113 * slot[470 - 1]
        + 0.008673821512079233 * slot[471 - 1]
        + 0.009566793493814422 * slot[472 - 1]
        + 0.010420010688225283 * slot[473 - 1]
        + 0.011427954790187477 * slot[474 - 1]
        + 0.01232812249334748 * slot[475 - 1]
        + 0.013297872685894847 * slot[476 - 1]
        + 0.014354215446738058 * slot[477 - 1]
        + 0.015286785326806851 * slot[478 - 1]
        + 0.016232059385366062 * slot[479 - 1]
        + 0.017249446741018037 * slot[480 - 1]
        + 0.018154893409278443 * slot[481 - 1]
        + 0.019078674238162736 * slot[482 - 1]
        + 0.01997882814707411 * slot[483 - 1]
        - 0.06332327101935821 * slot[484 - 1]
        - 0.06270986209408942 * slot[485 - 1]
        - 0.062113763315484016 * slot[486 - 1]
        - 0.06165418499099187 * slot[487 - 1]
        - 0.06113282149937957 * slot[488 - 1]
        - 0.06058383347445784 * slot[489 - 1]
        - 0.060059556143701044 * slot[490 - 1]
        - 0.059525354032162615 * slot[491 - 1]
        - 0.059043321658017636 * slot[492 - 1]
        - 0.0585362230527993 * slot[493 - 1]
        - 0.05809699048035613 * slot[494 - 1]
        - 0.05760962002968678 * slot[495 - 1]
        - 0.05704616318376801 * slot[496 - 1]
        - 0.056652463188554596 * slot[497 - 1]
        - 0.056268204283209146 * slot[498 - 1]
        - 0.026532159774030437 * slot[499 - 1]
        - 0.018817472874959003 * slot[500 - 1]
        - 0.018275106442191203 * slot[501 - 1]
        - 0.017674019054166667 * slot[502 - 1]
        - 0.017222651923256383 * slot[503 - 1]
        - 0.016778344687882297 * slot[504 - 1]
        - 0.016152835872561323 * slot[505 - 1]
        - 0.01563050902042048 * slot[506 - 1]
        - 0.016085206162054163 * slot[507 - 1]
        - 0.015507181609270773 * slot[508 - 1]
        - 0.010936615588365515 * slot[509 - 1]
        - 0.010449826911165382 * slot[510 - 1]
        - 0.009827991105556358 * slot[511 - 1]
        - 0.009390565723569664 * slot[512 - 1]
        + 0.004309496802841238 * slot[513 - 1]
        + 0.02163545263953833 * slot[514 - 1]
        + 0.04156102406345851 * slot[515 - 1]
        + 0.05602941679496356 * slot[516 - 1]
        + 0.05704027201245976 * slot[517 - 1]
        + 0.05791163122521543 * slot[518 - 1]
        + 0.05890520148600055 * slot[519 - 1]
        + 0.060044711268437474 * slot[520 - 1]
        + 0.1023973347417498 * slot[521 - 1]
        + 0.10369851967501638 * slot[522 - 1]
        + 0.1052596470078202 * slot[523 - 1]
        + 0.10659102743686824 * slot[524 - 1]
        + 0.028398383319023165 * slot[525 - 1]
        - 0.025153398977824683 * slot[526 - 1]
        - 0.024320290207578373 * slot[527 - 1]
        - 0.023451897003179477 * slot[528 - 1]
        - 0.022284360506509455 * slot[529 - 1]
        - 0.021289344408643857 * slot[530 - 1]
        - 0.020446205589242764 * slot[531 - 1]
        - 0.01943248294984123 * slot[532 - 1]
        - 0.018268835737851 * slot[533 - 1]
        - 0.01728574478355284 * slot[534 - 1]
        - 0.01523695250745773 * slot[535 - 1]
        - 0.014040695951424164 * slot[536 - 1]
        - 0.012972594813359616 * slot[537 - 1]
        - 0.011668476540512951 * slot[538 - 1]
        - 0.010200377072912107 * slot[539 - 1]
        - 0.008899642107955836 * slot[540 - 1]
        - 0.007708014826629891 * slot[541 - 1]
        - 0.006258990622200419 * slot[542 - 1]
        - 0.00498182416228755 * slot[543 - 1]
        - 0.0038018024527292425 * slot[544 - 1]
        - 0.0023691502884824853 * slot[545 - 1]
        - 0.0009710099845324975 * slot[546 - 1]
        + 0.0006161133876372336 * slot[547 - 1]
        + 0.0027248232173826543 * slot[548 - 1]
        + 0.030073942602110486 * slot[549 - 1]
        + 0.031966945644777395 * slot[550 - 1]
        + 0.03376552863794307 * slot[551 - 1]
        + 0.03524623015952037 * slot[552 - 1]
        + 0.03677225853191452 * slot[553 - 1]
        + 0.0384707513288818 * slot[554 - 1]
        - 0.04137167644512138 * slot[555 - 1]
        - 0.040011584199274794 * slot[556 - 1]
        - 0.03858945198292927 * slot[557 - 1]
        - 0.03742725077143485 * slot[558 - 1]
        - 0.03620534205291128 * slot[559 - 1]
        - 0.03506170797034704 * slot[560 - 1]
        - 0.03384849145438292 * slot[561 - 1]
        - 0.03302324978946937 * slot[562 - 1]
        - 0.031707071897168516 * slot[563 - 1]
        - 0.030527127031660547 * slot[564 - 1]
        - 0.029716662122833054 * slot[565 - 1]
        - 0.0284299367373104 * slot[566 - 1]
        - 0.027381398496988184 * slot[567 - 1]
        - 0.026544647874289992 * slot[568 - 1]
        - 0.025208785529812427 * slot[569 - 1]
        - 0.024439966859246792 * slot[570 - 1]
        - 0.023724047483521367 * slot[571 - 1]
        - 0.02278857305029705 * slot[572 - 1]
        - 0.022049648781731487 * slot[573 - 1]
        - 0.02116426021530982 * slot[574 - 1]
        - 0.014172365336154525 * slot[575 - 1]
        - 0.013150495882454784 * slot[576 - 1]
        - 0.012304167243842385 * slot[577 - 1]
        - 0.01152399575987849 * slot[578 - 1]
        - 0.010369382036019491 * slot[579 - 1]
        - 0.009311151922842828 * slot[580 - 1]
        - 0.00845615563335879 * slot[581 - 1]
        - 0.00718949016484588 * slot[582 - 1]
        - 0.00638385550197781 * slot[583 - 1]
        - 0.005686142187543081 * slot[584 - 1]
        - 0.004631997095397443 * slot[585 - 1]
        - 0.003951077025748338 * slot[586 - 1]
        - 0.0034834974631212726 * slot[587 - 1]
        - 0.0025050026553743855 * slot[588 - 1]
        - 0.0016895765838651942 * slot[589 - 1]
        - 0.0010437941579304086 * slot[590 - 1]
        - 0.00017298979864309303 * slot[591 - 1]
        + 0.0005378393328380108 * slot[592 - 1]
        + 0.0011370383639922282 * slot[593 - 1]
        + 0.005924575578855084 * slot[594 - 1]
        + 0.006592678729437269 * slot[595 - 1]
        + 0.007423232698142431 * slot[596 - 1]
        + 0.008467377306937585 * slot[597 - 1]
        + 0.009040006242300764 * slot[598 - 1]
        + 0.009624888044729598 * slot[599 - 1]
        + 0.01034744022484825 * slot[600 - 1]
        + 0.010774563877628802 * slot[601 - 1]
        + 0.011253317065884795 * slot[602 - 1]
        + 0.011905140541327016 * slot[603 - 1]
        + 0.01237560613802635 * slot[604 - 1]
        + 0.012738623514692973 * slot[605 - 1]
        + 0.013383553843133176 * slot[606 - 1]
        + 0.013722208376521692 * slot[607 - 1]
        + 0.014184598812965011 * slot[608 - 1]
        + 0.014919093470152212 * slot[609 - 1]
        + 0.015215106599981066 * slot[610 - 1]
        + 0.015439605320350883 * slot[611 - 1]
        + 0.016096795716532393 * slot[612 - 1]
        + 0.016123800007497812 * slot[613 - 1]
        + 0.04296948382320204 * slot[614 - 1]
        + 0.04353646286787556 * slot[615 - 1]
        + 0.04426187893074381 * slot[616 - 1]
        + 0.044414372867104056 * slot[617 - 1]
        + 0.04463011496549732 * slot[618 - 1]
        + 0.045291593463136874 * slot[619 - 1]
        + 0.04540407781212238 * slot[620 - 1]
        + 0.04562803560565832 * slot[621 - 1]
        + 0.04633720326710179 * slot[622 - 1]
        + 0.0465065981951846 * slot[623 - 1]
        + 0.04652015793253747 * slot[624 - 1]
        - 0.016440990852386903 * slot[625 - 1]
        - 0.01672820455663664 * slot[626 - 1]
        - 0.017014888829876552 * slot[627 - 1]
        - 0.016975475400455655 * slot[628 - 1]
        - 0.017277619786761962 * slot[629 - 1]
        - 0.017217262370167078 * slot[630 - 1]
        - 0.016985573377010066 * slot[631 - 1]
        - 0.017114221863721413 * slot[632 - 1]
        - 0.017448133926974797 * slot[633 - 1]
        - 0.017487820219945414 * slot[634 - 1]
        - 0.01792030408885967 * slot[635 - 1]
        - 0.018021576164775625 * slot[636 - 1]
        - 0.017929350938423093 * slot[637 - 1]
        - 0.018102196210271072 * slot[638 - 1]
        - 0.0181892311203051 * slot[639 - 1]
        - 0.01796434408195584 * slot[640 - 1]
        - 0.018283688697818618 * slot[641 - 1]
        - 0.018675294253970513 * slot[642 - 1]
        - 0.018531102534209597 * slot[643 - 1]
        - 0.018690302854923218 * slot[644 - 1]
        - 0.018897789684952586 * slot[645 - 1]
        - 0.018684000782534183 * slot[646 - 1]
        - 0.01883661680943902 * slot[647 - 1]
        - 0.019366620501871205 * slot[648 - 1]
        - 0.01915341069321627 * slot[649 - 1]
        - 0.01942533431994682 * slot[650 - 1]
        - 0.019695422595970267 * slot[651 - 1]
        - 0.019683631675719362 * slot[652 - 1]
        - 0.019839564435721126 * slot[653 - 1]
        - 0.01981407308353821 * slot[654 - 1]
        - 0.019443575300032843 * slot[655 - 1]
        - 0.01981118530250124 * slot[656 - 1]
        - 0.019843409904467254 * slot[657 - 1]
        - 0.004936746473468678 * slot[658 - 1]
        - 0.004319060737251368 * slot[659 - 1]
        - 0.004189752693068423 * slot[660 - 1]
        - 0.004285523965973457 * slot[661 - 1]
        - 0.0035074611163782425 * slot[662 - 1]
        - 0.003413447634950524 * slot[663 - 1]
        - 0.003102826530029766 * slot[664 - 1]
        - 0.0022393733963738016 * slot[665 - 1]
        - 0.0020942978016850293 * slot[666 - 1]
        - 0.002148576442643202 * slot[667 - 1]
        - 0.0010695260506998563 * slot[668 - 1]
        - 0.0010571086555221513 * slot[669 - 1]
        - 0.0012622778848689702 * slot[670 - 1]
        - 0.0005592857321262967 * slot[671 - 1]
        - 0.0006783239586175151 * slot[672 - 1]
        - 0.0004506850780200155 * slot[673 - 1]
        + 0.000369994808744358 * slot[674 - 1]
        + 0.0005717583108349065 * slot[675 - 1]
        + 0.000928453938187194 * slot[676 - 1]
        + 0.0016841519472753546 * slot[677 - 1]
        + 0.0017430076232578236 * slot[678 - 1]
        + 0.0026754838616965797 * slot[679 - 1]
        + 0.003384194445638276 * slot[680 - 1]
        + 0.003066905092472076 * slot[681 - 1]
        + 0.0040273994857195706 * slot[682 - 1]
        + 0.004880147278913406 * slot[683 - 1]
        + 0.0045674190519300484 * slot[684 - 1]
        + 0.00562972011236491 * slot[685 - 1]
        + 0.006571309292137398 * slot[686 - 1]
        + 0.006359355666398876 * slot[687 - 1]
        + 0.007396396890280233 * slot[688 - 1]
        + 0.008643967501221709 * slot[689 - 1]
        + 0.00834568227386707 * slot[690 - 1]
        + 0.009414813448756044 * slot[691 - 1]
        + 0.0105953516038485 * slot[692 - 1]
        + 0.010244809033558948 * slot[693 - 1]
        + 0.01130750814368928 * slot[694 - 1]
        + 0.012219888770050228 * slot[695 - 1]
        + 0.01210752691915732 * slot[696 - 1]
        + 0.013110519710057052 * slot[697 - 1]
        + 0.01315555783839885 * slot[698 - 1]
        + 0.01421540890182509 * slot[699 - 1]
        + 0.015067717084009268 * slot[700 - 1]
        + 0.014848548223782678 * slot[701 - 1]
        + 0.016154111875828412 * slot[702 - 1]
        + 0.01729398688272513 * slot[703 - 1]
        + 0.017294893797184716 * slot[704 - 1]
        + 0.01844382697781403 * slot[705 - 1]
        + 0.019349740293408576 * slot[706 - 1]
        + 0.019269672040262423 * slot[707 - 1]
        + 0.02048506592634252 * slot[708 - 1]
        + 0.021310808707147917 * slot[709 - 1]
        + 0.021489883577434195 * slot[710 - 1]
        + 0.02269680363027479 * slot[711 - 1]
        + 0.024031046657686805 * slot[712 - 1]
        + 0.024050544742243742 * slot[713 - 1]
        + 0.02532517868824039 * slot[714 - 1]
        + 0.02675361056056312 * slot[715 - 1]
        + 0.026696663529746506 * slot[716 - 1]
        + 0.027856405381047965 * slot[717 - 1]
        + 0.029035820318312356 * slot[718 - 1]
        + 0.02853503494549509 * slot[719 - 1]
        + 0.029836856894939683 * slot[720 - 1]
        + 0.03106291224849938 * slot[721 - 1]
        + 0.031145692702787595 * slot[722 - 1]
        + 0.03261466874440878 * slot[723 - 1]
        + 0.03370186714974581 * slot[724 - 1]
        + 0.033516726405572794 * slot[725 - 1]
        + 0.03495482368253767 * slot[726 - 1]
        + 0.03645035570351501 * slot[727 - 1]
        + 0.03663106449250585 * slot[728 - 1]
        + 0.03783824829114726 * slot[729 - 1]
        + 0.03931473583780821 * slot[730 - 1]
        + 0.03947887178018049 * slot[731 - 1]
        + 0.04089608531595214 * slot[732 - 1]
        + 0.04218463788569691 * slot[733 - 1]
        + 0.04228452334921457 * slot[734 - 1]
        + 0.04386351849149407 * slot[735 - 1]
        + 0.045428451612202526 * slot[736 - 1]
        + 0.04547018271385082 * slot[737 - 1]
        + 0.046836943078430344 * slot[738 - 1]
        + 0.04801460259189707 * slot[739 - 1]
        + 0.04824790515015491 * slot[740 - 1]
        + 0.049632016608266646 * slot[741 - 1]
        + 0.0512111534051631 * slot[742 - 1]
        + 0.05134050131509978 * slot[743 - 1]
        + 0.052592714237240344 * slot[744 - 1]
        + 0.05399594867190159 * slot[745 - 1]
        + 0.05365079941483793 * slot[746 - 1]
        + 0.05451364622261233 * slot[747 - 1]
        + 0.05559795466808953 * slot[748 - 1]
        + 0.054924160714067506 * slot[749 - 1]
        + 0.055885792521959236 * slot[750 - 1]
        + 0.05656762257753188 * slot[751 - 1]
        + 0.05557500907924834 * slot[752 - 1]
        + 0.056131790796076865 * slot[753 - 1]
        - 0.0053702403662346096 * slot[754 - 1]
        - 0.006804313355570687 * slot[755 - 1]
        - 0.006330489076701784 * slot[756 - 1]
        - 0.005513714206002167 * slot[757 - 1]
        - 0.0060469984444107405 * slot[758 - 1]
        - 0.004940895725675288 * slot[759 - 1]
        - 0.0032769767915627993 * slot[760 - 1]
        - 0.0033970492587386664 * slot[761 - 1]
        - 0.0018897351694363972 * slot[762 - 1]
        + 0.00004883915375417048 * slot[763 - 1]
        + 0.00012529122027148522 * slot[764 - 1]
        + 0.0018059859298004855 * slot[765 - 1]
        + 0.0034085934056868705 * slot[766 - 1]
        + 0.0036390068866226986 * slot[767 - 1]
        + 0.005264331494745093 * slot[768 - 1]
        + 0.0069591132459848335 * slot[769 - 1]
        + 0.007149764972266053 * slot[770 - 1]
        + 0.00892598391814456 * slot[771 - 1]
        + 0.010795494197788759 * slot[772 - 1]
        + 0.010426917705261697 * slot[773 - 1]
        + 0.012095428962122168 * slot[774 - 1]
        + 0.01353286695019536 * slot[775 - 1]
        + 0.013092478444013062 * slot[776 - 1]
        + 0.014458382438101327 * slot[777 - 1]
        + 0.015535298597327232 * slot[778 - 1]
        + 0.01539515952432607 * slot[779 - 1]
        + 0.016333376816301013 * slot[780 - 1]
        + 0.016208887128516168 * slot[781 - 1]
        + 0.017154766541228515 * slot[782 - 1]
        + 0.01853092378048338 * slot[783 - 1]
        + 0.017765534264122137 * slot[784 - 1]
        + 0.01838995684775613 * slot[785 - 1]
        + 0.0196555593084948 * slot[786 - 1]
        + 0.01896270523932442 * slot[787 - 1]
        + 0.019802711234255244 * slot[788 - 1]
        + 0.020131019999366874 * slot[789 - 1]
        + 0.019194840027978263 * slot[790 - 1]
        + 0.01951248928020225 * slot[791 - 1]
        + 0.0194136162744745 * slot[792 - 1]
        + 0.018056567404069548 * slot[793 - 1]
        + 0.01809231564681792 * slot[794 - 1]
        + 0.018708104186536678 * slot[795 - 1]
        + 0.017664700928795348 * slot[796 - 1]
        + 0.01810308078196688 * slot[797 - 1]
        + 0.01796474003475607 * slot[798 - 1]
        + 0.01661003166059933 * slot[799 - 1]
        + 0.016416956392362018 * slot[800 - 1]
        + 0.016464059798147475 * slot[801 - 1]
        + 0.015999183732242458 * slot[802 - 1]
        + 0.016007920888305296 * slot[803 - 1]
        + 0.017267387947841592 * slot[804 - 1]
        + 0.01664472000474038 * slot[805 - 1]
        + 0.01844583980228659 * slot[806 - 1]
        + 0.01851640186933337 * slot[807 - 1]
        + 0.017569793215470305 * slot[808 - 1]
        + 0.02108246343143047 * slot[809 - 1]
        + 0.022307454464180386 * slot[810 - 1]
        + 0.021669741264502843 * slot[811 - 1]
        + 0.022212989586083212 * slot[812 - 1]
        + 0.023497143011617334 * slot[813 - 1]
        + 0.022000887711876 * slot[814 - 1]
        + 0.02206293650646957 * slot[815 - 1]
        + 0.02297793081380701 * slot[816 - 1]
        + 0.021437983969804113 * slot[817 - 1]
        + 0.02226832471812825 * slot[818 - 1]
        + 0.022436564878347828 * slot[819 - 1]
        + 0.021588760741071918 * slot[820 - 1]
        + 0.0211430117087345 * slot[821 - 1]
        + 0.022545895795923854 * slot[822 - 1]
        + 0.020598016529985892 * slot[823 - 1]
        + 0.021163775129093447 * slot[824 - 1]
        + 0.021922211983672953 * slot[825 - 1]
        + 0.02002895336895648 * slot[826 - 1]
        + 0.021697446345038153 * slot[827 - 1]
        + 0.02120275165344833 * slot[828 - 1]
        + 0.019629974326757237 * slot[829 - 1]
        + 0.019626096383321162 * slot[830 - 1]
        + 0.017669532943049484 * slot[831 - 1]
        + 0.018576939040624713 * slot[832 - 1]
        + 0.018573693115125765 * slot[833 - 1]
        + 0.021612366716256236 * slot[834 - 1]
        + 0.021405732900616974 * slot[835 - 1]
        + 0.024550795909238646 * slot[836 - 1]
        + 0.024957468239381126 * slot[837 - 1]
        + 0.022510850466250402 * slot[838 - 1]
        + 0.026204906206326997 * slot[839 - 1]
        + 0.024037391307561495 * slot[840 - 1]
        + 0.027381829936571532 * slot[841 - 1]
        + 0.027773713794672847 * slot[842 - 1]
        + 0.026490200529048703 * slot[843 - 1]
        + 0.025553771308321666 * slot[844 - 1]
        + 0.031519797163109035 * slot[845 - 1]
        + 0.0338248850851689 * slot[846 - 1]
        + 0.03094890646506021 * slot[847 - 1]
        + 0.03168206554424749 * slot[848 - 1]
        + 0.031449740564382274 * slot[849 - 1]
        + 0.03801869410780897 * slot[850 - 1]
        + 0.037893267662763526 * slot[851 - 1]
        + 0.03586121864042211 * slot[852 - 1]
        + 0.035709076819390694 * slot[853 - 1]
        + 0.03374211695436336 * slot[854 - 1]
        + 0.040311002300362206 * slot[855 - 1]
        + 0.036247147586642756 * slot[856 - 1]
        + 0.03590527438670009 * slot[857 - 1]
        + 0.03594014509277399 * slot[858 - 1]
        + 0.044058114942455893 * slot[859 - 1]
        + 0.04454648418078701 * slot[860 - 1]
        + 0.04146744126258562 * slot[861 - 1]
        + 0.04153676271498576 * slot[862 - 1]
        + 0.04202829834880763 * slot[863 - 1]
        + 0.04956243624119188 * slot[864 - 1]
        + 0.0488447325901608 * slot[865 - 1]
        + 0.04989368561031381 * slot[866 - 1]
        + 0.04663235420455996 * slot[867 - 1]
        + 0.05595435325108498 * slot[868 - 1]
        + 0.056034855699065786 * slot[869 - 1]
        + 0.051881188633841384 * slot[870 - 1]
        + 0.051719539245174626 * slot[871 - 1]
        + 0.04924264842454079 * slot[872 - 1]
        + 0.055070320565820875 * slot[873 - 1]
        + 0.05189660408721636 * slot[874 - 1]
        + 0.05038064219000611 * slot[875 - 1]
        + 0.0471898220758477 * slot[876 - 1]
        + 0.04533789052580071 * slot[877 - 1]
        + 0.05157725437237794 * slot[878 - 1]
        + 0.04581216741119821 * slot[879 - 1]
        + 0.04581550933002172 * slot[880 - 1]
        + 0.04122228478398429 * slot[881 - 1]
        + 0.046770170784434864 * slot[882 - 1]
        + 0.0411921919504787 * slot[883 - 1]
        + 0.03717859594687372 * slot[884 - 1]
        + 0.027950397088753513 * slot[885 - 1]
        + 0.02302979902230624 * slot[886 - 1]
        + 0.02561007780378495 * slot[887 - 1]
        + 0.015110114972739692 * slot[888 - 1]
        + 0.010678352549109375 * slot[889 - 1]
        + 0.002718247231206396 * slot[890 - 1]
        + 0.005217604438431459 * slot[891 - 1]
        - 0.005322532179089291 * slot[892 - 1]
        - 0.010340704625377336 * slot[893 - 1]
        - 0.019772835683109096 * slot[894 - 1]
        - 0.02527782489096696 * slot[895 - 1]
        - 0.02889215812494286 * slot[896 - 1]
        - 0.03638774596682971 * slot[897 - 1]
        - 0.04193909937101526 * slot[898 - 1]
        - 0.05330211087902772 * slot[899 - 1]
        - 0.06196369148882618 * slot[900 - 1]
        - 0.06739612422925548 * slot[901 - 1]
        - 0.07240429694896305 * slot[902 - 1]
        - 0.08606707993164577 * slot[903 - 1]
        - 0.09367098026469085 * slot[904 - 1]
        - 0.10040148686548915 * slot[905 - 1]
        - 0.1102398534940136 * slot[906 - 1]
        - 0.1179759541977259 * slot[907 - 1]
        - 0.13277987213714781 * slot[908 - 1]
        - 0.14120346955559382 * slot[909 - 1]
        - 0.14834719878594027 * slot[910 - 1]
        - 0.16135966061916007 * slot[911 - 1]
        - 0.18224442925432288 * slot[912 - 1]
        - 0.1933724309171789 * slot[913 - 1]
        - 0.20121692218390888 * slot[914 - 1]
        - 0.21276325749585714 * slot[915 - 1]
        - 0.2245759226021222 * slot[916 - 1]
        - 0.24211734129204693 * slot[917 - 1]
        - 0.255749610833907 * slot[918 - 1]
        - 0.26051414332276185 * slot[919 - 1]
        - 0.2719420696879652 * slot[920 - 1]
        - 0.3056333913472289 * slot[921 - 1]
        - 0.332020115150795 * slot[922 - 1]
        - 0.33151247921080546 * slot[923 - 1]
        - 0.3486695622704991 * slot[924 - 1]
        - 0.3572389140556036 * slot[925 - 1]
        - 0.4027085683472747 * slot[926 - 1]
        - 0.4160380798843801 * slot[927 - 1]
        - 0.3945958931429521 * slot[928 - 1]
        - 0.40528566928941584 * slot[929 - 1]
        - 0.4794285018566754 * slot[930 - 1]
        - 0.4882751600545394 * slot[931 - 1]
        - 0.5405652085845546 * slot[932 - 1]
        - 0.4531270915999745 * slot[933 - 1]
        - 0.4317174481201991 * slot[934 - 1]
        - 0.4967140278082152 * slot[935 - 1]
        - 0.5102604135880235 * slot[936 - 1]
        - 0.36940120044695074 * slot[937 - 1]
        - 0.343045311404214 * slot[938 - 1]
        - 0.43012910777426694 * slot[939 - 1]
        - 0.38795797997760634 * slot[940 - 1]
        - 0.47754028031887086 * slot[941 - 1]
        - 0.24915769734860305 * slot[942 - 1]
        - 0.19520458979321312 * slot[943 - 1]
        - 0.2818697573953122 * slot[944 - 1]
        - 0.284649787368602 * slot[945 - 1]
        - 0.11129903464974703 * slot[946 - 1]
        - 0.04444618037225443 * slot[947 - 1]
        - 0.1489037923835057 * slot[948 - 1]
        - 0.1513583133784608 * slot[949 - 1]
        - 0.23451845738791818 * slot[950 - 1]
        + 0.22671936516840904 * slot[951 - 1]
        + 0.23399748353794486 * slot[952 - 1]
        - 0.0013187146840808234 * slot[953 - 1]
        - 0.04140383004448797 * slot[954 - 1]
        - 0.19846568455297672 * slot[955 - 1]
        + 0.2421984503143543 * slot[956 - 1]
        - 0.006201780308534111 * slot[957 - 1]
        + 0.10804618274880609 * slot[958 - 1]
        - 0.1712326832759858 * slot[959 - 1]
        + 0.8074345447482016 * slot[960 - 1]
        + 1.140232497375317 * slot[961 - 1]
        + 0.9873827320920852 * slot[962 - 1]
        + 1.384600397354164 * slot[963 - 1]
        + 1.603687655011576 * slot[964 - 1]
        + 3.6115753581896133 * slot[965 - 1]
        + 4.398884687673004 * slot[966 - 1]
        + 5.297246638998754 * slot[967 - 1]
        + 5.423582984220182 * slot[968 - 1]
        + 7.531906663115844 * slot[969 - 1]
        + 8.162023747950025 * slot[970 - 1]
        + 8.161000858486796 * slot[971 - 1]
        + 9.086647153526885 * slot[972 - 1]
        + 8.686292701467995 * slot[973 - 1]
        + 9.714938219983406 * slot[974 - 1]
        + 9.70812333425954 * slot[975 - 1]
        + 10.445383427863245 * slot[976 - 1]
        + 9.732679940760601 * slot[977 - 1]
        + 9.386691575989046 * slot[978 - 1]
        + 10.863738848002264 * slot[979 - 1]
        + 11.060923877885735 * slot[980 - 1]
        + 10.905853015245615 * slot[981 - 1]
        + 10.452095128426338 * slot[982 - 1]
        + 11.049170219990232 * slot[983 - 1]
        + 10.975296010133214 * slot[984 - 1]
        + 10.574239692993034 * slot[985 - 1]
        + 9.915557410590035 * slot[986 - 1]
        + 10.332218939963893 * slot[987 - 1]
        + 11.574342538782165 * slot[988 - 1]
        + 9.987914341817227 * slot[989 - 1]
        + 9.488780708158147 * slot[990 - 1]
        + 8.971794477395179 * slot[991 - 1]
        + 9.298504571904191 * slot[992 - 1]
        + 9.472241258180313 * slot[993 - 1]
        + 7.747311036481196 * slot[994 - 1]
        + 7.610896072476914 * slot[995 - 1]
        + 7.507555970263573 * slot[996 - 1]
        + 7.933907381319066 * slot[997 - 1]
        + 7.546666297459183 * slot[998 - 1]
        + 5.788746450822852 * slot[999 - 1]
        + 5.963721288587236 * slot[1000 - 1]
        + 3.957352393781848 * slot[1001 - 1]
        + 3.9957832840005354 * slot[1002 - 1]
        + 3.1014386647154035 * slot[1003 - 1]
        + 1.5734117507834497 * slot[1004 - 1]
        - 1.1926984962652998 * slot[1005 - 1]
        + 1.333913830339177 * slot[1006 - 1]
        - 1.1532072932835504 * slot[1007 - 1]
        - 2.1294740429044454 * slot[1008 - 1]
        - 3.388805261681612 * slot[1009 - 1]
        - 8.33081312908391 * slot[1010 - 1]
        - 8.681525600686879 * slot[1011 - 1]
        - 14.929434079081867 * slot[1012 - 1]
        - 18.849384441428914 * slot[1013 - 1]
        - 24.89503011018147 * slot[1014 - 1]
        - 21.315238016927836 * slot[1015 - 1]
        - 26.48908117807192 * slot[1016 - 1]
        - 24.32948068019222 * slot[1017 - 1]
        - 23.163578148245396 * slot[1018 - 1]
        - 32.59089511797474 * slot[1019 - 1]
        - 12.478779338688108 * slot[1020 - 1]
        - 9.527446096473414 * slot[1021 - 1]
        - 27.698843636748823 * slot[1022 - 1]
        + 28.668144118076697 * slot[1023 - 1];

    let ratio = -528.6559213159951
        - 0.3952187184615387 * slot[1 - 1]
        - 0.15242912776421835 * slot[2 - 1]
        - 0.0528974695085134 * slot[3 - 1]
        - 0.03452412486594122 * slot[4 - 1]
        - 0.017071419018406787 * slot[5 - 1]
        - 0.0051057750456450545 * slot[6 - 1]
        - 0.005695303782085396 * slot[7 - 1]
        - 0.009830462718906998 * slot[8 - 1]
        - 0.01819139636014748 * slot[9 - 1]
        - 0.016249814769544707 * slot[10 - 1]
        - 0.014595746680884468 * slot[11 - 1]
        - 0.01472594959923547 * slot[12 - 1]
        - 0.016929634690056098 * slot[13 - 1]
        - 0.01820708324491947 * slot[14 - 1]
        - 0.01854475442665141 * slot[15 - 1]
        - 0.01819580521463235 * slot[16 - 1]
        - 0.011975870222283502 * slot[17 - 1]
        - 0.011400958605662037 * slot[18 - 1]
        - 0.011286499204137832 * slot[19 - 1]
        - 0.011370140292731343 * slot[20 - 1]
        - 0.01145685959853933 * slot[21 - 1]
        - 0.011087076415608063 * slot[22 - 1]
        - 0.007428096411274721 * slot[23 - 1]
        - 0.005626629627552265 * slot[24 - 1]
        - 0.004519956161109149 * slot[25 - 1]
        - 0.003440533010064743 * slot[26 - 1]
        + 0.0008824921140098245 * slot[27 - 1]
        + 0.002139344502168074 * slot[28 - 1]
        + 0.0034976313829825534 * slot[29 - 1]
        + 0.005012745905385267 * slot[30 - 1]
        + 0.006738229409992884 * slot[31 - 1]
        + 0.008458964717240848 * slot[32 - 1]
        + 0.010219137356832607 * slot[33 - 1]
        + 0.011966004598001685 * slot[34 - 1]
        + 0.013856871314278497 * slot[35 - 1]
        + 0.01605378896093887 * slot[36 - 1]
        + 0.0183333738820266 * slot[37 - 1]
        + 0.022840209026061323 * slot[38 - 1]
        + 0.0251730634452673 * slot[39 - 1]
        + 0.02706352404895949 * slot[40 - 1]
        + 0.028745744601484817 * slot[41 - 1]
        + 0.03010080097385591 * slot[42 - 1]
        + 0.030278069009623442 * slot[43 - 1]
        + 0.030140197080165156 * slot[44 - 1]
        + 0.029868146762386402 * slot[45 - 1]
        + 0.02957613599497762 * slot[46 - 1]
        + 0.029266054012696087 * slot[47 - 1]
        + 0.010276810075799198 * slot[48 - 1]
        + 0.009979750636986882 * slot[49 - 1]
        + 0.00983141562572461 * slot[50 - 1]
        + 0.009922004855597999 * slot[51 - 1]
        + 0.010053342805952907 * slot[52 - 1]
        + 0.01043636944787818 * slot[53 - 1]
        + 0.018083746596112858 * slot[54 - 1]
        + 0.01868791765052243 * slot[55 - 1]
        + 0.019389810518036087 * slot[56 - 1]
        + 0.020160693083722554 * slot[57 - 1]
        + 0.02104479423813558 * slot[58 - 1]
        + 0.021899292241587737 * slot[59 - 1]
        + 0.022733197536328163 * slot[60 - 1]
        + 0.03899988297917229 * slot[61 - 1]
        + 0.03991790827910631 * slot[62 - 1]
        + 0.04085654341238078 * slot[63 - 1]
        + 0.04181491670417505 * slot[64 - 1]
        + 0.04265269107583741 * slot[65 - 1]
        + 0.043376703543251446 * slot[66 - 1]
        + 0.04416394177974087 * slot[67 - 1]
        + 0.04483963065485313 * slot[68 - 1]
        + 0.04543603195393588 * slot[69 - 1]
        + 0.04596547103830681 * slot[70 - 1]
        + 0.04637348355196834 * slot[71 - 1]
        + 0.04674431545014404 * slot[72 - 1]
        + 0.033612862023038856 * slot[73 - 1]
        + 0.0339515770536779 * slot[74 - 1]
        + 0.034134796427982485 * slot[75 - 1]
        + 0.03435379645799119 * slot[76 - 1]
        + 0.034532516678259725 * slot[77 - 1]
        + 0.03462872541546938 * slot[78 - 1]
        + 0.03462040860277862 * slot[79 - 1]
        + 0.03457751147202277 * slot[80 - 1]
        + 0.034274356009410145 * slot[81 - 1]
        + 0.03378277295746939 * slot[82 - 1]
        + 0.03320567226832566 * slot[83 - 1]
        + 0.032546881497849206 * slot[84 - 1]
        + 0.031700523909139966 * slot[85 - 1]
        + 0.03088096702600423 * slot[86 - 1]
        + 0.030082912867521933 * slot[87 - 1]
        + 0.029291155330207486 * slot[88 - 1]
        + 0.02858335932735813 * slot[89 - 1]
        + 0.02793793611827016 * slot[90 - 1]
        + 0.02737449999047574 * slot[91 - 1]
        + 0.027061692223352297 * slot[92 - 1]
        + 0.02715970554960741 * slot[93 - 1]
        + 0.0288462561658815 * slot[94 - 1]
        + 0.03029665786347863 * slot[95 - 1]
        + 0.02633949863810136 * slot[96 - 1]
        + 0.0264782256412394 * slot[97 - 1]
        + 0.026452089717127415 * slot[98 - 1]
        + 0.02647217881825229 * slot[99 - 1]
        + 0.026470349962475215 * slot[100 - 1]
        + 0.026473722584261745 * slot[101 - 1]
        + 0.02644295886677903 * slot[102 - 1]
        + 0.026384920836533536 * slot[103 - 1]
        + 0.026390633258449314 * slot[104 - 1]
        + 0.026397531006070805 * slot[105 - 1]
        + 0.02641031205257559 * slot[106 - 1]
        - 0.00812295234754771 * slot[107 - 1]
        - 0.008181929368974118 * slot[108 - 1]
        - 0.008174056517072951 * slot[109 - 1]
        - 0.008117692242161585 * slot[110 - 1]
        - 0.008052194498880873 * slot[111 - 1]
        - 0.007950842272746568 * slot[112 - 1]
        - 0.007840210866433037 * slot[113 - 1]
        - 0.007729654220555899 * slot[114 - 1]
        - 0.007566441486833458 * slot[115 - 1]
        - 0.007497233224420643 * slot[116 - 1]
        - 0.007350632395439853 * slot[117 - 1]
        - 0.007276032116268478 * slot[118 - 1]
        - 0.0072088429314127395 * slot[119 - 1]
        - 0.0071655873155339386 * slot[120 - 1]
        - 0.007124624755152053 * slot[121 - 1]
        - 0.007077644772397731 * slot[122 - 1]
        - 0.007058258368906904 * slot[123 - 1]
        - 0.007055632410611439 * slot[124 - 1]
        - 0.007050299398873935 * slot[125 - 1]
        - 0.007002594174449689 * slot[126 - 1]
        - 0.0069948879281282305 * slot[127 - 1]
        - 0.006909040868568303 * slot[128 - 1]
        - 0.0068594720046376585 * slot[129 - 1]
        - 0.006803406686026474 * slot[130 - 1]
        - 0.006706712527036734 * slot[131 - 1]
        - 0.006627529118963343 * slot[132 - 1]
        - 0.006524411867397967 * slot[133 - 1]
        - 0.006452559640867687 * slot[134 - 1]
        - 0.006313160428075637 * slot[135 - 1]
        - 0.006295060242994386 * slot[136 - 1]
        - 0.010482297099485336 * slot[137 - 1]
        - 0.010430674947885103 * slot[138 - 1]
        - 0.010346820553452278 * slot[139 - 1]
        - 0.01025958898660952 * slot[140 - 1]
        - 0.015257748848753389 * slot[141 - 1]
        - 0.015283704637597508 * slot[142 - 1]
        - 0.01520338225679581 * slot[143 - 1]
        - 0.015049022943458553 * slot[144 - 1]
        - 0.014874069492720361 * slot[145 - 1]
        - 0.014669571536893503 * slot[146 - 1]
        - 0.014524093846752724 * slot[147 - 1]
        - 0.014287121221925228 * slot[148 - 1]
        - 0.014028011832009597 * slot[149 - 1]
        - 0.013788607008663863 * slot[150 - 1]
        - 0.013486863435880911 * slot[151 - 1]
        - 0.013123846861527512 * slot[152 - 1]
        - 0.012836736490357941 * slot[153 - 1]
        - 0.012425875161231696 * slot[154 - 1]
        - 0.012078934342854685 * slot[155 - 1]
        - 0.011683777531098737 * slot[156 - 1]
        - 0.011308610629388946 * slot[157 - 1]
        + 0.016569507043261284 * slot[158 - 1]
        + 0.016928920630020293 * slot[159 - 1]
        + 0.01723698063314205 * slot[160 - 1]
        + 0.01756402767776213 * slot[161 - 1]
        + 0.01793350350908637 * slot[162 - 1]
        + 0.01826752540245713 * slot[163 - 1]
        + 0.018572141678605204 * slot[164 - 1]
        + 0.018921929321561528 * slot[165 - 1]
        + 0.019209780090122027 * slot[166 - 1]
        + 0.019508426274069145 * slot[167 - 1]
        + 0.01975463294885468 * slot[168 - 1]
        + 0.020053164155071013 * slot[169 - 1]
        + 0.020412038290703417 * slot[170 - 1]
        + 0.02068686018298158 * slot[171 - 1]
        + 0.02094501878945714 * slot[172 - 1]
        + 0.02116957293615326 * slot[173 - 1]
        + 0.021402377391893632 * slot[174 - 1]
        + 0.021553282086522237 * slot[175 - 1]
        + 0.021815367873134907 * slot[176 - 1]
        + 0.02197672561554493 * slot[177 - 1]
        + 0.022160369316942347 * slot[178 - 1]
        + 0.02234356975844893 * slot[179 - 1]
        + 0.022485934357729984 * slot[180 - 1]
        + 0.02255767296759449 * slot[181 - 1]
        + 0.02271956373676945 * slot[182 - 1]
        + 0.02285767274813103 * slot[183 - 1]
        + 0.0229706738725498 * slot[184 - 1]
        + 0.02309612813822922 * slot[185 - 1]
        + 0.023152203098823047 * slot[186 - 1]
        + 0.023224701958610053 * slot[187 - 1]
        + 0.020814081946594183 * slot[188 - 1]
        + 0.02089156747533264 * slot[189 - 1]
        + 0.020973482368897997 * slot[190 - 1]
        + 0.02103679340161029 * slot[191 - 1]
        + 0.02110180405206216 * slot[192 - 1]
        + 0.021151272777780682 * slot[193 - 1]
        + 0.02120672398509916 * slot[194 - 1]
        + 0.02128883050239205 * slot[195 - 1]
        + 0.021359735431827384 * slot[196 - 1]
        + 0.02138254472854191 * slot[197 - 1]
        + 0.02139302584653641 * slot[198 - 1]
        + 0.021391937992116868 * slot[199 - 1]
        + 0.021356148289816405 * slot[200 - 1]
        + 0.02127979837441805 * slot[201 - 1]
        + 0.02124954915976053 * slot[202 - 1]
        + 0.021220564825173975 * slot[203 - 1]
        + 0.021241306702231358 * slot[204 - 1]
        + 0.021224991845355045 * slot[205 - 1]
        + 0.020009466757223867 * slot[206 - 1]
        + 0.019947496358668795 * slot[207 - 1]
        + 0.019937584518179226 * slot[208 - 1]
        + 0.019969751310082818 * slot[209 - 1]
        + 0.019894393422671106 * slot[210 - 1]
        + 0.019877789048278933 * slot[211 - 1]
        + 0.019860214128583336 * slot[212 - 1]
        + 0.01985566203268286 * slot[213 - 1]
        + 0.01986408018283633 * slot[214 - 1]
        + 0.019916140884116713 * slot[215 - 1]
        + 0.019945389162306235 * slot[216 - 1]
        + 0.01994657677404306 * slot[217 - 1]
        + 0.01997351777734055 * slot[218 - 1]
        + 0.019974575243114478 * slot[219 - 1]
        + 0.01993757065574953 * slot[220 - 1]
        + 0.019917364241754894 * slot[221 - 1]
        + 0.0199506808319437 * slot[222 - 1]
        + 0.019954018553607122 * slot[223 - 1]
        + 0.019920536329183867 * slot[224 - 1]
        + 0.019973434200089596 * slot[225 - 1]
        + 0.01999386249707205 * slot[226 - 1]
        + 0.019981190091222685 * slot[227 - 1]
        + 0.01996930057272622 * slot[228 - 1]
        + 0.019981761733131972 * slot[229 - 1]
        + 0.0199965128679732 * slot[230 - 1]
        + 0.02006589981542677 * slot[231 - 1]
        + 0.020090075384295126 * slot[232 - 1]
        + 0.02008950786451476 * slot[233 - 1]
        + 0.020052514595351578 * slot[234 - 1]
        + 0.020044132534913547 * slot[235 - 1]
        + 0.020016878539147773 * slot[236 - 1]
        + 0.019984686600094552 * slot[237 - 1]
        + 0.019956341538933837 * slot[238 - 1]
        + 0.019942605599213063 * slot[239 - 1]
        + 0.019960497604130784 * slot[240 - 1]
        + 0.019986243895515864 * slot[241 - 1]
        + 0.019979707716867537 * slot[242 - 1]
        + 0.019977875104802555 * slot[243 - 1]
        + 0.019931690648571976 * slot[244 - 1]
        + 0.019888489122557928 * slot[245 - 1]
        + 0.0198632253343507 * slot[246 - 1]
        + 0.019833587904261176 * slot[247 - 1]
        + 0.019822065158852337 * slot[248 - 1]
        + 0.01979214306480105 * slot[249 - 1]
        + 0.019758947732040955 * slot[250 - 1]
        + 0.019725772252451035 * slot[251 - 1]
        + 0.019647618366594004 * slot[252 - 1]
        + 0.019612175588392337 * slot[253 - 1]
        + 0.019540227575615535 * slot[254 - 1]
        + 0.019455508506440847 * slot[255 - 1]
        + 0.01935948235840506 * slot[256 - 1]
        + 0.01931780346875997 * slot[257 - 1]
        + 0.019278396290541337 * slot[258 - 1]
        + 0.019258452748407328 * slot[259 - 1]
        + 0.01918342590731498 * slot[260 - 1]
        + 0.01864663516453329 * slot[261 - 1]
        + 0.018599185925947778 * slot[262 - 1]
        + 0.018506312333645025 * slot[263 - 1]
        + 0.018465073848180094 * slot[264 - 1]
        + 0.018410644081355898 * slot[265 - 1]
        + 0.01836047761349164 * slot[266 - 1]
        + 0.018309978175754634 * slot[267 - 1]
        + 0.018254230777764262 * slot[268 - 1]
        + 0.018171340259367726 * slot[269 - 1]
        + 0.017679356204589186 * slot[270 - 1]
        + 0.017624251588159154 * slot[271 - 1]
        + 0.017575458627158935 * slot[272 - 1]
        + 0.01717477822146165 * slot[273 - 1]
        + 0.0171008721530112 * slot[274 - 1]
        + 0.017005868804953524 * slot[275 - 1]
        + 0.016984033780760903 * slot[276 - 1]
        + 0.016896759405878897 * slot[277 - 1]
        + 0.016787963326073223 * slot[278 - 1]
        + 0.016579353644535832 * slot[279 - 1]
        + 0.01652594003944395 * slot[280 - 1]
        + 0.01645007571157261 * slot[281 - 1]
        + 0.01636062167455232 * slot[282 - 1]
        + 0.016285959662697243 * slot[283 - 1]
        + 0.01622646527010344 * slot[284 - 1]
        + 0.01619169137431429 * slot[285 - 1]
        + 0.0161235058180409 * slot[286 - 1]
        + 0.016061295063008275 * slot[287 - 1]
        + 0.016013463603765328 * slot[288 - 1]
        + 0.015962267783240554 * slot[289 - 1]
        + 0.01586935678076567 * slot[290 - 1]
        + 0.01580049257940755 * slot[291 - 1]
        + 0.015744854028428068 * slot[292 - 1]
        + 0.015701453744165712 * slot[293 - 1]
        + 0.015588382541190358 * slot[294 - 1]
        + 0.015521870924594162 * slot[295 - 1]
        + 0.015429758456840203 * slot[296 - 1]
        + 0.015332457097978671 * slot[297 - 1]
        + 0.015291193714927919 * slot[298 - 1]
        + 0.015211815075815871 * slot[299 - 1]
        + 0.015159779888965622 * slot[300 - 1]
        + 0.01511966670941491 * slot[301 - 1]
        + 0.015066458979169051 * slot[302 - 1]
        + 0.015026532112015052 * slot[303 - 1]
        + 0.014980825629529465 * slot[304 - 1]
        + 0.014909866349613827 * slot[305 - 1]
        + 0.014822317678487726 * slot[306 - 1]
        + 0.014787898545945958 * slot[307 - 1]
        + 0.014729696733676982 * slot[308 - 1]
        + 0.01469139654932552 * slot[309 - 1]
        + 0.01461849049464824 * slot[310 - 1]
        + 0.01455475268424115 * slot[311 - 1]
        + 0.014496644676927564 * slot[312 - 1]
        + 0.014386826944299767 * slot[313 - 1]
        + 0.014351365561332992 * slot[314 - 1]
        + 0.014331349147617887 * slot[315 - 1]
        + 0.014286437555647843 * slot[316 - 1]
        + 0.014263261128823207 * slot[317 - 1]
        + 0.01422329475419422 * slot[318 - 1]
        + 0.01414204868824279 * slot[319 - 1]
        + 0.014110941962830291 * slot[320 - 1]
        + 0.01405873143774471 * slot[321 - 1]
        + 0.013992446753030753 * slot[322 - 1]
        + 0.013933033571974741 * slot[323 - 1]
        + 0.013857098924170294 * slot[324 - 1]
        + 0.0138272135390006 * slot[325 - 1]
        + 0.013742619927906251 * slot[326 - 1]
        + 0.013657369562302431 * slot[327 - 1]
        + 0.013555435514808705 * slot[328 - 1]
        + 0.013313754208009851 * slot[329 - 1]
        + 0.013229904829067256 * slot[330 - 1]
        + 0.013192520336464557 * slot[331 - 1]
        + 0.013100776060917039 * slot[332 - 1]
        + 0.013040650929903455 * slot[333 - 1]
        + 0.012992482987303728 * slot[334 - 1]
        + 0.012916177325198713 * slot[335 - 1]
        + 0.012819545634305889 * slot[336 - 1]
        + 0.012753299936617725 * slot[337 - 1]
        + 0.012724935621261698 * slot[338 - 1]
        + 0.012659669993326269 * slot[339 - 1]
        + 0.01258060030506851 * slot[340 - 1]
        + 0.012535904644903695 * slot[341 - 1]
        + 0.012441545298685233 * slot[342 - 1]
        + 0.012349331025646278 * slot[343 - 1]
        + 0.01229132376948043 * slot[344 - 1]
        + 0.012258660083189885 * slot[345 - 1]
        + 0.012209549656238997 * slot[346 - 1]
        + 0.012190988352860312 * slot[347 - 1]
        + 0.012154275475487463 * slot[348 - 1]
        + 0.012096265609751803 * slot[349 - 1]
        + 0.012036447123938868 * slot[350 - 1]
        + 0.012008425565649758 * slot[351 - 1]
        + 0.011943483131559986 * slot[352 - 1]
        + 0.011874453409162981 * slot[353 - 1]
        + 0.011812361512173816 * slot[354 - 1]
        + 0.011763565291745855 * slot[355 - 1]
        + 0.011708739955742663 * slot[356 - 1]
        + 0.011649652673333837 * slot[357 - 1]
        + 0.011628806582066185 * slot[358 - 1]
        + 0.011622750626547738 * slot[359 - 1]
        + 0.011560508317169867 * slot[360 - 1]
        + 0.011509556092057352 * slot[361 - 1]
        + 0.011475179995365948 * slot[362 - 1]
        + 0.011427035490389668 * slot[363 - 1]
        + 0.011430184787204522 * slot[364 - 1]
        + 0.01138745572181494 * slot[365 - 1]
        + 0.011359795928634338 * slot[366 - 1]
        + 0.01130018961946655 * slot[367 - 1]
        + 0.011220973513235866 * slot[368 - 1]
        + 0.01117245391708586 * slot[369 - 1]
        + 0.011112173713189908 * slot[370 - 1]
        + 0.011092721772579446 * slot[371 - 1]
        + 0.011071899535317434 * slot[372 - 1]
        + 0.011003889355228972 * slot[373 - 1]
        + 0.010977349969379761 * slot[374 - 1]
        + 0.0109173626862205 * slot[375 - 1]
        + 0.010880415465139556 * slot[376 - 1]
        + 0.010855404237467343 * slot[377 - 1]
        + 0.01081197907802654 * slot[378 - 1]
        + 0.010730572636641395 * slot[379 - 1]
        + 0.010704683177895882 * slot[380 - 1]
        + 0.010669376124875264 * slot[381 - 1]
        + 0.010642570013493884 * slot[382 - 1]
        + 0.010625039021408827 * slot[383 - 1]
        + 0.01055826295571372 * slot[384 - 1]
        + 0.010516760004643994 * slot[385 - 1]
        + 0.010489454862953915 * slot[386 - 1]
        + 0.010459004580400742 * slot[387 - 1]
        + 0.01037502168555718 * slot[388 - 1]
        + 0.01034632254628242 * slot[389 - 1]
        + 0.010292025002791597 * slot[390 - 1]
        + 0.010522853052351257 * slot[391 - 1]
        + 0.010504757369411487 * slot[392 - 1]
        + 0.010469205664428603 * slot[393 - 1]
        + 0.0103542063301738 * slot[394 - 1]
        + 0.010330756258013303 * slot[395 - 1]
        + 0.010299839259037815 * slot[396 - 1]
        + 0.010275243547195083 * slot[397 - 1]
        + 0.010250843874845164 * slot[398 - 1]
        + 0.010206892027836574 * slot[399 - 1]
        + 0.010200451193070743 * slot[400 - 1]
        + 0.010139017313145647 * slot[401 - 1]
        + 0.010127283727643195 * slot[402 - 1]
        + 0.010106247408261652 * slot[403 - 1]
        + 0.010067283303193142 * slot[404 - 1]
        + 0.010172949821925629 * slot[405 - 1]
        + 0.010154685352349441 * slot[406 - 1]
        + 0.010128515722090678 * slot[407 - 1]
        + 0.010115818942815361 * slot[408 - 1]
        + 0.01005067095051904 * slot[409 - 1]
        + 0.010019254455631645 * slot[410 - 1]
        + 0.009982161574206977 * slot[411 - 1]
        + 0.009906245260865535 * slot[412 - 1]
        + 0.009867273141446097 * slot[413 - 1]
        + 0.009788775550471416 * slot[414 - 1]
        + 0.009705734986667859 * slot[415 - 1]
        + 0.00968814698061291 * slot[416 - 1]
        + 0.009688390272376373 * slot[417 - 1]
        + 0.009668864356583875 * slot[418 - 1]
        + 0.00957936625103748 * slot[419 - 1]
        + 0.009590001782920382 * slot[420 - 1]
        + 0.009526652734645485 * slot[421 - 1]
        + 0.009494908837856395 * slot[422 - 1]
        + 0.009424828961136089 * slot[423 - 1]
        + 0.009331772849724522 * slot[424 - 1]
        + 0.009285181926043272 * slot[425 - 1]
        + 0.009247407090500969 * slot[426 - 1]
        + 0.009144960917885366 * slot[427 - 1]
        + 0.009079498175430032 * slot[428 - 1]
        + 0.00907299428466183 * slot[429 - 1]
        + 0.009025519524094751 * slot[430 - 1]
        + 0.008957724434883933 * slot[431 - 1]
        + 0.008929404337270849 * slot[432 - 1]
        + 0.00888902966668188 * slot[433 - 1]
        + 0.009437239778719428 * slot[434 - 1]
        + 0.009400796522547893 * slot[435 - 1]
        + 0.009390240344384166 * slot[436 - 1]
        + 0.009323338107474868 * slot[437 - 1]
        + 0.009265926755538148 * slot[438 - 1]
        + 0.009157056222223193 * slot[439 - 1]
        + 0.009016668686934121 * slot[440 - 1]
        + 0.008927465917425 * slot[441 - 1]
        + 0.00881709200696994 * slot[442 - 1]
        + 0.008860723299175362 * slot[443 - 1]
        + 0.008804108301006565 * slot[444 - 1]
        + 0.008817413624922875 * slot[445 - 1]
        + 0.00875211761580289 * slot[446 - 1]
        + 0.008729118524385562 * slot[447 - 1]
        + 0.008688843848175434 * slot[448 - 1]
        + 0.008614863164647825 * slot[449 - 1]
        + 0.008497951363804082 * slot[450 - 1]
        + 0.008445337889997882 * slot[451 - 1]
        + 0.008392757114342063 * slot[452 - 1]
        + 0.008350301067018144 * slot[453 - 1]
        + 0.008301904883204952 * slot[454 - 1]
        + 0.008414694088143607 * slot[455 - 1]
        + 0.008375573925884202 * slot[456 - 1]
        + 0.008340349541633369 * slot[457 - 1]
        + 0.008327167227803252 * slot[458 - 1]
        + 0.008257222789192374 * slot[459 - 1]
        + 0.00828545259478108 * slot[460 - 1]
        + 0.008248299580114585 * slot[461 - 1]
        + 0.008219679576683474 * slot[462 - 1]
        + 0.008223447960875367 * slot[463 - 1]
        + 0.008172579853796795 * slot[464 - 1]
        + 0.008113410116404618 * slot[465 - 1]
        + 0.008097035449147229 * slot[466 - 1]
        + 0.008078660246665342 * slot[467 - 1]
        + 0.008034409808452027 * slot[468 - 1]
        + 0.008007622858409013 * slot[469 - 1]
        + 0.008768747134849498 * slot[470 - 1]
        + 0.008740149461584244 * slot[471 - 1]
        + 0.008687538709414792 * slot[472 - 1]
        + 0.008651994925550879 * slot[473 - 1]
        + 0.008545565647899787 * slot[474 - 1]
        + 0.008527013658648977 * slot[475 - 1]
        + 0.008424552606562387 * slot[476 - 1]
        + 0.008367781622513972 * slot[477 - 1]
        + 0.008311356149557048 * slot[478 - 1]
        + 0.008257379386476171 * slot[479 - 1]
        + 0.008199796029125727 * slot[480 - 1]
        + 0.008106664299132121 * slot[481 - 1]
        + 0.00800766688869395 * slot[482 - 1]
        + 0.00783175765659425 * slot[483 - 1]
        + 0.010605593739047705 * slot[484 - 1]
        + 0.010539945807231912 * slot[485 - 1]
        + 0.010410438104146904 * slot[486 - 1]
        + 0.010280891987885732 * slot[487 - 1]
        + 0.010215367100848619 * slot[488 - 1]
        + 0.01017494073317648 * slot[489 - 1]
        + 0.010091156954728373 * slot[490 - 1]
        + 0.010063003943344456 * slot[491 - 1]
        + 0.01004923214938114 * slot[492 - 1]
        + 0.009900089093454278 * slot[493 - 1]
        + 0.00979172455251042 * slot[494 - 1]
        + 0.00973718576407282 * slot[495 - 1]
        + 0.009721122179631253 * slot[496 - 1]
        + 0.009645927557143712 * slot[497 - 1]
        + 0.009532459048988052 * slot[498 - 1]
        + 0.00938562576026097 * slot[499 - 1]
        + 0.00927609379480281 * slot[500 - 1]
        + 0.009177185658123145 * slot[501 - 1]
        + 0.008988492250127413 * slot[502 - 1]
        + 0.008913444188494461 * slot[503 - 1]
        + 0.008734602594676327 * slot[504 - 1]
        + 0.008480593594687306 * slot[505 - 1]
        + 0.008394575844424526 * slot[506 - 1]
        + 0.008119463098573483 * slot[507 - 1]
        + 0.007890056910994684 * slot[508 - 1]
        + 0.0076778281021881905 * slot[509 - 1]
        + 0.007513909232169928 * slot[510 - 1]
        + 0.007231870799749815 * slot[511 - 1]
        + 0.0070569312469529535 * slot[512 - 1]
        + 0.006981654136122227 * slot[513 - 1]
        + 0.006854079770379987 * slot[514 - 1]
        + 0.006886519198588898 * slot[515 - 1]
        + 0.006894283629668635 * slot[516 - 1]
        + 0.006526782797484208 * slot[517 - 1]
        + 0.006270638592403101 * slot[518 - 1]
        + 0.006090296114664182 * slot[519 - 1]
        + 0.0059080132955974096 * slot[520 - 1]
        + 0.0060661304099355814 * slot[521 - 1]
        + 0.0058152171499044145 * slot[522 - 1]
        + 0.005571299509347157 * slot[523 - 1]
        + 0.005241663588406994 * slot[524 - 1]
        + 0.0058550246820545165 * slot[525 - 1]
        + 0.006262022686407807 * slot[526 - 1]
        + 0.006047326725737036 * slot[527 - 1]
        + 0.0056021478319408175 * slot[528 - 1]
        + 0.005427510796429478 * slot[529 - 1]
        + 0.005183597942212561 * slot[530 - 1]
        + 0.004805812620678812 * slot[531 - 1]
        + 0.004451886864745763 * slot[532 - 1]
        + 0.0041335246290178625 * slot[533 - 1]
        + 0.0037694359110372333 * slot[534 - 1]
        + 0.003452922789571772 * slot[535 - 1]
        + 0.0031364324129558203 * slot[536 - 1]
        + 0.002862355961372612 * slot[537 - 1]
        + 0.002634915646668168 * slot[538 - 1]
        + 0.002393073006673679 * slot[539 - 1]
        + 0.00219629428479734 * slot[540 - 1]
        + 0.0019455800206576346 * slot[541 - 1]
        + 0.0017231348006133526 * slot[542 - 1]
        + 0.0015182899233478248 * slot[543 - 1]
        + 0.0012579351142379537 * slot[544 - 1]
        + 0.0010558229529442065 * slot[545 - 1]
        + 0.0007828205779498739 * slot[546 - 1]
        + 0.0005878418640679356 * slot[547 - 1]
        + 0.00040974496506221566 * slot[548 - 1]
        + 0.0005759312236583157 * slot[549 - 1]
        + 0.0002346694572403321 * slot[550 - 1]
        - 0.00006917418645987319 * slot[551 - 1]
        - 0.0003135657568127908 * slot[552 - 1]
        - 0.0006674371177349807 * slot[553 - 1]
        - 0.0009758778357846107 * slot[554 - 1]
        - 0.00036462059479381464 * slot[555 - 1]
        - 0.0007397824832639817 * slot[556 - 1]
        - 0.000979537952941836 * slot[557 - 1]
        - 0.0012683803865676756 * slot[558 - 1]
        - 0.0015235280995039497 * slot[559 - 1]
        - 0.0019677008447522683 * slot[560 - 1]
        - 0.0021849194370361785 * slot[561 - 1]
        - 0.002618073044658053 * slot[562 - 1]
        - 0.0030093858626602113 * slot[563 - 1]
        - 0.003324949110877438 * slot[564 - 1]
        - 0.003818540732247422 * slot[565 - 1]
        - 0.004114978136720642 * slot[566 - 1]
        - 0.004388519573791748 * slot[567 - 1]
        - 0.004846617087983753 * slot[568 - 1]
        - 0.005142053635484361 * slot[569 - 1]
        - 0.005603644655134404 * slot[570 - 1]
        - 0.006031112340499508 * slot[571 - 1]
        - 0.006591249478451163 * slot[572 - 1]
        - 0.007140892039227243 * slot[573 - 1]
        - 0.0075987220346921324 * slot[574 - 1]
        - 0.008124384967320703 * slot[575 - 1]
        - 0.008561121384680162 * slot[576 - 1]
        - 0.008897000279203105 * slot[577 - 1]
        - 0.009516036151970827 * slot[578 - 1]
        - 0.010103147588702921 * slot[579 - 1]
        - 0.01044675980032483 * slot[580 - 1]
        - 0.010786841567663671 * slot[581 - 1]
        - 0.011063038900000854 * slot[582 - 1]
        - 0.011469037072224944 * slot[583 - 1]
        - 0.011977793223481146 * slot[584 - 1]
        - 0.01235293836690877 * slot[585 - 1]
        - 0.012790098848373187 * slot[586 - 1]
        - 0.01321450639978258 * slot[587 - 1]
        - 0.013547050801672112 * slot[588 - 1]
        - 0.01382198776881589 * slot[589 - 1]
        - 0.014319595640217267 * slot[590 - 1]
        - 0.014780344895576994 * slot[591 - 1]
        - 0.015257147796119802 * slot[592 - 1]
        - 0.015551543458096373 * slot[593 - 1]
        - 0.01588392540483212 * slot[594 - 1]
        - 0.016112435435723545 * slot[595 - 1]
        - 0.016231553416933202 * slot[596 - 1]
        - 0.016605083032771882 * slot[597 - 1]
        - 0.01704079040117089 * slot[598 - 1]
        - 0.01745350041099225 * slot[599 - 1]
        - 0.017848061963556754 * slot[600 - 1]
        - 0.018241334707773237 * slot[601 - 1]
        - 0.01852807944521595 * slot[602 - 1]
        - 0.018873185974609356 * slot[603 - 1]
        - 0.019216726854593937 * slot[604 - 1]
        - 0.019584238298878863 * slot[605 - 1]
        - 0.019828068246884947 * slot[606 - 1]
        - 0.020176331984905062 * slot[607 - 1]
        - 0.020430181059073464 * slot[608 - 1]
        - 0.02074390030228585 * slot[609 - 1]
        - 0.020992415055016627 * slot[610 - 1]
        - 0.0213065573299184 * slot[611 - 1]
        - 0.021411318621544686 * slot[612 - 1]
        - 0.021712168962396975 * slot[613 - 1]
        - 0.023324486622500338 * slot[614 - 1]
        - 0.023527275063309226 * slot[615 - 1]
        - 0.023859918337683642 * slot[616 - 1]
        - 0.024105689440951705 * slot[617 - 1]
        - 0.024551678451162816 * slot[618 - 1]
        - 0.02473105205357072 * slot[619 - 1]
        - 0.025111391928419872 * slot[620 - 1]
        - 0.025266892184875177 * slot[621 - 1]
        - 0.02539296380261646 * slot[622 - 1]
        - 0.025795125373328127 * slot[623 - 1]
        - 0.026243991178524922 * slot[624 - 1]
        - 0.02707490013886083 * slot[625 - 1]
        - 0.02761848513888612 * slot[626 - 1]
        - 0.027860843701497676 * slot[627 - 1]
        - 0.028188716251875422 * slot[628 - 1]
        - 0.028533630833181986 * slot[629 - 1]
        - 0.028697921682536404 * slot[630 - 1]
        - 0.028839004002782016 * slot[631 - 1]
        - 0.02902875596937093 * slot[632 - 1]
        - 0.029230036529050943 * slot[633 - 1]
        - 0.02977015826959724 * slot[634 - 1]
        - 0.030086487622596282 * slot[635 - 1]
        - 0.03024035270779521 * slot[636 - 1]
        - 0.030658643673822667 * slot[637 - 1]
        - 0.030898932786132455 * slot[638 - 1]
        - 0.03092695114999704 * slot[639 - 1]
        - 0.03109128820981956 * slot[640 - 1]
        - 0.031218115262256874 * slot[641 - 1]
        - 0.03147454554261212 * slot[642 - 1]
        - 0.03177768771385604 * slot[643 - 1]
        - 0.032002632213790586 * slot[644 - 1]
        - 0.0321894894286238 * slot[645 - 1]
        - 0.03255296462176602 * slot[646 - 1]
        - 0.03272542276261754 * slot[647 - 1]
        - 0.033103477579413655 * slot[648 - 1]
        - 0.03349233473432045 * slot[649 - 1]
        - 0.03383292696499043 * slot[650 - 1]
        - 0.034045270509304094 * slot[651 - 1]
        - 0.03445332337869557 * slot[652 - 1]
        - 0.034679641231687854 * slot[653 - 1]
        - 0.034690250543899 * slot[654 - 1]
        - 0.035146556109058036 * slot[655 - 1]
        - 0.03541905869809918 * slot[656 - 1]
        - 0.03549937876195916 * slot[657 - 1]
        - 0.03859910565050049 * slot[658 - 1]
        - 0.03889302221015029 * slot[659 - 1]
        - 0.03885444042091958 * slot[660 - 1]
        - 0.03903905340384729 * slot[661 - 1]
        - 0.039140219609893896 * slot[662 - 1]
        - 0.03933138208304836 * slot[663 - 1]
        - 0.03934276675298095 * slot[664 - 1]
        - 0.03931844838128006 * slot[665 - 1]
        - 0.03928021691146265 * slot[666 - 1]
        - 0.03968021443991749 * slot[667 - 1]
        - 0.03989927606277054 * slot[668 - 1]
        - 0.040010537565254925 * slot[669 - 1]
        - 0.04023610616674892 * slot[670 - 1]
        - 0.04053177592561101 * slot[671 - 1]
        - 0.04099407803555921 * slot[672 - 1]
        - 0.041177957944577376 * slot[673 - 1]
        - 0.04142252917755085 * slot[674 - 1]
        - 0.04139087673996088 * slot[675 - 1]
        - 0.04142307392517115 * slot[676 - 1]
        - 0.04174815180749198 * slot[677 - 1]
        - 0.04191879899387143 * slot[678 - 1]
        - 0.042031461629140274 * slot[679 - 1]
        - 0.04245413282988656 * slot[680 - 1]
        - 0.04258887443737089 * slot[681 - 1]
        - 0.04275746585596558 * slot[682 - 1]
        - 0.04303387971899847 * slot[683 - 1]
        - 0.043074504823958956 * slot[684 - 1]
        - 0.04325904516857125 * slot[685 - 1]
        - 0.043464021338553614 * slot[686 - 1]
        - 0.04347979826567052 * slot[687 - 1]
        - 0.043712961607577344 * slot[688 - 1]
        - 0.043819636444551606 * slot[689 - 1]
        - 0.04378894057907155 * slot[690 - 1]
        - 0.04393382992693 * slot[691 - 1]
        - 0.04408115567207052 * slot[692 - 1]
        - 0.0443217326137478 * slot[693 - 1]
        - 0.04458429058323034 * slot[694 - 1]
        - 0.044862252501786055 * slot[695 - 1]
        - 0.044953191454675426 * slot[696 - 1]
        - 0.04534125084216981 * slot[697 - 1]
        - 0.04511529195061775 * slot[698 - 1]
        - 0.0453151944707476 * slot[699 - 1]
        - 0.04555998188299069 * slot[700 - 1]
        - 0.045742680530688286 * slot[701 - 1]
        - 0.04590594259618646 * slot[702 - 1]
        - 0.04624957729991253 * slot[703 - 1]
        - 0.04631549859805204 * slot[704 - 1]
        - 0.04654181013650017 * slot[705 - 1]
        - 0.04685326119107824 * slot[706 - 1]
        - 0.04718579782085112 * slot[707 - 1]
        - 0.04724081919856578 * slot[708 - 1]
        - 0.04765541258256142 * slot[709 - 1]
        - 0.047627863841357865 * slot[710 - 1]
        - 0.04793175204867856 * slot[711 - 1]
        - 0.04815499305113753 * slot[712 - 1]
        - 0.048188284998020005 * slot[713 - 1]
        - 0.048416753083508124 * slot[714 - 1]
        - 0.04868190419422016 * slot[715 - 1]
        - 0.04891368806053767 * slot[716 - 1]
        - 0.049224291319299125 * slot[717 - 1]
        - 0.04967469514148797 * slot[718 - 1]
        - 0.049923961242672094 * slot[719 - 1]
        - 0.05015820165745529 * slot[720 - 1]
        - 0.05052815960074533 * slot[721 - 1]
        - 0.05056285552108443 * slot[722 - 1]
        - 0.05059687661176128 * slot[723 - 1]
        - 0.05098156071918954 * slot[724 - 1]
        - 0.05122285134390636 * slot[725 - 1]
        - 0.051267431774530324 * slot[726 - 1]
        - 0.05145863171677167 * slot[727 - 1]
        - 0.051332271348372906 * slot[728 - 1]
        - 0.05156521542107769 * slot[729 - 1]
        - 0.051606560023391014 * slot[730 - 1]
        - 0.051566963461014775 * slot[731 - 1]
        - 0.051598070382155865 * slot[732 - 1]
        - 0.051739972759186646 * slot[733 - 1]
        - 0.0515726807900998 * slot[734 - 1]
        - 0.051603388180934344 * slot[735 - 1]
        - 0.05163927512277141 * slot[736 - 1]
        - 0.051745420814874224 * slot[737 - 1]
        - 0.051691915618009454 * slot[738 - 1]
        - 0.051769122507359766 * slot[739 - 1]
        - 0.05143365467769379 * slot[740 - 1]
        - 0.05126563118138641 * slot[741 - 1]
        - 0.05082381024892415 * slot[742 - 1]
        - 0.05023122132767842 * slot[743 - 1]
        - 0.050029191615160666 * slot[744 - 1]
        - 0.04946957116032246 * slot[745 - 1]
        - 0.04907288932011327 * slot[746 - 1]
        - 0.04857779803562723 * slot[747 - 1]
        - 0.04766757059730938 * slot[748 - 1]
        - 0.046474272398849846 * slot[749 - 1]
        - 0.0449256945062988 * slot[750 - 1]
        - 0.043001186216341636 * slot[751 - 1]
        - 0.04077421128297582 * slot[752 - 1]
        - 0.03799737913335472 * slot[753 - 1]
        + 0.08108005373107707 * slot[754 - 1]
        + 0.08512414352069411 * slot[755 - 1]
        + 0.08879622145461578 * slot[756 - 1]
        + 0.0920277302031524 * slot[757 - 1]
        + 0.09499328371372204 * slot[758 - 1]
        + 0.09733125630501743 * slot[759 - 1]
        + 0.09964120872299441 * slot[760 - 1]
        + 0.10172739659322765 * slot[761 - 1]
        + 0.10343301727615625 * slot[762 - 1]
        + 0.10521767036948919 * slot[763 - 1]
        + 0.1068832161046568 * slot[764 - 1]
        + 0.10810872612252935 * slot[765 - 1]
        + 0.10911213276434817 * slot[766 - 1]
        + 0.11048600732631646 * slot[767 - 1]
        + 0.11154802189645271 * slot[768 - 1]
        + 0.11250322865339393 * slot[769 - 1]
        + 0.11381392247372686 * slot[770 - 1]
        + 0.11476267531102845 * slot[771 - 1]
        + 0.11568428021432518 * slot[772 - 1]
        + 0.11659903801563783 * slot[773 - 1]
        + 0.11754037769182707 * slot[774 - 1]
        + 0.11834620775986841 * slot[775 - 1]
        + 0.11910908163605141 * slot[776 - 1]
        + 0.11986271483749165 * slot[777 - 1]
        + 0.1201404151549039 * slot[778 - 1]
        + 0.12127042353262481 * slot[779 - 1]
        + 0.12166220909169066 * slot[780 - 1]
        + 0.12244451186596103 * slot[781 - 1]
        + 0.12273772604149803 * slot[782 - 1]
        + 0.12346962160473654 * slot[783 - 1]
        + 0.12402494609608357 * slot[784 - 1]
        + 0.12421428442440405 * slot[785 - 1]
        + 0.12467597548935383 * slot[786 - 1]
        + 0.12501514464525745 * slot[787 - 1]
        + 0.12530963556658156 * slot[788 - 1]
        + 0.1250630680680048 * slot[789 - 1]
        + 0.12556157396973228 * slot[790 - 1]
        + 0.12546251110109033 * slot[791 - 1]
        + 0.12509411933767703 * slot[792 - 1]
        + 0.12515314132796201 * slot[793 - 1]
        + 0.12497859215447342 * slot[794 - 1]
        + 0.12503383233908075 * slot[795 - 1]
        + 0.12509358793820963 * slot[796 - 1]
        + 0.12503145764196893 * slot[797 - 1]
        + 0.1249505178651099 * slot[798 - 1]
        + 0.12489938671958489 * slot[799 - 1]
        + 0.12438287574430774 * slot[800 - 1]
        + 0.12416945609663012 * slot[801 - 1]
        + 0.12462907270084887 * slot[802 - 1]
        + 0.12426836183592635 * slot[803 - 1]
        + 0.12446952014772442 * slot[804 - 1]
        + 0.12495948116153217 * slot[805 - 1]
        + 0.12565323762032046 * slot[806 - 1]
        + 0.1253795534997196 * slot[807 - 1]
        + 0.12551661595861538 * slot[808 - 1]
        + 0.12742888070812589 * slot[809 - 1]
        + 0.1277230490542109 * slot[810 - 1]
        + 0.12801013671835146 * slot[811 - 1]
        + 0.12783232841353265 * slot[812 - 1]
        + 0.12810454082426664 * slot[813 - 1]
        + 0.1281109212807569 * slot[814 - 1]
        + 0.12767200594143652 * slot[815 - 1]
        + 0.1279681630835259 * slot[816 - 1]
        + 0.12778159531412495 * slot[817 - 1]
        + 0.12798610404550625 * slot[818 - 1]
        + 0.127439067616926 * slot[819 - 1]
        + 0.127749894254697 * slot[820 - 1]
        + 0.12696371193876135 * slot[821 - 1]
        + 0.12738671945525512 * slot[822 - 1]
        + 0.12691162023191216 * slot[823 - 1]
        + 0.12686356403973817 * slot[824 - 1]
        + 0.1267678069309619 * slot[825 - 1]
        + 0.12641847681206184 * slot[826 - 1]
        + 0.12708206799853727 * slot[827 - 1]
        + 0.12636144662129845 * slot[828 - 1]
        + 0.12608648596262673 * slot[829 - 1]
        + 0.12559442829505416 * slot[830 - 1]
        + 0.12441333051831271 * slot[831 - 1]
        + 0.12505476341369362 * slot[832 - 1]
        + 0.12447911502733862 * slot[833 - 1]
        + 0.12555501256399582 * slot[834 - 1]
        + 0.12627535718403546 * slot[835 - 1]
        + 0.12742840285907298 * slot[836 - 1]
        + 0.12718449249853128 * slot[837 - 1]
        + 0.1267787965663776 * slot[838 - 1]
        + 0.1280006114800719 * slot[839 - 1]
        + 0.1263758792693928 * slot[840 - 1]
        + 0.128485949542814 * slot[841 - 1]
        + 0.12829644456011835 * slot[842 - 1]
        + 0.12677711805998595 * slot[843 - 1]
        + 0.12676838066557616 * slot[844 - 1]
        + 0.12914569615575977 * slot[845 - 1]
        + 0.13009205479680186 * slot[846 - 1]
        + 0.1290650940581913 * slot[847 - 1]
        + 0.1288138999207942 * slot[848 - 1]
        + 0.1281730551091131 * slot[849 - 1]
        + 0.13187427655101522 * slot[850 - 1]
        + 0.13108708563787522 * slot[851 - 1]
        + 0.12943495271636837 * slot[852 - 1]
        + 0.13012803161645195 * slot[853 - 1]
        + 0.1284740237714882 * slot[854 - 1]
        + 0.13068498882716195 * slot[855 - 1]
        + 0.12896074110398373 * slot[856 - 1]
        + 0.12846291993505715 * slot[857 - 1]
        + 0.12816345943016424 * slot[858 - 1]
        + 0.13263952850873115 * slot[859 - 1]
        + 0.13219439515348586 * slot[860 - 1]
        + 0.13129290837629218 * slot[861 - 1]
        + 0.13094340658581516 * slot[862 - 1]
        + 0.1308740425496093 * slot[863 - 1]
        + 0.13479396805070656 * slot[864 - 1]
        + 0.13465563467287203 * slot[865 - 1]
        + 0.13469799799338025 * slot[866 - 1]
        + 0.1337818393795347 * slot[867 - 1]
        + 0.13839308771122655 * slot[868 - 1]
        + 0.13795699845387824 * slot[869 - 1]
        + 0.13624191192203677 * slot[870 - 1]
        + 0.13567401296031537 * slot[871 - 1]
        + 0.13461736346144412 * slot[872 - 1]
        + 0.13767887044660074 * slot[873 - 1]
        + 0.13562371133518542 * slot[874 - 1]
        + 0.13443489505171863 * slot[875 - 1]
        + 0.13353931310603065 * slot[876 - 1]
        + 0.1320176247468032 * slot[877 - 1]
        + 0.13425578914755695 * slot[878 - 1]
        + 0.132179486338035 * slot[879 - 1]
        + 0.13179633946490887 * slot[880 - 1]
        + 0.12887239087034003 * slot[881 - 1]
        + 0.13166728930263064 * slot[882 - 1]
        + 0.12908049532619664 * slot[883 - 1]
        + 0.12686192592895318 * slot[884 - 1]
        + 0.12237659757334385 * slot[885 - 1]
        + 0.11939969952995938 * slot[886 - 1]
        + 0.1211191418563195 * slot[887 - 1]
        + 0.1165896747819706 * slot[888 - 1]
        + 0.1142563652144209 * slot[889 - 1]
        + 0.11046261923959201 * slot[890 - 1]
        + 0.11303496090899177 * slot[891 - 1]
        + 0.10777290396230889 * slot[892 - 1]
        + 0.10534426107869721 * slot[893 - 1]
        + 0.10223325981189692 * slot[894 - 1]
        + 0.09962854250865645 * slot[895 - 1]
        + 0.09812483082501694 * slot[896 - 1]
        + 0.095789363387989 * slot[897 - 1]
        + 0.09443204344469894 * slot[898 - 1]
        + 0.08938289450587933 * slot[899 - 1]
        + 0.08674383011453382 * slot[900 - 1]
        + 0.08493002554682627 * slot[901 - 1]
        + 0.08525440196691723 * slot[902 - 1]
        + 0.08092496215166385 * slot[903 - 1]
        + 0.07878197093911352 * slot[904 - 1]
        + 0.07832876467426515 * slot[905 - 1]
        + 0.07673349094762356 * slot[906 - 1]
        + 0.07573856385560378 * slot[907 - 1]
        + 0.07132460018105996 * slot[908 - 1]
        + 0.07127728891962773 * slot[909 - 1]
        + 0.0712886410862129 * slot[910 - 1]
        + 0.06856341860891824 * slot[911 - 1]
        + 0.06419543797248428 * slot[912 - 1]
        + 0.06447785603636795 * slot[913 - 1]
        + 0.06573573572943067 * slot[914 - 1]
        + 0.06755899528218987 * slot[915 - 1]
        + 0.06961142866792944 * slot[916 - 1]
        + 0.07109445917460623 * slot[917 - 1]
        + 0.07562836883367528 * slot[918 - 1]
        + 0.08377286285311124 * slot[919 - 1]
        + 0.091021880148748 * slot[920 - 1]
        + 0.08786926466728794 * slot[921 - 1]
        + 0.08645121946623621 * slot[922 - 1]
        + 0.10068859877437747 * slot[923 - 1]
        + 0.11360653016147444 * slot[924 - 1]
        + 0.1269615679300754 * slot[925 - 1]
        + 0.12114772530243724 * slot[926 - 1]
        + 0.13614761112168752 * slot[927 - 1]
        + 0.169311934138293 * slot[928 - 1]
        + 0.1851129811907007 * slot[929 - 1]
        + 0.17286698986813975 * slot[930 - 1]
        + 0.19462493576036455 * slot[931 - 1]
        + 0.19128626591485176 * slot[932 - 1]
        + 0.2620602018767476 * slot[933 - 1]
        + 0.3012987050740674 * slot[934 - 1]
        + 0.2967693094526247 * slot[935 - 1]
        + 0.3141792766387061 * slot[936 - 1]
        + 0.40343796325383136 * slot[937 - 1]
        + 0.4371698761569807 * slot[938 - 1]
        + 0.4398666763698252 * slot[939 - 1]
        + 0.487859655521671 * slot[940 - 1]
        + 0.46791873326913375 * slot[941 - 1]
        + 0.6170039335272655 * slot[942 - 1]
        + 0.6916674545373983 * slot[943 - 1]
        + 0.7064480388938357 * slot[944 - 1]
        + 0.7307014345906688 * slot[945 - 1]
        + 0.8624859153256038 * slot[946 - 1]
        + 0.9334497889753839 * slot[947 - 1]
        + 0.9017992291040657 * slot[948 - 1]
        + 0.9027357302854504 * slot[949 - 1]
        + 0.9392604101642124 * slot[950 - 1]
        + 1.2262092198817651 * slot[951 - 1]
        + 1.2392821027555008 * slot[952 - 1]
        + 1.1902964021694593 * slot[953 - 1]
        + 1.256016896486102 * slot[954 - 1]
        + 1.2409342337661016 * slot[955 - 1]
        + 1.5157284192409521 * slot[956 - 1]
        + 1.4453411008776795 * slot[957 - 1]
        + 1.5981708356205107 * slot[958 - 1]
        + 1.5003558363623115 * slot[959 - 1]
        + 2.0553590667259662 * slot[960 - 1]
        + 2.377166260994738 * slot[961 - 1]
        + 2.34639976612214 * slot[962 - 1]
        + 2.650690864949525 * slot[963 - 1]
        + 2.8287406504073926 * slot[964 - 1]
        + 3.977029088214841 * slot[965 - 1]
        + 4.41650935597905 * slot[966 - 1]
        + 4.909827245814919 * slot[967 - 1]
        + 5.024686799723611 * slot[968 - 1]
        + 6.240696974563136 * slot[969 - 1]
        + 6.648963155673164 * slot[970 - 1]
        + 6.768435791627201 * slot[971 - 1]
        + 7.385261930540362 * slot[972 - 1]
        + 6.938582472354606 * slot[973 - 1]
        + 7.567319850163125 * slot[974 - 1]
        + 7.5668982106617255 * slot[975 - 1]
        + 7.93368255619365 * slot[976 - 1]
        + 7.7130941289804635 * slot[977 - 1]
        + 7.324078517045919 * slot[978 - 1]
        + 8.163379680563748 * slot[979 - 1]
        + 8.83195466540829 * slot[980 - 1]
        + 9.165763880896492 * slot[981 - 1]
        + 8.952500822896743 * slot[982 - 1]
        + 9.3925884372303 * slot[983 - 1]
        + 9.6300306868563 * slot[984 - 1]
        + 9.42565227542515 * slot[985 - 1]
        + 9.842426176944944 * slot[986 - 1]
        + 10.828785842781228 * slot[987 - 1]
        + 11.709807589629936 * slot[988 - 1]
        + 11.213255118244248 * slot[989 - 1]
        + 11.430302109251814 * slot[990 - 1]
        + 11.62305074164638 * slot[991 - 1]
        + 12.60389772619945 * slot[992 - 1]
        + 14.499203438391069 * slot[993 - 1]
        + 12.843389504699095 * slot[994 - 1]
        + 14.441429705834373 * slot[995 - 1]
        + 14.287448139751627 * slot[996 - 1]
        + 14.622482855993862 * slot[997 - 1]
        + 15.56782788827701 * slot[998 - 1]
        + 11.968483258822141 * slot[999 - 1]
        + 12.13740807312507 * slot[1000 - 1]
        + 11.457726408978269 * slot[1001 - 1]
        + 11.55772684802132 * slot[1002 - 1]
        + 10.820228459397239 * slot[1003 - 1]
        + 12.036255338123896 * slot[1004 - 1]
        + 11.166205880934053 * slot[1005 - 1]
        + 13.133012056371713 * slot[1006 - 1]
        + 13.83843440202586 * slot[1007 - 1]
        + 14.307078344769046 * slot[1008 - 1]
        + 15.531873481020924 * slot[1009 - 1]
        + 13.520596173591887 * slot[1010 - 1]
        + 10.641548695258626 * slot[1011 - 1]
        + 10.248825349397027 * slot[1012 - 1]
        + 8.953940965534835 * slot[1013 - 1]
        + 2.1711528595245904 * slot[1014 - 1]
        + 5.486803094799959 * slot[1015 - 1]
        + 0.9726921583567536 * slot[1016 - 1]
        + 0.09125056165895032 * slot[1017 - 1]
        + 3.542181971110563 * slot[1018 - 1]
        - 4.18161973699999 * slot[1019 - 1]
        + 5.521860571096337 * slot[1020 - 1]
        + 3.3412421551243088 * slot[1021 - 1]
        - 27.20917609989489 * slot[1022 - 1]
        - 14.943357513738484 * slot[1023 - 1];

    let square = 0.0_f64;

    println!("tone mapping function classifier (logistic regression)");
    println!(
        "legacy: {}, linear: {}, logistic: {}, ratio: {}, square: {}",
        legacy, linear, logistic, ratio, square
    );

    if (legacy > linear) && (legacy > logistic) && (legacy > ratio) && (legacy > square) {
        return 0;
    }

    if (linear > legacy) && (linear > logistic) && (linear > ratio) && (linear > square) {
        return 1;
    }

    if (logistic > linear) && (logistic > legacy) && (logistic > ratio) && (logistic > square) {
        return 2;
    }

    if (ratio > linear) && (ratio > legacy) && (ratio > logistic) && (ratio > square) {
        return 3;
    }

    if (square > linear) && (square > legacy) && (square > logistic) && (square > ratio) {
        return 4;
    }

    return 0;
}
