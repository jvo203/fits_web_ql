use std::str::FromStr;
extern crate chrono;
extern crate actix_web;
extern crate half;

use std::thread;
use half::f16;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_json;

use std::time::{/*Duration,*/ SystemTime};
use std::collections::HashMap;
use std::sync::RwLock;

static FITSCACHE: &'static str = "FITSCACHE";
static NBINS: i32 = 1024;

#[derive(Debug)]
struct FITS {
        dataset_id: String,
        data_id: String,
        //basic header/votable        
        obj_name: String,        
        obs_date: String,
        timesys: String,
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
        width: i32,
        height: i32,
        depth: i32,
        polarisation: i32,
        data_u8: Vec<u8>,
        data_i16: Vec<i16>,        
        data_i32: Vec<i32>,
        data_f16: Vec<f16>,//half-float (short)
        data_f32: Vec<f32>,
        data_f64: Vec<f64>,
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
        timestamp: RwLock<SystemTime>,//last access time
}

lazy_static! {
    static ref DATASETS: RwLock<HashMap<String, FITS>> = {
        RwLock::new(HashMap::new())
    };
}

#[cfg(not(feature = "server"))]
static SERVER_STRING: &'static str = "FITSWebQL v1.2.0";

static VERSION_STRING: &'static str = "SV2018-06-08.0";

#[cfg(not(feature = "server"))]
static SERVER_MODE: &'static str = "LOCAL";

#[cfg(feature = "server")]
static SERVER_MODE: &'static str = "SERVER";

use actix_web::*;
use std::env;

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

            //load FITS data in a new thread
            thread::spawn(move || {
                // some work here
                let filepath = format!("{}/{}.{}", my_dir, my_data_id, my_ext);
                println!("loading FITS data from {}", filepath); 

                //let path = std::path::Path::new(&filepath);
                //let fits = FITS::from_path(path) ;
                //let mut datasets = DATASETS.write().unwrap();
                //datasets.insert(data_id.clone(), fits);            
            });
        }
        else {
            //update the timestamp
            let datasets = DATASETS.read().unwrap();
            let dataset = datasets.get(data_id).unwrap() ;
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
    #[cfg(not(feature = "server"))]
    let index_file = "fitswebql.html" ;

    #[cfg(feature = "server")]
    let index_file = "almawebql.html" ;    

    server::new(
        move || App::new()
        .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::GET).f(fitswebql_entry)})
        .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::PUT).f(fitswebql_entry)})
        .resource("/get_directory", |r| {r.method(http::Method::GET).f(directory_handler)})
        .handler("/", fs::StaticFiles::new("htdocs").index_file(index_file)))
        .bind("localhost:8080").expect("Cannot bind to localhost:8080")
        .run();
}