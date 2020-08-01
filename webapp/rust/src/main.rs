extern crate actix_web;

use std::env;

use actix_session::CookieSession;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};

struct User {
    id: i64,
    nick_name: String,
    login_name: String,
    pass_hash: String,
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
