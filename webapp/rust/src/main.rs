extern crate actix_web;

use std::collections::HashMap;
use std::env;

use actix_session::CookieSession;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};

use chrono::NaiveDateTime;

struct User {
    id: i64,
    nick_name: String,
    login_name: String,
    pass_hash: String,
}

// TODO db_column nameとの対応
struct Event {
    id: i64,
    title: String,
    public: bool,
    closed: bool,
    price: i64,

    total: i32,
    remains: i32,
    sheets: HashMap<String, Sheets>,
}

struct Sheets {
    total: i32,
    remains: i32,
    details: Vec<Sheet>,
    price: i64,
}

struct Sheet {
    id: i64,
    rank: String,
    num: i64,
    price: i64,

    mine: bool,
    reserved: bool,
    reserved_at: NaiveDateTime,
    reserved_at_unix: i64,
}

async fn get_dummy(req: HttpRequest) -> impl Responder {
    println!("{:?}", req);
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("Hello, actix_web.")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    env_logger::init();

    HttpServer::new(move || {
        App::new()
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            .wrap(middleware::Logger::default())
            .service(web::resource("/").route(web::get().to(get_dummy)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
