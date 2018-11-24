use memmap::Mmap;

pub fn from_path_mmap(id: &String, flux: &String, filepath: &std::path::Path, server: &Addr<Syn, server::SessionServer>) -> FITS {
        let mut fits = FITS::new(id, flux);            
        fits.is_dummy = false;

        //load data from filepath
        let f = match File::open(filepath) {
            Ok(x) => x,
            Err(x) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, x);
                return fits;                                           
            }
        } ;

        match f.metadata() {
            Ok(metadata) => {
                let len = metadata.len() ;                                

                println!("{:?}, {} bytes", f, len);                                
                                
                fits.filesize = len;

                if len < FITS_CHUNK_LENGTH as u64 {
                    return fits;
                };
            }                
            Err(err) => {
                println!("CRITICAL ERROR file metadata reading problem: {}", err);
                return fits;
            }
        } ;

        //OK, we have a FITS file with at least one chunk        
        println!("{}: reading FITS header...", id) ;

        //mmap the file
        let mmap = match unsafe { Mmap::map(&f) } {
            Ok(mmap) => mmap,
            Err(err) => {
                println!("CRITICAL ERROR failed to mmap {:?}: {}", filepath, err);
                return fits;
            }
        };

        if mmap.len() != fits.filesize as usize {
            println!("CRITICAL ERROR {:?}: length mismatch", filepath);
            return fits;
        };

        let mut header: Vec<u8> = Vec::new();
        let mut end: bool = false ;
        let mut no_hu: i32 = 0 ;
        let mut offset: usize = 0 ;

        while !end {
            //read a FITS chunk (get a slice from mmap)
            let chunk : &[u8] = &mmap[offset .. offset+FITS_CHUNK_LENGTH];
            offset += FITS_CHUNK_LENGTH;
            
            no_hu = no_hu + 1;

            //parse a FITS header chunk
            end = fits.parse_fits_header_chunk(&chunk);
            header.extend_from_slice(&chunk);
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

        let freq_range = fits.get_frequency_range();
        fits.notify_frequency_range(&server, freq_range);

        //next read the data HUD(s)        
        let frame_size: usize = fits.init_data_storage();        

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

        //into_iter or into_par_iter
        (0 .. fits.depth).into_iter().for_each(|frame| {            
            //frame is i32
            let start = offset + (frame as usize) * frame_size ;
            let end = start + frame_size ;
            let data : &[u8] = &mmap[start..end];

            println!("processing cube frame {}/{}", frame+1, fits.depth);
            fits.process_cube_frame(&data, cdelt3, frame as usize);//cannot borrow mutable fits in parallel
            fits.send_progress_notification(&server, &"processing FITS".to_owned(), total, frame+1);
        });

        let mut frame: i32 = 0;
        while frame < fits.depth {                                 
            //println!("requesting a cube frame {}/{}", frame, fits.depth);

            //FITS data (mmap slice)
            let data: Vec<u8> = vec![0; frame_size];
            //take a slice at offset + frame*frame_size
            /*let start = offset + frame * frame_size ;
            let end = start + frame_size ;
            let data : &[u8] = &mmap[start..end];*/

            frame = frame + 1 ;

            //read a FITS cube frame
            /*match f.read_exact(&mut data) {
                Ok(()) => {                                        
                    //process a FITS cube frame (endianness, half-float)
                    //println!("processing cube frame {}/{}", frame+1, fits.depth);
                    fits.process_cube_frame(&data, cdelt3, frame as usize);                    
                    frame = frame + 1 ;
                    fits.send_progress_notification(&server, &"processing FITS".to_owned(), total, frame);
                },
                Err(err) => {
                    println!("CRITICAL ERROR reading FITS data: {}", err);
                    return fits;
                }
            } ;*/           
        }        

        //we've gotten so far, we have the data, pixels, mask and spectrum
        fits.has_data = true ;

        if !fits.pixels.is_empty() {
            //sort the pixels in parallel using rayon
            let mut ord_pixels = fits.pixels.clone();
            //println!("unordered pixels: {:?}", ord_pixels);

            let start = precise_time::precise_time_ns();
            ord_pixels.par_sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Equal));
            let stop = precise_time::precise_time_ns();

            println!("[pixels] parallel sorting time: {} [ms]", (stop-start)/1000000);

            fits.make_histogram(&ord_pixels);

            if fits.flux == "" {
                fits.histogram_classifier();
            };
        };

        fits.send_progress_notification(&server, &"processing FITS done".to_owned(), 0, 0);
        println!("{}: reading FITS data completed", id);

        //and lastly create a symbolic link in the FITSCACHE directory
        let filename = format!("{}/{}.fits", FITSCACHE, id);
        let cachefile = std::path::Path::new(&filename);      
        let _ = std::os::unix::fs::symlink(filepath, cachefile);

        fits
    }

    fn read_from_cache_mmap(&mut self, filepath: &std::path::Path, frame_size: usize, cdelt3: f32, server: &Addr<Syn, server::SessionServer>) -> bool {
        //mmap the half-float file

        //load data from filepath
        let f = match File::open(filepath) {
            Ok(x) => x,
            Err(err) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, err);
                return false;      
            }
        } ;

        let mmap = match unsafe { Mmap::map(&f) } {
            Ok(mmap) => mmap,
            Err(err) => {
                println!("CRITICAL ERROR failed to mmap {:?}: {}", filepath, err);
                return false ;
            }
        };

        let total = self.depth;

        (0 .. self.depth).into_iter().for_each(|frame| {            
            //frame is i32
            let start = (frame as usize) * frame_size ;
            let end = start + frame_size ;
            let data : &[u8] = &mmap[start..end];

            let mut rdr = Cursor::new(data);

            let len = data.len() / 2 ;
            let mut sum : f32 = 0.0 ;
            let mut count : i32 = 0 ;

            for i in 0..len {                
                match rdr.read_u16::<LittleEndian>() {
                    Ok(u16) => {                            
                        let float16 = f16::from_bits(u16);
                        self.data_f16[frame as usize].push(float16);

                        let tmp = self.bzero + self.bscale * float16.to_f32() ;
                        if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                            self.pixels[i as usize] += tmp * cdelt3;                                
                            self.mask[i as usize] = true ;

                            sum += tmp ;
                            count += 1 ;                                
                        }
                    },
                    Err(err) => println!("LittleEndian --> LittleEndian u16 conversion error: {}", err)
                }
            }

            //mean and integrated intensities @ frame
            if count > 0 {
                self.mean_spectrum[frame as usize] = sum / (count as f32) ;
                self.integrated_spectrum[frame as usize] = sum * cdelt3 ;
            }
            
            self.send_progress_notification(&server, &"processing FITS".to_owned(), total, frame+1);
        });

        return true;
    }

