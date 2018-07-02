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