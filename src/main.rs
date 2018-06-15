extern crate chrono;
extern crate actix;
extern crate actix_web;
extern crate half;
extern crate byteorder;

use std::thread;
use half::f16;
use std::io::{Read,Write};
use std::str::FromStr;
use std::fs::File;
use std::io::Cursor;
use byteorder::{BigEndian, ReadBytesExt};
use std::fmt;
use std::mem;
use std::env;

use actix::*;
use actix_web::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFloatError {
    _private: (),
}

impl ParseFloatError {
    fn new() -> ParseFloatError {
        ParseFloatError { _private: () }
    }
}

impl fmt::Display for ParseFloatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Could not parse float")
    }
}

fn parse_f32(s: &String) -> Result<f32, ParseFloatError> {
    let (a,b) = scan_fmt!( &s.replace("E"," ").replace("e"," "), "{f}{d}", f32, i32);   

    let mantissa = match a {
        Some(x) => x,
        None => return Err(ParseFloatError::new()),
    };

    let exponent = match b {
        Some(x) => x,
        None => 0
    };

    Ok(mantissa * 10f32.powi(exponent))
}

#[macro_use]
extern crate scan_fmt;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_json;

use std::time::{/*Duration,*/ SystemTime};
use std::collections::HashMap;
use std::sync::RwLock;

static JVO_FITS_SERVER: &'static str = "jvox.vo.nao.ac.jp";
static JVO_FITS_DB: &'static str = "alma";
static FITSCACHE: &'static str = "FITSCACHE";

const FITS_CHUNK_LENGTH: usize = 2880;
const FITS_LINE_LENGTH: usize = 80;
const NBINS: usize = 1024;

#[derive(Debug)]
struct FITS {
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
        sensitivity: f32,
        has_frequency: bool,
        has_velocity: bool,
        frame_multiplier: f32,
        has_data: bool,       
        timestamp: RwLock<SystemTime>,//last access time
}

impl FITS {
    fn new(id: &String) -> FITS {
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
            sensitivity: 0.0,
            has_frequency: false,
            has_velocity: false,
            frame_multiplier: 1.0,
            has_data: false,
            timestamp: RwLock::new(SystemTime::now()),                    
        } ;        
        