{
    let gather_f16 : Vec<_> = (0 .. self.depth).into_par_iter()./*for_each*/map(|frame| {            
            //frame is i32
            let start = (frame as usize) * frame_size ;
            let end = start + frame_size ;
            let data : &[u8] = &mmap[start..end];

            let mut rdr = Cursor::new(data);

            let len = data.len() / 2 ;
            let mut sum : f32 = 0.0 ;
            let mut count : i32 = 0 ;

            let mut data_f16 : Vec<f16> = Vec::with_capacity(len) ; 

            for i in 0..len {                
                match rdr.read_u16::<LittleEndian>() {
                    Ok(u16) => {                            
                        let float16 = f16::from_bits(u16);
                        //self.data_f16[frame as usize].push(float16);
                        data_f16.push(float16);

                        let tmp = self.bzero + self.bscale * float16.to_f32() ;
                        if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                            /*self.pixels[i as usize] += tmp * cdelt3;                                
                            self.mask[i as usize] = true ;*/

                            sum += tmp ;
                            count += 1 ;                                
                        }
                    },
                    Err(err) => println!("LittleEndian --> LittleEndian u16 conversion error: {}", err)
                }
            }

            //mean and integrated intensities @ frame
            /*if count > 0 {
                self.mean_spectrum[frame as usize] = sum / (count as f32) ;
                self.integrated_spectrum[frame as usize] = sum * cdelt3 ;
            }*/
            
            self.send_progress_notification(&server, &"processing FITS".to_owned(), total, frame+1);
            data_f16
        }).collect();

        self.data_f16 = gather_f16 ;

        //calculate pixels, mask and *_spectrum in a separate loop
}

//then deal with processing the data (sequentially for the time being)
        //the ispc-accelerated serial version (needs to be parallelised)
        for frame in 0..self.depth {
            //no mistake here, the initial ranges are supposed to be broken
            let mut frame_min = std::f32::MAX;
            let mut frame_max = std::f32::MIN;

            let mut mean_spectrum = 0.0_f32;
            let mut integrated_spectrum = 0.0_f32;

            let vec = &self.data_f16[frame as usize];            

            let mut references: [f32; 4] = [frame_min, frame_max, mean_spectrum, integrated_spectrum];

            let vec_ptr = vec.as_ptr() as *mut i16;
            let vec_len = vec.len() ;

            let mask_ptr = self.mask.as_ptr() as *mut u8;
            let mask_len = self.mask.len() ;

            unsafe {                    
                let vec_raw = slice::from_raw_parts_mut(vec_ptr, vec_len);
                let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                //make_image_spectrumF16_minmax( vec_raw.as_mut_ptr(), self.bzero, self.bscale, self.datamin, self.datamax, cdelt3, self.pixels.as_mut_ptr(), mask_raw.as_mut_ptr(), vec_len as u32, references.as_mut_ptr());  
            }

            frame_min = references[0] ;
            frame_max = references[1] ;
            mean_spectrum = references[2] ;
            integrated_spectrum = references[3] ;

            //println!("frame {}, references: {:?}", frame, references);

            /*self.mean_spectrum[frame as usize] = mean_spectrum ;
            self.integrated_spectrum[frame as usize] = integrated_spectrum ;
            self.dmin = self.dmin.min(frame_min);
            self.dmax = self.dmax.max(frame_max);*/

            //self.send_progress_notification(&server, &"processing FITS".to_owned(), total, frame+1);
        }

        let stop2 = precise_time::precise_time_ns();

        println!("[read_from_cache_par] processing time: {} [ms]", (stop2-stop)/1000000);


