use std;
//use std::sync::RwLock;
use parking_lot::RwLock;
use std::time::{SystemTime};
use std::fs::File;
use std::io::{Read,Write};
use std::io::Cursor;
use half::f16;
use std::mem;
use byteorder::{BigEndian, ReadBytesExt};

use actix::*;
use server;

static JVO_FITS_DB: &'static str = "alma";
pub static FITSCACHE: &'static str = "FITSCACHE";

const FITS_CHUNK_LENGTH: usize = 2880;
const FITS_LINE_LENGTH: usize = 80;
const NBINS: usize = 1024;

#[derive(Debug)]
pub struct FITS {
        dataset_id: String,
        data_id: String,
        //basic header/votable        
        obj_name: String,        
        obs_date: String,
        timesys: String,
        specsys: String,
        beam_unit: String,
        beam_type: String,
        filepath: String,
        bmaj: f32,
        bmin: f32,
        bpa: f32,
        restfrq: f32,
        line: String,
        obsra: f32,
        obsdec: f32,
        datamin: f32,
        datamax: f32,
        //this is a FITS data part
        bitpix: i32,
        naxis: i32,
        naxes: [i32; 4],    
        width: i32,
        height: i32,
        depth: i32,
        polarisation: i32,
        data_u8: Vec<Vec<u8>>,
        data_i16: Vec<Vec<i16>>,        
        data_i32: Vec<Vec<i32>>,
        data_f16: Vec<Vec<f16>>,//half-float (short)
        //data_f32: Vec<f32>,//float32 will always be converted to float16
        data_f64: Vec<Vec<f64>>,
        header: String,        
        mean_spectrum: Vec<f32>,
        integrated_spectrum: Vec<f32>,
        mask: Vec<bool>,
        pixels: Vec<f32>,
        bscale: f32,
        bzero: f32,
        ignrval: f32,
        crval1: f32,
        cdelt1: f32,
        crpix1: f32,
        cunit1: String,
        ctype1: String,
        crval2: f32,
        cdelt2: f32,
        crpix2: f32,
        cunit2: String,
        ctype2: String,
        crval3: f32,
        cdelt3: f32,
        crpix3: f32,
        cunit3: String,
        ctype3: String,     
        min: f32,
        max: f32,
        hist: Vec<i32>,
        median: f32,
        mad: f32,
        mad_p: f32,
        mad_n: f32,        
        black: f32,
        white: f32,
        sensitivity: f32,
        flux: String,
        has_frequency: bool,
        has_velocity: bool,
        frame_multiplier: f32,
        pub has_header: bool,
        pub has_data: bool,       
        pub timestamp: RwLock<SystemTime>,//last access time
}

impl FITS {
    pub fn new(id: &String) -> FITS {
        let fits = FITS {
            dataset_id: id.clone(),
            data_id: format!("{}_00_00_00", id),
            obj_name: String::from(""),
            obs_date: String::from(""),
            timesys: String::from(""),
            specsys: String::from(""),
            beam_unit: String::from(""),
            beam_type: String::from(""),
            filepath: String::from(""),
            bmaj: 0.0,
            bmin: 0.0,
            bpa: 0.0,
            restfrq: 0.0,
            line: String::from(""),
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
            bscale: 0.0,
            bzero: 0.0,
            ignrval: std::f32::MIN,
            crval1: 0.0,
            cdelt1: 0.0,
            crpix1: 0.0,
            cunit1: String::from(""),
            ctype1: String::from(""),
            crval2: 0.0,
            cdelt2: 0.0,
            crpix2: 0.0,
            cunit2: String::from(""),
            ctype2: String::from(""),
            crval3: 0.0,
            cdelt3: 0.0,
            crpix3: 0.0,
            cunit3: String::from(""),
            ctype3: String::from(""),
            min: std::f32::MIN,
            max: std::f32::MAX,
            hist: Vec::new(),
            median: 0.0,
            mad: 0.0,
            mad_p: 0.0,
            mad_n: 0.0,
            black: 0.0,
            white: 0.0,
            sensitivity: 0.0,
            flux: String::from("logistic"),
            has_frequency: false,
            has_velocity: false,
            frame_multiplier: 1.0,
            has_header: false,
            has_data: false,
            timestamp: RwLock::new(SystemTime::now()),                    
        } ;        
        
        fits
    }    

