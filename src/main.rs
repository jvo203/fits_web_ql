extern crate actix_web;

#[macro_use]
extern crate serde_json;

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
            if let Ok(metadata) = entry.metadata() {
                //println!("{:?}:{:?} filesize: {}", entry.path(), metadata, metadata.len());

                if metadata.is_dir() {
                    has_contents = true ;

                    let dir_entry = json!({
                        "type" : "dir",
                        "name" : format!("{:?}", entry.file_name()),
                        "last_modified" : format!("{:?}", metadata.modified())
                    });

                    println!("{}", dir_entry.to_string());

                    contents.push_str(&dir_entry.to_string()) ;

                    //contents.push_str(&format!("{{\"type\":\"dir\",\"name\":{:?},\"last_modified\":\"{:?}\"}},", entry.file_name(), metadata.modified() )) ;
                }

                //filter by .fits .FITS
                /*if metadata.is_file() {
                    println!("extension: {:?}", entry.path().extension()) ;

                    has_contents = true ;
                    contents.push_str(&format!("{{\"type\":\"file\",\"name\":{:?},\"size\":{},\"last_modified\":\"{:?}\"}},", entry.file_name(), metadata.len(), metadata.modified() )) ;
                }*/
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
    
    let db = match query.get("db") {
        Some(x) => {x},
        None => {"alma"}//default database
    };

    let table = match query.get("table") {
        Some(x) => {x},
        None => {"cube"}//default table
    };

    let dataset_id = match query.get("datasetId") {
        Some(x) => {x},
        None => {return HttpResponse::NotFound()
            .content_type("text/html")
            .body(format!("<p><b>Critical Error</b>: no datasetId available</p>"));}
    };

    //execute_fits(&fitswebql_path, &db, &table, &dataset_id)
    HttpResponse::Ok()
        .content_type("text/html")
        .body(format!("FITSWebQL path: {}, db: {}, table: {}, dataset_id: {}", fitswebql_path, db, table, dataset_id))
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
        .handler("/",fs::StaticFiles::new("htdocs").index_file(index_file)))
        .bind("localhost:8080").expect("Cannot bind to localhost:8080")
        .run();
}