fn get_molecules_lock(req: HttpRequest<WsSessionState>) -> Box<Future<Item=HttpResponse, Error=Error>> {
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
        if freq_start == 0.0 || freq_end == 0.0 {
            let datasets = DATASETS.read();//.unwrap();

            println!("[get_molecules] obtained read access to <DATASETS>, trying to get read access to {}", dataset_id);

            let fits = match datasets.get(dataset_id).unwrap().try_read_for(time::Duration::from_millis(LONG_POLL_TIMEOUT)) {
                Some(x) => x,
                None => {
                    println!("[get_molecules]: RwLock timeout, cannot obtain a read access to {}", dataset_id);

                    return result(Ok(HttpResponse::Accepted()
                    .content_type("text/html")
                    .body(format!("<p><b>RwLock timeout</b>: {} not available yet</p>", dataset_id))))
                    .responder();
                }
            };

            println!("[get_molecules] obtained read access to {}, has_header = {}", dataset_id, fits.has_header);

            if fits.has_header {
                //get the freq_start, freq_end range from FITS
                let (freq_start, freq_end) = fits.get_frequency_range();
                println!("[get_molecules] frequency range {} - {} GHz", freq_start, freq_end);

                let content = fetch_molecules(freq_start, freq_end);                

                HttpResponse::Ok()
                    .content_type("application/json")
                    .body(format!("{{\"molecules\" : {}}}", content))
            }        
            else {            
                HttpResponse::NotFound()
                    .content_type("text/html")
                    .body(format!("<p><b>Critical Error</b>: spectral lines not found</p>"))            
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

#[cfg(not(feature = "server"))]
fn execute_fits_global_lock(fitswebql_path: &String, dir: &str, ext: &str, dataset_id: &Vec<&str>, composite: bool, flux: &str, server: &Addr<Syn, server::SessionServer>) -> HttpResponse {
    println!("calling execute_fits for {:?}", dataset_id);
    //get fits location    

    //launch FITS threads
    let mut has_fits: bool = true ;

    //for each dataset_id
    for i in 0..dataset_id.len() {
        let data_id = dataset_id[i];        
        
        println!("execute_fits: waiting for a DATASETS write lock for {}", data_id);
        let mut datasets = DATASETS.write();//.unwrap();                

        //does the entry exist in the datasets hash map?
        /*let has_entry = {
            println!("execute_fits/has_entry: waiting for a DATASETS read lock for {}", data_id);
            let datasets = DATASETS.read();
            datasets.contains_key(data_id) 
        } ;*/

        //if it does not exist set has_fits to false and load the FITS data        
        if ! /*has_entry*/ datasets.contains_key(data_id) {
            has_fits = false ;            

            let my_dir = dir.to_string();
            let my_data_id = data_id.to_string();
            let my_ext = ext.to_string();
            let my_server = server.clone();                                                           
            let my_flux = flux.to_string();
                                                        
            //println!("execute_fits: waiting for a DATASETS write lock for {}", my_data_id);
            //let mut datasets = DATASETS.write();                
            datasets.insert(my_data_id.clone(), Arc::new(RwLock::new(Box::new(fits::FITS::new(&my_data_id, &my_flux)))));

            //load FITS data in a new thread
            thread::spawn(move || {
                let filename = format!("{}/{}.{}", my_dir, my_data_id, my_ext);
                println!("loading FITS data from {}", filename);                 
                
                //let datasets = DATASETS.read();//.unwrap();
                //let mut fits = /*match*/ datasets.get(&my_data_id).unwrap().write();                
                /* {                    
                    Ok(x) => x,                        
                    Err(err) => {                        
                        println!("{}: cannot obtain a mutable reference to {}", err, my_data_id);
                        return;
                    }                
                };*/                

                //println!("obtained a mutable reference to {}", my_data_id);

                /*let mut fits = {
                    let datasets = DATASETS.read();
                    datasets.get(&my_data_id).unwrap().write()
                };*/

                /*let filepath = std::path::Path::new(&filename);
                fits.load_from_path(&my_data_id.clone(), filepath, &my_server);*/
            });
        }
        else {
            //update the timestamp
            println!("execute_fits/timestamp: waiting for a DATASETS read lock for {}", data_id);
            //let datasets = DATASETS.read();        
            let dataset = datasets.get(data_id).unwrap().read();//.unwrap() ;
            
            has_fits = has_fits && dataset.has_data ;
            *dataset.timestamp.write()/*.unwrap()*/ = SystemTime::now() ;

            println!("updated an access timestamp for {}", data_id);
        } ;
    } ;

    http_fits_response(&fitswebql_path, &dataset_id, composite, has_fits)
}

fn read_from_cache(&mut self, filepath: &std::path::Path, frame_size: usize, cdelt3: f32, server: &Addr<Syn, server::SessionServer>) -> bool {
        //mmap the half-float file

        //load data from filepath
        let mut f = match File::open(filepath) {
            Ok(x) => x,
            Err(err) => {
                println!("CRITICAL ERROR {:?}: {:?}", filepath, err);
                return false;      
            }
        } ;

        let total = self.depth;
        let mut frame: i32 = 0;

        while frame < self.depth {                                 
            //println!("requesting a cube frame {}/{}", frame, fits.depth);
            let mut data: Vec<u8> = vec![0; frame_size];

            //read a FITS cube frame (half-float only)
            match f.read_exact(&mut data) {
                Ok(()) => {                                        
                    //println!("processing cube frame {}/{}", frame+1, fits.depth);

                    let len = data.len() / 2 ;
                    let mut sum : f32 = 0.0 ;
                    let mut count : i32 = 0 ;

                    let mut rdr = Cursor::new(data);

                    for i in 0..len {                
                        match rdr.read_u16::<LittleEndian>() {
                            Ok(u16) => {                      
                                let float16 = f16::from_bits(u16);
                                self.data_f16[frame as usize].push(float16);

                                let tmp = self.bzero + self.bscale * float16.to_f32() ;
                                if tmp.is_finite() && tmp >= self.datamin && tmp <= self.datamax {
                                    self.pixels[i as usize] += tmp * cdelt3;    
                                    self.mask[i as usize] = true ;

                                    sum += tmp ;
                                    count += 1 ;                                
                                }
                            },
                            Err(err) => println!("LittleEndian --> LittleEndian u16 conversion error: {}", err)
                        }
                    }

                    //mean and integrated intensities @ frame
                    if count > 0 {
                        self.mean_spectrum[frame as usize] = sum / (count as f32) ;
                        self.integrated_spectrum[frame as usize] = sum * cdelt3 ;
                    }

                    frame = frame + 1 ;
                    self.send_progress_notification(&server, &"processing FITS".to_owned(), total, frame);
                },
                Err(err) => {
                    println!("CRITICAL ERROR reading FITS data: {}", err);
                    return false;
                }
            } ;            
        }

        return true;
    }

    //median of a data histogram: 0.1552124 at pos 279, mad_p = 1.4211754, mad_n = 0.785294

    /*let (tx, rx) = mpsc::channel();               

        transfer.progress_function(|dltotal, dlnow, ultotal, ulnow| {
            println!("{}/{}", dlnow, dltotal);
            
            if (dltotal > 0.0) && (dlnow == dltotal) {
                false
            }
            else {
                true
            }
        }).unwrap();

        transfer.header_function(|data| {
            let header = String::from_utf8(data.to_vec()).unwrap();
            println!(">{}", header);

            true
        }).unwrap();

        transfer.write_function(move |data| {
            println!("curl received {} bytes", data.len());

            match cachefile.write_all(data) {
                Ok(_) => {},
                Err(err) => {
                    println!("cannot append to the temporary FITS file: {}", err);                    
                }
            };

            //send data for processing            
            match tx.send(data.to_vec()) {
                Ok(()) => {},
                Err(err) => {
                    println!("file download thread: {}", err);                    
                }
            };

            Ok(data.len())
        }).unwrap();

        transfer.perform().unwrap();
        
        let mut buffer: Vec<u8> = Vec::new(); 

        for data in rx {
            for b in data {
                buffer.push(b);
            };
        };*/   

        //===========================================================


#[cfg(feature = "server")]
const GARBAGE_COLLECTION_TIMEOUT: u64 = 60;//[s]; a dataset inactivity timeout

#[cfg(not(feature = "server"))]
const GARBAGE_COLLECTION_TIMEOUT: u64 = 5;//[s]; a dataset inactivity timeout

const DUMMY_DATASET_TIMEOUT: u64 = 24*60*60;//[s]; 24 hours

fn garbage_collection(/*server: &Addr<server::SessionServer>*/) {
    let datasets = DATASETS.read();

    for key in datasets.keys() {        
        let dataset = datasets.get(key).unwrap().read() ;

        let now = SystemTime::now();
        let elapsed = now.duration_since(*dataset.timestamp.read());

        let timeout = if dataset.is_dummy {
            std::time::Duration::new(DUMMY_DATASET_TIMEOUT, 0)
        } else {
            std::time::Duration::new(GARBAGE_COLLECTION_TIMEOUT, 0)
        };

        match elapsed {
            Ok(elapsed) => {
                println!("key: {}, elapsed time: {:?}", key, elapsed);

                if elapsed > timeout {
                    println!("{} marked as a candidate for deletion", key);

                    //check if there are no active sessions
                /*let _ = server.do_send(server::Remove {
                    dataset_id: key.clone(),
                });*/

                    //a deadlock!!!
                    DATASETS.write().remove(key);//a previous read lock is preventing a write lock
                }
            },
            Err(err) => println!("SystemTime::duration_since failed: {}", err),
        }
    }

    /*for dataset in datasets.values() {
        println!("key: {}", dataset.read().dataset_id);
    }*/
}

    //let addr = &server.clone();
    /*thread::spawn(move ||{
        loop {            
            thread::sleep(time::Duration::from_secs(10));

            garbage_collection();
        }
    });*/


                    if can_remove {                        
                        //molecules.write().remove(&msg.dataset_id);
                        //DATASETS.write().remove(&msg.dataset_id);

                        /*let dataset = { DATASETS.write().remove(&msg.dataset_id); };

                        match dataset {
                            Some(_) => {
                                println!("{} has been expunged from memory", &msg.dataset_id);
                            },
                            None => {
                                println!("{} could not be removed from the HashMap", &msg.dataset_id);
                            },
                        };*/
                    }

 fn data_to_luminance_f16(&self, frame: usize) -> Vec<u8> {
        //calculate white, black, sensitivity from the data_histogram
        let u = 7.5_f32 ;
        //let v = 15.0_f32 ;

        let median = *self.data_median.read() ;
        let black = self.dmin.max((*self.data_median.read()) - u * (*self.data_mad_n.read())) ;
        let white = self.dmax.min((*self.data_median.read()) + u * (*self.data_mad_p.read())) ;
        let sensitivity = 1.0 / (white - black) ;

        //interfacing with Intel SPMD Program Compiler
        let vec = &self.data_f16[frame];
        let ptr = vec.as_ptr() as *mut i16;
        let len = vec.len();

        let mask_ptr = self.mask.as_ptr() as *mut u8;
        let mask_len = self.mask.len() ;

        let mut y: Vec<u8> = vec![0; len];
        //end of interface

        match self.flux.as_ref() {            
            "linear" => {
                let slope = 1.0 / (white - black) ;

                unsafe {                    
                    let mut raw = slice::from_raw_parts_mut(ptr, len);
                    let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                    data_to_luminance_f16_linear( raw.as_mut_ptr(), mask_raw.as_mut_ptr(), self.bzero, self.bscale, black, slope, y.as_mut_ptr(), len as u32);
                }

                y
                /*self.data_f16[frame].par_iter()
                    .zip(self.mask.par_iter())
                        .map(|(x, m)| {
                            if *m {                         
                                let x = self.bzero + self.bscale * (*x).to_f32();       
                                let pixel = num::clamp( (x - black) * slope, 0.0, 1.0);
                                (255.0*pixel) as u8
                            }                            
                            else {
                                0
                            }
                        })                        
                        .collect()*/
            },
            "logistic" => {
                unsafe {                    
                    let mut raw = slice::from_raw_parts_mut(ptr, len);
                    let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                    data_to_luminance_f16_logistic( raw.as_mut_ptr(), mask_raw.as_mut_ptr(), self.bzero, self.bscale, median, sensitivity, y.as_mut_ptr(), len as u32);
                }

                y
                /*self.data_f16[frame].par_iter()
                    .zip(self.mask.par_iter())
                        .map(|(x, m)| {
                            if *m {                      
                                let x = self.bzero + self.bscale * (*x).to_f32();          
                                let pixel = num::clamp( 1.0/( 1.0 + (-6.0 * (x - median) * sensitivity).exp() ), 0.0, 1.0);
                                (255.0*pixel) as u8
                            }                            
                            else {
                                0
                            }
                        })
                        .collect()*/                       
            },
            "ratio" => {
                unsafe {                    
                    let mut raw = slice::from_raw_parts_mut(ptr, len);
                    let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                    data_to_luminance_f16_ratio( raw.as_mut_ptr(), mask_raw.as_mut_ptr(), self.bzero, self.bscale, black, sensitivity, y.as_mut_ptr(), len as u32);
                }

                y        
                /*self.data_f16[frame].par_iter()
                    .zip(self.mask.par_iter())
                        .map(|(x, m)| {
                            if *m {                                            
                                let x = self.bzero + self.bscale * (*x).to_f32();                    
                                let pixel = 5.0 * (x - black) * sensitivity;
                                
                                if pixel > 0.0 {
                                    (255.0*pixel/(1.0 + pixel)) as u8
                                }
                                else {
                                    0
                                }                                
                            }                            
                            else {
                                0
                            }
                        })
                        .collect()*/
            },
            "square" => {
                unsafe {                    
                    let mut raw = slice::from_raw_parts_mut(ptr, len);
                    let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                    data_to_luminance_f16_square( raw.as_mut_ptr(), mask_raw.as_mut_ptr(), self.bzero, self.bscale, black, sensitivity, y.as_mut_ptr(), len as u32);
                }

                y
                /*self.data_f16[frame].par_iter()
                    .zip(self.mask.par_iter())
                        .map(|(x, m)| {
                            if *m {                
                                let x = self.bzero + self.bscale * (*x).to_f32();                     
                                let pixel = (x - black) * sensitivity;
                                
                                if pixel > 0.0 {
                                    (255.0*num::clamp(pixel*pixel, 0.0, 1.0)) as u8  
                                }
                                else {
                                    0
                                }                                
                            }                            
                            else {
                                0
                            }
                        })
                        .collect()*/
            },            
            //by default assume "legacy"
            _ => {
                let lmin = (0.5f32).ln() ;
                let lmax = (1.5f32).ln() ;

                unsafe {                    
                    let mut raw = slice::from_raw_parts_mut(ptr, len);
                    let mask_raw = slice::from_raw_parts_mut(mask_ptr, mask_len);

                    data_to_luminance_f16_legacy( raw.as_mut_ptr(), mask_raw.as_mut_ptr(), self.bzero, self.bscale, self.dmin, self.dmax, lmin, lmax, y.as_mut_ptr(), len as u32);
                }

                y
                /*self.data_f16[frame].par_iter()
                    .zip(self.mask.par_iter())
                        .map(|(x, m)| {
                            if *m {          
                                let x = self.bzero + self.bscale * (*x).to_f32();               
                                let pixel = 0.5 + (x - self.dmin) / (self.dmax - self.dmin) ;
                                
                                if pixel > 0.0 {
                                    (255.0*num::clamp((pixel.ln() - lmin) / (lmax - lmin), 0.0, 1.0)) as u8  
                                }
                                else {
                                    0
                                }                                
                            }                            
                            else {
                                0
                            }
                        })
                        .collect()*/
            },
        }
    }

//VideoSession handling (eventually merged with the UserSession, not needed anymore)


struct VideoSession {    
    dataset_id: Vec<String>,
    session_id: Uuid,
    timestamp: std::time::Instant,
    log: std::io::Result<File>,
    hevc: std::io::Result<File>,    
    param: *mut x265_param,//HEVC param
    enc: *mut x265_encoder,//HEVC context
    pic: *mut x265_picture,//HEVC picture    
    width: u32,
    height: u32, 
}


impl VideoSession {
    pub fn new(id: &Vec<String>) -> VideoSession {
        let uuid = Uuid::new_v4();

        #[cfg(not(feature = "server"))]
        let filename = format!("/dev/null");

        #[cfg(feature = "server")]
        let filename = format!("{}/{}.log", LOG_DIRECTORY, uuid);

        let log = File::create(filename);

        #[cfg(not(feature = "server"))]
        let filename = format!("/dev/null");

        #[cfg(feature = "server")]
        let filename = format!("{}/{}.hevc", LOG_DIRECTORY, uuid);

        let hevc = File::create(filename);

        let session = VideoSession {
            dataset_id: id.clone(),            
            session_id: uuid,
            timestamp: std::time::Instant::now(),   
            log: log,
            hevc: hevc,                  
            param: ptr::null_mut(),
            enc: ptr::null_mut(),
            pic: ptr::null_mut(),            
            width: 0,
            height: 0,
        } ;

        println!("allocating a new websocket session for {:?}", id);

        session
    }
}

impl Drop for VideoSession {
    fn drop(&mut self) {
        println!("dropping a websocket video session for {:?}", self.dataset_id);        

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

impl Actor for VideoSession {
    type Context = ws::WebsocketContext<Self, WsSessionState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("video websocket connection started for {:?}/{}", self.dataset_id, self.session_id);

        ctx.run_interval(std::time::Duration::new(10,0), |act, ctx| {
            if std::time::Instant::now().duration_since(act.timestamp) > std::time::Duration::new(WEBSOCKET_TIMEOUT,0) {        
                println!("video websocket inactivity timeout for {:?}", act.dataset_id);
                
                ctx.stop();
            }            
        });

        ctx.run_later(std::time::Duration::new(10,0), |_, ctx| {
            ctx.text("[heartbeat]");
        });
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        println!("stopping a video websocket connection for {:?}/{}", self.dataset_id, self.session_id);

        Running::Stop
    }     
}

// Handler for ws::Message messages
impl StreamHandler<ws::Message, ws::ProtocolError> for VideoSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        //println!("VIDEO WEBSOCKET MESSAGE: {:?}", msg);

        match msg {
            ws::Message::Ping(msg) => ctx.pong(&msg),
            ws::Message::Text(text) => {                
                if (&text).contains("[heartbeat]") {
                    self.timestamp = std::time::Instant::now();                    

                    //schedule the next heartbeat request
                    ctx.run_later(std::time::Duration::new(10,0), |_, ctx| {
                        ctx.text("[heartbeat]");
                    });
                }
            },
            _ => {},
        }
    }
}


fn video_websocket_entry(req: &HttpRequest<WsSessionState>) -> Result<HttpResponse> {
    let dataset_id_orig: String = req.match_info().query("id").unwrap();

    //dataset_id needs to be URI-decoded
    let dataset_id = match percent_decode(dataset_id_orig.as_bytes()).decode_utf8() {
        Ok(x) => x.into_owned(),
        Err(_) => dataset_id_orig.clone(),
    };

    let id: Vec<String> = dataset_id.split(',').map(|s| s.to_string()).collect();

    println!("new video websocket request for {:?}", id);

    ws::start(req, VideoSession::new(&id))
}

# switching between VP9 and HEVC streaming video during development (testing purposes only, not recommended in normal use; by default streaming video is handled by HEVC and still images by VP9)

cargo run --features 'server production vp9' --release

cargo run --features 'server production hevc' --release

+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++=
for CentOS 7 you may try the alonid repository:

https://copr.fedorainfracloud.org/coprs/alonid/llvm-3.9.0/

as root add the following contents to /etc/yum.repos.d/epel.repo

[alonid-llvm-3.9.0]

name=Copr repo for llvm-3.9.0 owned by alonid

baseurl=https://copr-be.cloud.fedoraproject.org/results/alonid/llvm-3.9.0/epel-7-$basearch/

type=rpm-md

skip_if_unavailable=True

gpgcheck=1

gpgkey=https://copr-be.cloud.fedoraproject.org/results/alonid/llvm-3.9.0/pubkey.gpg

repo_gpgcheck=0

enabled=1

enabled_metadata=1

, then execute

sudo yum install clang-3.9.0

and add /opt/llvm-3.9.0/bin to your $PATH

and set LIBCLANG_PATH as well:

export PATH=/opt/llvm-3.9.0/bin:$PATH

export LIBCLANG_PATH=/opt/llvm-3.9.0/lib64
+++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++=


    fn make_j2k_image(&mut self) {
        //check if the .img binary image file is already in the IMAGECACHE

        let filename = format!("{}/{}.img", IMAGECACHE, self.dataset_id.replace("/", "_"));
        let filepath = std::path::Path::new(&filename);

        if filepath.exists() {
            return;
        }

        let start = precise_time::precise_time_ns();

        let mut image_frame: Vec<u8> = Vec::new();

        let mut w = self.width as u32;
        let mut h = self.height as u32;
        let pixel_count = (w as u64) * (h as u64);

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

        let mut y: Vec<u8> = self.pixels_to_luminance(
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
            &self.flux,
        );

        {
            let start = precise_time::precise_time_ns();

            let mut dst = vec![0; (w as usize) * (h as usize)];
            self.resize_and_invert(&y, &mut dst, w, h, libyuv_FilterMode_kFilterBox);
            y = dst;

            let stop = precise_time::precise_time_ns();

            println!(
                "JPEG2000 image frame inverting/downscaling time: {} [ms]",
                (stop - start) / 1000000
            );
        }

        let alpha_frame = {
            let start = precise_time::precise_time_ns();

            //invert/downscale the mask (alpha channel) without interpolation
            let mut alpha = vec![0; (w as usize) * (h as usize)];

            self.resize_and_invert(&self.mask, &mut alpha, w, h, libyuv_FilterMode_kFilterNone);

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

        //compress y, copy the output to image_frame
        let mut l_param: ffi::opj_cparameters_t = unsafe { mem::zeroed() };
        unsafe { ffi::opj_set_default_encoder_parameters(&mut l_param) };

        let num_comps: u32 = 1;
        let image_width = w;
        let image_height = h;

        //one tile for now
        let tile_width = image_width;
        let tile_height = image_height;

        let comp_prec = 8;
        let irreversible = true;
        let quality_loss = true;
        let cblockw_init = 64;
        let cblockh_init = 64;
        let numresolution = 6;
        let offsetx: u32 = 0;
        let offsety: u32 = 0;

        let l_nb_tiles_width = (offsetx + image_width + tile_width - 1) / tile_width;
        let l_nb_tiles_height = (offsety + image_height + tile_height - 1) / tile_height;
        let l_nb_tiles = l_nb_tiles_width * l_nb_tiles_height;
        let l_data_size = tile_width * tile_height * num_comps * (comp_prec / 8);

        println!(
            "l_nb_tiles_width: {}, l_nb_tiles_height: {}, l_nb_tiles: {}, l_data_size: {}",
            l_nb_tiles_width, l_nb_tiles_height, l_nb_tiles, l_data_size
        );

        if quality_loss {
            l_param.tcp_numlayers = 1;
            l_param.cp_fixed_quality = 1;
            l_param.tcp_distoratio[0] = 20.0;
        }

        /* tile definitions parameters */
        /* position of the tile grid aligned with the image */
        l_param.cp_tx0 = 0;
        l_param.cp_ty0 = 0;
        /* tile size, we are using tile based encoding */
        l_param.tile_size_on = ffi::OPJ_TRUE as i32;
        l_param.cp_tdx = tile_width as i32;
        l_param.cp_tdy = tile_height as i32;

        /* code block size */
        l_param.cblockw_init = cblockw_init;
        l_param.cblockh_init = cblockh_init;

        /* use irreversible encoding ?*/
        l_param.irreversible = irreversible as i32;

        /* do not bother with mct, the rsiz is set when calling opj_set_MCT*/
        /*l_param.cp_rsiz = OPJ_STD_RSIZ;*/

        /* no cinema */
        /*l_param.cp_cinema = 0;*/

        /* no not bother using SOP or EPH markers, do not use custom size precinct */
        /* number of precincts to specify */
        /* l_param.csty = 0;*/
        /* l_param.res_spec = ... */
        /* l_param.prch_init[i] = .. */
        /* l_param.prcw_init[i] = .. */

        /* do not use progression order changes */
        /*l_param.numpocs = 0;*/
        /* l_param.POC[i].... */

        /* do not restrain the size for a component.*/
        /* l_param.max_comp_size = 0; */

        /* block encoding style for each component, do not use at the moment */
        /* J2K_CCP_CBLKSTY_TERMALL, J2K_CCP_CBLKSTY_LAZY, J2K_CCP_CBLKSTY_VSC, J2K_CCP_CBLKSTY_SEGSYM, J2K_CCP_CBLKSTY_RESET */
        /* l_param.mode = 0;*/

        /* number of resolutions */
        l_param.numresolution = numresolution;

        /* progression order to use*/
        /* OPJ_LRCP, OPJ_RLCP, OPJ_RPCL, PCRL, CPRL */
        l_param.prog_order = ffi::PROG_ORDER_OPJ_LRCP;

        /* no "region" of interest, more precisely component */
        /* l_param.roi_compno = -1; */
        /* l_param.roi_shift = 0; */

        /* we are not using multiple tile parts for a tile. */
        /* l_param.tp_on = 0; */
        /* l_param.tp_flag = 0; */

        /* image definition */

        let mut l_params: Vec<ffi::opj_image_cmptparm_t> = (0..num_comps)
            .into_iter()
            .map(|_| ffi::opj_image_cmptparm_t {
                dx: 1,
                dy: 1,
                h: image_height,
                w: image_width,
                sgnd: 0,
                prec: comp_prec,
                bpp: comp_prec,
                x0: offsetx,
                y0: offsety,
            }).collect();

        let mut l_codec = unsafe { ffi::opj_create_compress(ffi::CODEC_FORMAT_OPJ_CODEC_J2K) };

        if l_codec.is_null() {
            println!("error creating a JPEG2000 codec");
            return;
        }

        let mut l_image = unsafe {
            ffi::opj_image_tile_create(
                num_comps,
                l_params.as_mut_ptr(),
                ffi::COLOR_SPACE_OPJ_CLRSPC_GRAY,
            )
        };

        if l_image.is_null() {
            println!("error creating a JPEG2000 image");
            unsafe { ffi::opj_destroy_codec(l_codec) };

            return;
        }

        unsafe {
            (*l_image).x0 = offsetx;
            (*l_image).y0 = offsety;
            (*l_image).x1 = offsetx + image_width;
            (*l_image).y1 = offsety + image_height;
            (*l_image).color_space = ffi::COLOR_SPACE_OPJ_CLRSPC_GRAY;
        }

        let f = CString::new(format!("{}/test.j2k", IMAGECACHE)).unwrap();
        let mut l_stream = unsafe {
            ffi::opj_stream_create_default_file_stream(f.as_ptr(), ffi::OPJ_FALSE as i32)
        };

        if l_stream.is_null() {
            println!("error creating a JPEG2000 stream");
            unsafe { ffi::opj_destroy_codec(l_codec) };
            unsafe { ffi::opj_image_destroy(l_image) };

            return;
        }

        //start the compression process
        let ret = unsafe { ffi::opj_start_compress(l_codec, l_image, l_stream) }; //this line segfaults

        if ret > 0 {
            //compress each tile
            println!("JPEG2000 compression started");

        //end the compression process
        //unsafe { ffi::opj_end_compress(l_codec, l_stream) };
        } else {
            println!("error starting JPEG2000 compression");
        }

        //free up the J2K encoder
        unsafe {
            ffi::opj_destroy_codec(l_codec);
            ffi::opj_image_destroy(l_image);
            ffi::opj_stream_destroy(l_stream);
        }

        if image_frame.is_empty() {
            println!("JPEG2000 codec error: no image produced");
            return;
        }

        let stop = precise_time::precise_time_ns();

        println!(
            "JPEG2000 image compression time: {} [ms]",
            (stop - start) / 1000000
        );

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
            identifier: String::from("J2K"),
            width: w,
            height: h,
            image: image_frame,
            alpha: alpha_frame,
        };

        match serialize(&image_frame) {
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