    pub fn load_from_path(&mut self, id: &String, filepath: &std::path::Path, server: &Addr<Syn, server::SessionServer>) /*-> FITS*/ {
        let mut fits = self;//FITS::new(id);        

        //load data from filepath
        let mut f = match File::open(filepath) {
            Ok(x) => x,
            Err(x) => { println!("{:?}: {:?}", filepath, x);
                        return //fits;
                        //a desperate attempt to download FITS using the ALMA URL (will fail for non-ALMA datasets)                        
                        /*let url = format!("http://{}:8060/skynode/getDataForALMA.do?db={}&table=cube&data_id={}_00_00_00", JVO_FITS_SERVER, JVO_FITS_DB, id) ;
                        return FITS::from_url(&data_id, &url);*/                      
                    }
        } ;

        match f.metadata() {
            Ok(metadata) => {   let len = metadata.len() ;
                                println!("{:?}, {} bytes", f, len);
                                
                                if len < FITS_CHUNK_LENGTH as u64 {
                                    return //fits;
                                };
                        }
            Err(err) => {   println!("file metadata reading problem: {}", err);
                            return //fits;
                    }
        } ;

        //OK, we have a FITS file with at least one chunk        
        println!("{}: reading FITS header...", id) ;

        //let mut f = BufReader::with_capacity(FITS_CHUNK_LENGTH, f);

        let mut header: Vec<u8> = Vec::new();
        let mut end: bool = false ;
        let mut no_hu: i32 = 0 ;

        while !end {
            //read a FITS chunk
            let mut chunk = [0; FITS_CHUNK_LENGTH];

            match f.read_exact(&mut chunk) {
                Ok(()) => {
                    no_hu = no_hu + 1;

                    //parse a FITS header chunk
                    end = fits.parse_fits_header_chunk(&chunk);
                    header.extend_from_slice(&chunk);                    
                },
                Err(err) => {                    
                    println!("CRITICAL ERROR reading FITS header: {}", err);
                    return //fits;
                }
            } ;
        }           

        //test for frequency/velocity
        fits.frame_reference_unit() ;
        fits.frame_reference_type() ;

        if fits.restfrq > 0.0 {
            fits.has_frequency = true ;
        }                

        fits.has_header = true ;

        println!("{}/#hu = {}, {:?}", id, no_hu, fits);

        fits.header = match String::from_utf8(header) {
            Ok(x) => x,
            Err(err) => {
                println!("FITS HEADER UTF8: {}", err);
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
        let mut data: Vec<u8> = vec![0; frame_size];
        let mut frame: i32 = 0;

        //let mut f = BufReader::with_capacity(frame_size, f);

        println!("FITS cube frame size: {} bytes", frame_size);

        let total = fits.depth;

        let cdelt3 = {
            if fits.has_velocity && fits.depth > 1 {
                fits.cdelt3 * fits.frame_multiplier / 1000.0
            }
            else {
                1.0
            }
        };

        while frame < fits.depth {                                 
            //println!("requesting a cube frame {}/{}", frame, fits.depth);

            //read a FITS cube frame
            match f.read_exact(&mut data) {
                Ok(()) => {                                        
                    //process a FITS cube frame (endianness, half-float)
                    //println!("processing cube frame {}/{}", frame+1, fits.depth);
                    fits.add_cube_frame(&data, cdelt3, frame as usize);                    
                    frame = frame + 1 ;
                    fits.send_progress_notification(&server, &"processing FITS".to_owned(), total, frame);
                },
                Err(err) => {
                    println!("CRITICAL ERROR reading FITS data: {}", err);
                    return //fits;
                }
            } ;            
        }        
        
        //println!("mean spectrum: {:?}", fits.mean_spectrum);
        //println!("integrated spectrum: {:?}", fits.integrated_spectrum);

        //we've gotten so far, we have the data, pixels, mask and spectrum
        fits.has_data = true ;

        fits.send_progress_notification(&server, &"processing FITS done".to_owned(), 0, 0);
        println!("{}: reading FITS data completed", id);

        //and lastly create a symbolic link in the FITSCACHE directory
        let filename = format!("{}/{}.fits", FITSCACHE, id);
        let cachefile = std::path::Path::new(&filename);      
        let _ = std::os::unix::fs::symlink(filepath, cachefile);     
        
        //fits
    }

    fn from_url(id: &String, url: &String) -> FITS {
        let fits = FITS::new(id);                

        println!("FITS::from_url({})", url);

        fits
    }

    fn send_progress_notification(&mut self, server: &Addr<Syn, server::SessionServer>, notification: &str, total: i32, running: i32) {
        let msg = json!({
            "type" : "progress",
            "message" : notification,
            "total" : total,
            "running" : running            
        });

        server.do_send(server::WsMessage {
            msg: msg.to_string(),
            dataset_id: self.dataset_id.clone(),
        });
    }

    fn init_data_storage(&mut self) -> usize {
        if self.width == 0 || self.height == 0 || self.depth == 0 {
            return 0;
        }

        let capacity = self.width * self.height ;

        self.mask.resize(capacity as usize, false);
        self.pixels.resize(capacity as usize, 0.0);

        self.mean_spectrum.resize(self.depth as usize, 0.0);
        self.integrated_spectrum.resize(self.depth as usize, 0.0);

        match self.bitpix {
            8 => self.data_u8.resize(self.depth as usize, Vec::with_capacity(capacity as usize)),
            16 => self.data_i16.resize(self.depth as usize, Vec::with_capacity(capacity as usize)),
            32 => self.data_i32.resize(self.depth as usize, Vec::with_capacity(capacity as usize)),
            -32 => self.data_f16.resize(self.depth as usize, Vec::with_capacity(capacity as usize)),
            -64 => self.data_f64.resize(self.depth as usize, Vec::with_capacity(capacity as usize)),
            _ => println!("unsupported bitpix: {}", self.bitpix)
        }

        (self.width * self.height * self.bitpix.abs() / 8) as usize
    }

    fn frame_reference_type(&mut self) {
        if self.ctype3.contains("F") || self.ctype3.contains("f") {
            self.has_frequency = true ;
        }

        if self.ctype3.contains("V") || self.ctype3.contains("v") {
            self.has_velocity = true ;
        }
    }

    fn frame_reference_unit(&mut self) {
        match self.cunit3.to_uppercase().as_ref() {
            "HZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1.0;
            },
            "KHZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1000.0;
            },
            "MHZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1000000.0;
            },
            "GHZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1000000000.0;
            },
            "THZ" => {
                self.has_frequency = true;
                self.frame_multiplier = 1000000000000.0;
            },
            "M/S" => {
                self.has_velocity = true;
                self.frame_multiplier = 1.0;
            },
            "KM/S" => {
                self.has_velocity = true;
                self.frame_multiplier = 1000.0;
            },
            _ => {}
        }
    }    

    fn parse_fits_header_chunk(&mut self, buf: &[u8]) -> bool {
        let mut offset: usize = 0 ;

        while offset < FITS_CHUNK_LENGTH {
            let slice = &buf[offset..offset+FITS_LINE_LENGTH];
            let line = match std::str::from_utf8(slice) {
                Ok(x) => x,
                Err(err) => {
                    println!("non-UTF8 characters found: {}", err);
                    return true;
                }
            } ;

            if line.contains("END       ") {
                return true ;            
            }

            if line.contains("OBJECT  = ") {
                self.obj_name = match scan_fmt!(line, "OBJECT  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("DATE-OBS= ") {
                self.obs_date = match scan_fmt!(line, "DATE-OBS= {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("LINE    = ") {
                self.line = match scan_fmt!(line, "LINE    = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("J_LINE  = ") {
                self.line = match scan_fmt!(line, "J_LINE  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("SPECSYS = ") {
                self.specsys = match scan_fmt!(line, "SPECSYS = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("TIMESYS = ") {
                self.timesys = match scan_fmt!(line, "TIMESYS = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("BITPIX  = ") {
                self.bitpix = match scan_fmt!(line, "BITPIX  = {d}", i32) {
                    Some(x) => x,
                    _ => 0
                }
            }

            if line.contains("NAXIS   = ") {
                self.naxis = match scan_fmt!(line, "NAXIS   = {d}", i32) {
                    Some(x) => x,
                    _ => 0
                }
            }

            if line.contains("NAXIS1  = ") {
                self.width = match scan_fmt!(line, "NAXIS1  = {d}", i32) {
                    Some(x) => x,
                    _ => 0
                };

                self.naxes[0] = self.width;
            }

            if line.contains("NAXIS2  = ") {
                self.height = match scan_fmt!(line, "NAXIS2  = {d}", i32) {
                    Some(x) => x,
                    _ => 0
                };

                self.naxes[1] = self.height;
            }

            if line.contains("NAXIS3  = ") {
                self.depth = match scan_fmt!(line, "NAXIS3  = {d}", i32) {
                    Some(x) => x,
                    _ => 1
                };

                self.naxes[2] = self.depth;
            }

            if line.contains("NAXIS4  = ") {
                self.polarisation = match scan_fmt!(line, "NAXIS4  = {d}", i32) {
                    Some(x) => x,
                    _ => 1
                };

                self.naxes[3] = self.polarisation;
            }            

            if line.contains("BTYPE   = ") {
                self.beam_type = match scan_fmt!(line, "BTYPE   = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("BUNIT   = ") {
                self.beam_unit = match scan_fmt!(line, "BUNIT   = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("BMAJ    = ") {
                let s = match scan_fmt!(line, "BMAJ    = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bmaj = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("BMIN    = ") {
                let s = match scan_fmt!(line, "BMIN    = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bmin = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("BPA     = ") {
                let s = match scan_fmt!(line, "BPA     = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bpa = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("RESTFRQ = ") {
                let s = match scan_fmt!(line, "RESTFRQ = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.restfrq = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("RESTFREQ= ") {
                let s = match scan_fmt!(line, "RESTFREQ= {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.restfrq = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("OBSRA   = ") {
                let s = match scan_fmt!(line, "OBSRA   = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.obsra = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("OBSDEC  = ") {
                let s = match scan_fmt!(line, "OBSDEC  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.obsdec = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("DATAMIN = ") {
                let s = match scan_fmt!(line, "DATAMIN = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.datamin = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => std::f32::MIN
                }
            }

            if line.contains("DATAMAX = ") {
                let s = match scan_fmt!(line, "DATAMAX = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.datamax = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => std::f32::MAX
                }
            }

            if line.contains("BSCALE  = ") {
                let s = match scan_fmt!(line, "BSCALE  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bscale = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }            

            if line.contains("BZERO   = ") {
                let s = match scan_fmt!(line, "BZERO   = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bzero = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("IGNRVAL = ") {
                let s = match scan_fmt!(line, "IGNRVAL = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.ignrval = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => std::f32::MIN
                }
            }

            if line.contains("CRVAL1  = ") {
                let s = match scan_fmt!(line, "CRVAL1  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crval1 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRVAL2  = ") {
                let s = match scan_fmt!(line, "CRVAL2  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crval2 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRVAL3  = ") {
                let s = match scan_fmt!(line, "CRVAL3  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crval3 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CDELT1  = ") {
                let s = match scan_fmt!(line, "CDELT1  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.cdelt1 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CDELT2  = ") {
                let s = match scan_fmt!(line, "CDELT2  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.cdelt2 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CDELT3  = ") {
                let s = match scan_fmt!(line, "CDELT3  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.cdelt3 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRPIX1  = ") {
                let s = match scan_fmt!(line, "CRPIX1  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crpix1 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRPIX2  = ") {
                let s = match scan_fmt!(line, "CRPIX2  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crpix2 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRPIX3  = ") {
                let s = match scan_fmt!(line, "CRPIX3  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crpix3 = match s.parse::<f32>() {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CUNIT1  = ") {
                self.cunit1 = match scan_fmt!(line, "CUNIT1  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("CUNIT2  = ") {
                self.cunit2 = match scan_fmt!(line, "CUNIT2  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("CUNIT3  = ") {
                self.cunit3 = match scan_fmt!(line, "CUNIT3  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("CTYPE1  = ") {
                self.ctype1 = match scan_fmt!(line, "CTYPE1  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("CTYPE2  = ") {
                self.ctype2 = match scan_fmt!(line, "CTYPE2  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            if line.contains("CTYPE3  = ") {
                self.ctype3 = match scan_fmt!(line, "CTYPE3  = {}", String) {
                    Some(x) => x.replace("'", ""),
                    _ => String::from("")
                }
            }

            offset = offset + FITS_LINE_LENGTH;
        }

        return false ;
    }

    fn add_cube_frame(&mut self, buf: &Vec<u8>, cdelt3: f32, frame: usize) {                
        let mut rdr = Cursor::new(buf);
        let len = self.width * self.height ;

        match self.bitpix {
            8 => {                                
                let mut sum : f32 = 0.0;
                let mut count : i32 = 0;

                for i in 0..len {
                    self.data_u8[frame].push(buf[i as usize]);

                    let tmp = self.bzero + self.bscale * (buf[i as usize] as f32) ;
                    if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {    
                        self.pixels[i as usize] += tmp ;                                
                        self.mask[i as usize] = true ;

                        sum += tmp ;
                        count += 1 ;                                
                    }                   
                }

                //mean and integrated intensities @ frame
                if count > 0 {
                    self.mean_spectrum[frame] = sum / (count as f32) ;
                    self.integrated_spectrum[frame] = sum * cdelt3 ;
                }
            },

            16 => {
                let mut sum : f32 = 0.0;
                let mut count : i32 = 0;

                for i in 0..len {
                    match rdr.read_i16::<BigEndian>() {
                        Ok(int16) => {
                            self.data_i16[frame].push(int16);

                            let tmp = self.bzero + self.bscale * (int16 as f32) ;
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                self.pixels[i as usize] += tmp ;                                
                                self.mask[i as usize] = true ;

                                sum += tmp ;
                                count += 1 ;                                
                            }
                        },
                        Err(err) => println!("BigEndian --> LittleEndian i16 conversion error: {}", err)
                    }
                }

                //mean and integrated intensities @ frame
                if count > 0 {
                    self.mean_spectrum[frame] = sum / (count as f32) ;
                    self.integrated_spectrum[frame] = sum * cdelt3 ;
                }
            },

            32 => {
                let mut sum : f32 = 0.0;
                let mut count : i32 = 0;

                for i in 0..len {
                    match rdr.read_i32::<BigEndian>() {
                        Ok(int32) => {
                            self.data_i32[frame].push(int32);

                            let tmp = self.bzero + self.bscale * (int32 as f32) ;
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                self.pixels[i as usize] += tmp ;                                
                                self.mask[i as usize] = true ;

                                sum += tmp ;
                                count += 1 ;                                
                            }
                        },
                        Err(err) => println!("BigEndian --> LittleEndian i32 conversion error: {}", err)
                    }
                }

                //mean and integrated intensities @ frame
                if count > 0 {
                    self.mean_spectrum[frame] = sum / (count as f32) ;
                    self.integrated_spectrum[frame] = sum * cdelt3 ;
                }
            },

            -32 => {
                let mut sum : f32 = 0.0;
                let mut count : i32 = 0;

                for i in 0..len {
                    match rdr.read_f32::<BigEndian>() {
                        Ok(float32) => {                            
                            let float16 = f16::from_f32(float32);
                            //println!("f32 = {} <--> f16 = {}", float32, float16);
                            self.data_f16[frame].push(float16);

                            let tmp = self.bzero + self.bscale * float32;
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                self.pixels[i as usize] += tmp ;                                
                                self.mask[i as usize] = true ;

                                sum += tmp ;
                                count += 1 ;                                
                            }                            
                        },
                        Err(err) => println!("BigEndian --> LittleEndian f32 conversion error: {}", err)
                    }                    
                }

                //mean and integrated intensities @ frame
                if count > 0 {
                    self.mean_spectrum[frame] = sum / (count as f32) ;
                    self.integrated_spectrum[frame] = sum * cdelt3 ;
                }
            },

            -64 => {
                let mut sum : f32 = 0.0;
                let mut count : i32 = 0;

                for i in 0..len {
                    match rdr.read_f64::<BigEndian>() {
                        Ok(float64) => {
                            self.data_f64[frame].push(float64);

                            let tmp = self.bzero + self.bscale * (float64 as f32);
                            if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                self.pixels[i as usize] += tmp ;                                
                                self.mask[i as usize] = true ;

                                sum += tmp ;
                                count += 1 ;                                
                            }
                        },
                        Err(err) => println!("BigEndian --> LittleEndian f64 conversion error: {}", err)
                    }
                }

                //mean and integrated intensities @ frame
                if count > 0 {
                    self.mean_spectrum[frame] = sum / (count as f32) ;
                    self.integrated_spectrum[frame] = sum * cdelt3 ;
                }
            },

            _ => println!("unsupported bitpix: {}", self.bitpix)
        }      
    }

    pub fn get_frequency_range(&self) -> (f32, f32) {
        let mut fmin: f32 = 0.0;
        let mut fmax: f32 = 0.0;

        if self.depth > 1 && self.has_frequency {
            let mut f1 = 0_f32 ;
            let mut f2 = 0_f32 ;

            if self.has_velocity {
                let c = 299792458_f32 ;//speed of light [m/s]
	  
	            let v1 : f32 = self.crval3 * self.frame_multiplier + self.cdelt3 * self.frame_multiplier * (1.0 - self.crpix3) ;

	            let v2 : f32 = self.crval3 * self.frame_multiplier + self.cdelt3 * self.frame_multiplier * ((self.depth as f32) - self.crpix3) ;

	            f1 = self.restfrq * ( (1.0-v1/c)/(1.0+v1/c) ).sqrt() ;
	            f2 = self.restfrq * ( (1.0-v2/c)/(1.0+v2/c) ).sqrt() ;
            }
            else {
                f1 = self.crval3 * self.frame_multiplier + self.cdelt3 * self.frame_multiplier * (1.0 - self.crpix3) ;

	            f2 = self.crval3 * self.frame_multiplier + self.cdelt3 * self.frame_multiplier * ((self.depth as f32) - self.crpix3) ;                
            };

            fmin = f1.min(f2);
            fmax = f1.max(f2);         
        }

        (fmin/1000000000.0, fmax/1000000000.0)
    }

    pub fn to_json(&self) -> String {
        let value = json!({                
                "HEADER" : self.header,
                "width" : self.width,
                "height" : self.height,
                "depth" : self.depth,
                "polarisation" : self.polarisation,
                "filesize" : 0,
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
                "mean_spectrum" : &self.mean_spectrum,
                "integrated_spectrum" : &self.integrated_spectrum,
                /* the histogram part, min, max etc... */
                "min" : self.min,
                "max" : self.max, 
                "median" : self.median,
                "sensitivity" : self.sensitivity,
                "black" : self.black,
                "white" : self.white,
                "flux" : self.flux,
                "histogram" : &self.hist,
        });

        value.to_string()
    }
    
}

impl Drop for FITS {
    fn drop(&mut self) {
        if self.has_data {
            println!("deleting {}", self.dataset_id);

            if self.bitpix == -32 && self.data_f16.len() > 0 {
                //check if the binary file already exists in the FITSCACHE
                let filename = format!("{}/{}.bin", FITSCACHE, self.dataset_id.replace("/","_"));
                let filepath = std::path::Path::new(&filename);

                if !filepath.exists() {
                    println!("{}: writing half-float f16 data to cache", self.dataset_id);

                    let tmp_filename = format!("{}/{}.bin.tmp", FITSCACHE, self.dataset_id.replace("/","_"));
                    let tmp_filepath = std::path::Path::new(&tmp_filename);

                    let mut buffer = match File::create(tmp_filepath) {
                        Ok(f) => f,
                        Err(err) => {
                            println!("{}", err);
                            return;
                        }
                    };

                    for frame in 0..self.depth as usize {
                        let v16 = self.data_f16[frame].clone();                        
                        let ptr = v16.as_ptr() as *mut u8;
                        let len = v16.len() ;
                        let capacity = self.data_f16[frame].capacity() ;

                        unsafe {
                            mem::forget(v16);

                            let raw: Vec<u8> = Vec::from_raw_parts(ptr, 2*len, 2*capacity);
                            
                            match buffer.write_all(&raw) {
                                Ok(()) => {                                    
                                },
                                Err(err) => {
                                    println!("binary cache write error: {}, removing the temporary file", err);
                                    let _ = std::fs::remove_file(tmp_filepath);
                                    return;
                                }
                            }
                        };                                               
                    };                    

                    let _ = std::fs::rename(tmp_filepath, filepath);
                }
            }
        }
    }
}