        fits
    }    

    fn from_path(id: &String, filepath: &std::path::Path) -> FITS {
        let mut fits = FITS::new(id);
        
        //load data from filepath
        let mut f = match File::open(filepath) {
            Ok(x) => x,
            Err(x) => { println!("{:?}: {:?}", filepath, x);
                        return fits;
                        //a desperate attempt to download FITS using the ALMA URL (will fail for non-ALMA datasets)                        
                        /*let url = format!("http://{}:8060/skynode/getDataForALMA.do?db={}&table=cube&data_id={}_00_00_00", JVO_FITS_SERVER, JVO_FITS_DB, id) ;
                        return FITS::from_url(&data_id, &url);*/                      
                    }
        } ;

        match f.metadata() {
            Ok(metadata) => {   let len = metadata.len() ;
                                println!("{:?}, {} bytes", f, len);
                                
                                if len < FITS_CHUNK_LENGTH as u64 {
                                    return fits;
                                };
                        }
            Err(err) => {   println!("file metadata reading problem: {}", err);
                            return fits;
                    }
        } ;

        //OK, we have a FITS file with at least one chunk        
        println!("reading FITS header...") ;

        //let mut f = BufReader::with_capacity(FITS_CHUNK_LENGTH, f);

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
                },
                Err(err) => {                    
                    println!("CRITICAL ERROR reading FITS header: {}", err);
                    return fits;
                }
            } ;
        }           

        //test for frequency/velocity
        fits.frame_reference_unit() ;
        fits.frame_reference_type() ;

        if fits.restfrq > 0.0 {
            fits.has_frequency = true ;
        }        

        //compress the FITS header

        println!("{}/#hu = {}, {:?}", id, no_hu, fits);

        //next read the data HUD(s)        
        let frame_size: usize = fits.init_data_storage();
        let mut data: Vec<u8> = vec![0; frame_size];
        let mut frame: i32 = 0;

        //let mut f = BufReader::with_capacity(frame_size, f);

        println!("FITS cube frame size: {} bytes", frame_size);

        while frame < fits.depth {                                 
            //println!("requesting a cube frame {}/{}", frame, fits.depth);

            //read a FITS cube frame
            match f.read_exact(&mut data) {
                Ok(()) => {                                        
                    //process a FITS cube frame (endianness, half-float)
                    //println!("processing cube frame {}/{}", frame+1, fits.depth);
                    fits.add_cube_frame(&data, frame as usize);
                    frame = frame + 1 ;
                },
                Err(err) => {
                    println!("CRITICAL ERROR reading FITS data: {}", err);
                    return fits;
                }
            } ;            
        }        

        //we've gotten so far, we have the data
        fits.has_data = true ;        

        println!("{}: reading FITS data completed", id);

        //fits.make_fits_pixels_spectrum(f);

        //and lastly create a symbolic link in the FITSCACHE directory
        let filename = format!("{}/{}.fits", FITSCACHE, id);
        let cachefile = std::path::Path::new(&filename);      
        let _ = std::os::unix::fs::symlink(filepath, cachefile);     
        
        fits
    }

    fn from_url(id: &String, url: &String) -> FITS {
        let fits = FITS::new(id);                

        println!("FITS::from_url({})", url);

        fits
    }

    fn init_data_storage(&mut self) -> usize {
        if self.width == 0 || self.height == 0 || self.depth == 0 {
            return 0;
        }        

        let capacity = self.width * self.height ;

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

                self.bmaj = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("BMIN    = ") {
                let s = match scan_fmt!(line, "BMIN    = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bmin = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("BPA     = ") {
                let s = match scan_fmt!(line, "BPA     = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bpa = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("RESTFRQ = ") {
                let s = match scan_fmt!(line, "RESTFRQ = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.restfrq = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("RESTFREQ= ") {
                let s = match scan_fmt!(line, "RESTFREQ= {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.restfrq = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("OBSRA   = ") {
                let s = match scan_fmt!(line, "OBSRA   = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.obsra = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("OBSDEC  = ") {
                let s = match scan_fmt!(line, "OBSDEC  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.obsdec = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("DATAMIN = ") {
                let s = match scan_fmt!(line, "DATAMIN = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.datamin = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => std::f32::MIN
                }
            }

            if line.contains("DATAMAX = ") {
                let s = match scan_fmt!(line, "DATAMAX = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.datamax = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => std::f32::MAX
                }
            }

            if line.contains("BSCALE  = ") {
                let s = match scan_fmt!(line, "BSCALE  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bscale = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }            

            if line.contains("BZERO   = ") {
                let s = match scan_fmt!(line, "BZERO   = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.bzero = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("IGNRVAL = ") {
                let s = match scan_fmt!(line, "IGNRVAL = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.ignrval = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => std::f32::MIN
                }
            }

            if line.contains("CRVAL1  = ") {
                let s = match scan_fmt!(line, "CRVAL1  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crval1 = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRVAL2  = ") {
                let s = match scan_fmt!(line, "CRVAL2  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crval2 = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRVAL3  = ") {
                let s = match scan_fmt!(line, "CRVAL3  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crval3 = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CDELT1  = ") {
                let s = match scan_fmt!(line, "CDELT1  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.cdelt1 = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CDELT2  = ") {
                let s = match scan_fmt!(line, "CDELT2  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.cdelt2 = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CDELT3  = ") {
                let s = match scan_fmt!(line, "CDELT3  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.cdelt3 = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRPIX1  = ") {
                let s = match scan_fmt!(line, "CRPIX1  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crpix1 = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRPIX2  = ") {
                let s = match scan_fmt!(line, "CRPIX2  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crpix2 = match parse_f32(&s) {
                    Ok(x) => x,
                    Err(_) => 0.0
                }
            }

            if line.contains("CRPIX3  = ") {
                let s = match scan_fmt!(line, "CRPIX3  = {}", String) {
                    Some(x) => x,
                    _ => String::from("")
                };

                self.crpix3 = match parse_f32(&s) {
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

    fn add_cube_frame(&mut self, buf: &Vec<u8>, frame: usize) {                
        let mut rdr = Cursor::new(buf);
        let len = self.width * self.height ;

        match self.bitpix {
            8 => {
                //self.data_u8[frame].copy_from_slice(buf);
                self.data_u8[frame] = buf.clone();
                //println!("data_u8[{}]/len = {}", frame, self.data_u8[frame].len());
            },
            16 => {
                for _ in 0..len {
                    match rdr.read_i16::<BigEndian>() {
                        Ok(int16) => {
                            self.data_i16[frame].push(int16)
                        },
                        Err(err) => println!("BigEndian --> LittleEndian i16 conversion error: {}", err)
                    }
                }
            }
            32 => {
                for _ in 0..len {
                    match rdr.read_i32::<BigEndian>() {
                        Ok(int32) => {
                            self.data_i32[frame].push(int32)
                        },
                        Err(err) => println!("BigEndian --> LittleEndian i32 conversion error: {}", err)
                    }
                }
            }
            -32 => {
                for _ in 0..len {
                    match rdr.read_f32::<BigEndian>() {
                        Ok(float32) => {                            
                            let float16 = f16::from_f32(float32);
                            //println!("f32 = {} <--> f16 = {}", float32, float16);
                            self.data_f16[frame].push(float16);
                        },
                        Err(err) => println!("BigEndian --> LittleEndian f32 conversion error: {}", err)
                    }                    
                }                
            },
            -64 => {
                for _ in 0..len {
                    match rdr.read_f64::<BigEndian>() {
                        Ok(float64) => {
                            self.data_f64[frame].push(float64)
                        },
                        Err(err) => println!("BigEndian --> LittleEndian f64 conversion error: {}", err)
                    }
                }
            }
            _ => println!("unsupported bitpix: {}", self.bitpix)
        }      
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

#[derive(Debug)]
struct fits_session {    
    dataset_id: String,    
}

impl fits_session {
    fn new(id: &String) -> fits_session {
        let session = fits_session {
            dataset_id: id.clone(),
        } ;

        println!("allocating a new websocket session for {}", id);

        session
    }
}

impl Drop for fits_session {
    fn drop(&mut self) {
        println!("dropping a websocket session for {}", self.dataset_id);
    }
}

impl Actor for fits_session {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("websocket connection started for {}", self.dataset_id);
    }

    fn stopped(&mut self, ctx: &mut Self::Context) {
        println!("websocket connection stopped for {}", self.dataset_id);        
    }     
}

/// forward progress messages from FITS loading to the websocket
/*impl Handler<actix::Message> for fits_session {
    type Result = ();

    fn handle(&mut self, msg: actix::Message, ctx: &mut Self::Context) {
        //ctx.text(msg.0);
    }
}*/

// Handler for ws::Message messages
impl StreamHandler<ws::Message, ws::ProtocolError> for fits_session {
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

lazy_static! {
    static ref DATASETS: RwLock<HashMap<String, FITS>> = {
        RwLock::new(HashMap::new())
    };
}

#[cfg(not(feature = "server"))]
static SERVER_STRING: &'static str = "FITSWebQL v1.2.0";

static VERSION_STRING: &'static str = "SV2018-06-15.0";

#[cfg(not(feature = "server"))]
static SERVER_MODE: &'static str = "LOCAL";

#[cfg(feature = "server")]
static SERVER_MODE: &'static str = "SERVER";

fn remove_symlinks() {
    let cache = std::path::Path::new(FITSCACHE);

    for entry in cache.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            //remove a file if it's a symbolic link
            if let Ok(metadata) = entry.metadata() {
                let file_type = metadata.file_type();

                if file_type.is_symlink() {
                    println!("removing a symbolic link: {:?}", entry.file_name());
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

    let mut contents = String::from("[");
    let mut has_contents = false ;   
    
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
                    has_contents = true ;

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

                    contents.push_str(&dir_entry.to_string()) ;
                    contents.push(',');
                }

                //filter by .fits .FITS
                if metadata.is_file() {
                    let path = entry.path() ;
                    let ext = path.extension().and_then(std::ffi::OsStr::to_str) ; 
                    
                    if ext == Some("fits") || ext == Some("FITS") {
                        has_contents = true ;

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

                        contents.push_str(&file_entry.to_string()) ;
                        contents.push(',');
                    }
                }
            }
        }
    }

    if has_contents {
        //remove the last comma
        contents.pop() ;
    }

    contents.push(']');

    HttpResponse::Ok()
        .content_type("application/json")
        .body(format!("{{\"location\": \"{}\", \"contents\": {} }}", path.display(), contents))
}

fn directory_handler(req: HttpRequest) -> HttpResponse {
    let query = req.query();

    match query.get("dir") {
        Some(x) => get_directory(std::path::PathBuf::from(x)),
        None => get_home_directory()//default database
    }
}

// do websocket handshake and start actor
fn websocket_entry(req: HttpRequest) -> Result<HttpResponse> {
    let dataset_id: String = req.match_info().query("id").unwrap();

    let session = fits_session::new(&dataset_id);

    ws::start(req, session)
}

fn fitswebql_entry(req: HttpRequest) -> HttpResponse {
    let fitswebql_path: String = req.match_info().query("path").unwrap();
    
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
                    .body(format!("<p><b>Critical Error</b>: no {} available</p>", dataset));
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
    execute_fits(&fitswebql_path, &dir, &ext, &dataset_id, composite)
}

#[cfg(not(feature = "server"))]
fn execute_fits(fitswebql_path: &String, dir: &str, ext: &str, dataset_id: &Vec<&str>, composite: bool) -> HttpResponse {

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
            
            DATASETS.write().unwrap().insert(my_data_id.clone(), FITS::new(&my_data_id)); 

            //load FITS data in a new thread
            thread::spawn(move || {
                // some work here
                let filename = format!("{}/{}.{}", my_dir, my_data_id, my_ext);
                println!("loading FITS data from {}", filename); 

                let filepath = std::path::Path::new(&filename);           
                let fits = FITS::from_path(&my_data_id.clone(), filepath);

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
}

fn http_fits_response(fitswebql_path: &String, dataset_id: &Vec<&str>, composite: bool, has_fits: bool) -> HttpResponse {

    //let has_fits: bool = false ;//later on it should be changed to true; iterate over all datasets, setting it to false if not found    

    //build up an HTML response
    let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<link rel=\"icon\" href=\"favicon.ico\"/>\n");
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

    server::new(
        move || App::new()
        .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::GET).f(fitswebql_entry)})        
        .resource("/{path}/websocket/{id}", |r| {r.route().f(websocket_entry)})
        .resource("/get_directory", |r| {r.method(http::Method::GET).f(directory_handler)})
        .handler("/", fs::StaticFiles::new("htdocs").index_file(index_file)))
        .bind("localhost:8080").expect("Cannot bind to localhost:8080")
        .run();

    DATASETS.write().unwrap().clear();

    remove_symlinks();
}