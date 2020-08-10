extern crate actix_web;
extern crate sqlx;
extern crate tera;

use std::collections::HashMap;
use std::env;
use std::process::Command;

use actix_session::{CookieSession, Session};
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};

use sqlx::mysql::{MySqlPool, MySqlQueryAs};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use tera::Tera;

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
    public_fg: bool,
    closed_fg: bool,
    price: i64,

    total: i32,
    remains: i32,
    sheets: HashMap<char, Sheets>,
}

impl Event {
    fn new(
        id: i64,
        title: String,
        public_fg: bool,
        closed_fg: bool,
        price: i64,
        total: i32,
        remains: i32,
        sheets: HashMap<char, Sheets>,
    ) -> Event {
        Event {
            id: id,
            title: title,
            public_fg: public_fg,
            closed_fg: closed_fg,
            price: price,

            total: total,
            remains: remains,
            sheets: sheets,
        }
    }

    fn sanitize_event(self) -> Event {
        Event::new(
            self.id,
            self.title,
            false,
            false,
            0,
            self.total,
            self.remains,
            self.sheets,
        )
    }
}

struct Sheets {
    total: i32,
    remains: i32,
    details: Vec<Sheet>,
    price: i64,
}

impl Sheets {
    fn new(total: i32, remains: i32, details: Vec<Sheet>, price: i64) -> Sheets {
        Sheets {
            total: total,
            remains: remains,
            details: details,
            price: price,
        }
    }
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

#[derive(sqlx::FromRow, Deserialize, Serialize)]
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

#[derive(sqlx::FromRow)]
struct GetEvent {
    id: i64,
    title: String,
    public_fg: bool,
    closed_fg: bool,
    price: i64,
}

async fn get_events(pool: &MySqlPool, all: bool) -> anyhow::Result<Vec<Event>> {
    let events: Vec<GetEvent> =
        sqlx::query_as::<_, GetEvent>("SELECT * FROM events ORDER BY id ASC")
            .fetch_all(pool)
            .await?
            .into_iter()
            .filter(|e| !all && e.public_fg)
            .collect();

    events.iter().map(move |e| async move {
        let event = get_event(pool, e.id, -1).await.unwrap();
        // TODO sheetのdetailをNoneにする?
        event
    });

    Ok(Vec::new())
}

async fn get_event(pool: &MySqlPool, event_id: i64, login_user_id: i64) -> anyhow::Result<Event> {
    let get_event = sqlx::query_as::<_, GetEvent>("SELECT * FROM events WHERE id = ?")
        .bind(&event_id)
        .fetch_one(pool)
        .await?;

    let mut sheets = HashMap::new();
    sheets.insert('S', Sheets::new(0, 0, Vec::new(), 0));
    sheets.insert('A', Sheets::new(0, 0, Vec::new(), 0));
    sheets.insert('B', Sheets::new(0, 0, Vec::new(), 0));
    sheets.insert('C', Sheets::new(0, 0, Vec::new(), 0));

    // TODO sheetの処理をもっと書く必要あり
    Ok(Event {
        id: get_event.id,
        title: get_event.title,
        public_fg: get_event.public_fg,
        closed_fg: get_event.closed_fg,
        price: get_event.price,

        total: 0,
        remains: 0,
        sheets: sheets,
    })
}

async fn fillin_user(tux: &mut tera::Context, pool: &MySqlPool, session: &Session) {
    if let Some(user) = get_login_user(pool, session).await {
        tux.insert("user", &user);
    }
}

async fn fillin_administrator(tux: &mut tera::Context, pool: &MySqlPool, session: &Session) {
    if let Some(admin) = get_login_administrator(pool, session).await {
        tux.insert("administrator", &admin);
    }
}

async fn validate_rank(rank: String, pool: &MySqlPool) -> bool {
    let count = sqlx::query_as::<_, (i32,)>("SELECT COUNT(*) FROM sheets WHERE rank = ?")
        .bind(&rank)
        .fetch_one(pool)
        .await
        .expect("can't get count of sheets")
        .0;

    count > 0
}

async fn get_dummy(req: HttpRequest) -> impl Responder {
    println!("{:?}", req);
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("Hello, actix_web.")
}

struct Context {
    db_pool: MySqlPool,
    templates: tera::Tera,
}

async fn get_initialize(data: web::Data<Context>) -> impl Responder {
    Command::new("../../db/init.sh")
        .spawn()
        .expect("can't initialize db data.");

    HttpResponse::NoContent().finish()
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    env_logger::init();

    let database_url =
        "mysql://isucon:isucon@tcp(192.168.33.10:3306)/torb?parseTime=true&charset=utf8mb4";
    let pool = MySqlPool::builder()
        .max_size(5)
        .build(&database_url)
        .await
        .unwrap();

    HttpServer::new(move || {
        let templates = Tera::new("views/*.html").unwrap();

        App::new()
            .data(Context {
                db_pool: pool.clone(),
                templates: templates,
            })
            .wrap(CookieSession::signed(&[0; 32]).secure(false))
            .wrap(middleware::Logger::default())
            .service(web::resource("/").route(web::get().to(get_dummy)))
            .service(web::resource("/initialize").route(web::get().to(get_initialize)))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
