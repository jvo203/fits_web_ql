extern crate actix_web;
use actix_web::*;

fn index(req: HttpRequest) -> &'static str {
    "FITSWebQL index page"
}

fn main() {
    server::new(
        || App::new()
        .resource("/test", |r| r.f(index))
        .handler(
            "/",
            fs::StaticFiles::new("htdocs")
                .index_file("almawebql.html")))
        .bind("localhost:8080").expect("Cannot bind to localhost:8080")
        .run();
}