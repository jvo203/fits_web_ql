extern crate actix_web;
use actix_web::*;

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
    #[cfg(not(feature = "local"))]
    let index_file = "almawebql.html" ;

    #[cfg(feature = "local")]
    let index_file = "fitswebql.html" ;

    server::new(
        move || App::new()
        .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::GET).f(fitswebql_entry)})
        .resource("/{path}/FITSWebQL.html", |r| {r.method(http::Method::PUT).f(fitswebql_entry)})
        .handler("/",fs::StaticFiles::new("htdocs").index_file(index_file)))
        .bind("localhost:8080").expect("Cannot bind to localhost:8080")
        .run();
}