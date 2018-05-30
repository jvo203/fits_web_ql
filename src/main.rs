extern crate actix_web;
use actix_web::*;

fn fitswebql_entry(req: HttpRequest) -> Result<String> {
    let fitswebql_path: String = req.match_info().query("path")?;
    
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
        None => {return Ok(format!("Critical Error: no datasetId available"));}
    };

    //execute_fits(&fitswebql_path, &db, &table, &dataset_id)
    Ok(format!("FITSWebQL path: {}, db: {}, table: {}, dataset_id: {}", fitswebql_path, db, table, dataset_id))
}

fn main() {
    #[cfg(default)]
    let index_file = "almawebql.html" ;

    #[cfg(local)]
    let index_file = "fitswebql.html" ;

    server::new(
        move || App::new()
        .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::GET).f(fitswebql_entry)})
        .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::PUT).f(fitswebql_entry)})
        .handler("/",fs::StaticFiles::new("htdocs").index_file(index_file)))
        .bind("localhost:8080").expect("Cannot bind to localhost:8080")
        .run();
}