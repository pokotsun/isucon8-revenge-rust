extern crate actix_web;
extern crate sqlx;

use std::collections::HashMap;
use std::env;

use actix_session::{CookieSession, Session};
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};

use sqlx::mysql::{MySqlPool, MySqlQueryAs};

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

struct Reservation {
    id: i64,
    event_id: i64,
    sheet_id: i64,
    user_id: i64,
    reserved_at: NaiveDateTime,
    canceled_at: NaiveDateTime,

    event: Event,
    sheet_rank: String,
    sheet_num: i64,
    price: i64,
    reserved_at_unix: i64,
    canceled_at_unix: i64,
}

struct Administrator {
    id: i64,
    nick_name: String,
    login_name: String,
    pass_hash: String,
}

fn sess_user_id(session: &Session) -> Option<i64> {
    session.get::<i64>("user_id").ok().flatten()
}

fn sess_set_user_id(session: &Session, id: i64) -> Result<()> {
    session.set("user_id", id)?;
    Ok(())
}

fn sess_delete_user_id(session: &Session) {
    session.remove("user_id");
}

fn sess_administrator_id(session: &Session) -> Option<i64> {
    session.get::<i64>("administrator_id").ok().flatten()
}

fn sess_set_administrator_id(session: &Session, id: i64) -> Result<()> {
    session.set("administrator_id", id)?;
    Ok(())
}

fn sess_delete_administrator_id(session: &Session) {
    session.remove("administrator_id");
}

// TODO login_requiredの実装

// TODO admin_login_requiredの実装

#[derive(sqlx::FromRow)]
struct LoginUser {
    id: i64,
    nick_name: String,
}

async fn get_login_user(pool: &MySqlPool, session: &Session) -> Option<LoginUser> {
    let uid = sess_user_id(session)?;

    sqlx::query_as::<_, LoginUser>("SELECT id, nickname FROM users WHERE id = ?")
        .bind(uid)
        .fetch_one(pool)
        .await
        .ok()
}

async fn get_login_administrator(pool: &MySqlPool, session: &Session) -> Option<LoginUser> {
    let administrator_id = sess_administrator_id(session)?;

    sqlx::query_as::<_, LoginUser>("SELECT id, nickname FROM administrators WHERE id = ?")
        .bind(administrator_id)
        .fetch_one(pool)
        .await
        .ok()
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
