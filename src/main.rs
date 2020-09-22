use actix_web::{get, App, web, HttpResponse, HttpServer, ResponseError, http::header, post};
use askama::Template;
use thiserror::Error;
use r2d2_sqlite::SqliteConnectionManager;
use r2d2::Pool;
use rusqlite::params;
use std::net::AddrParseError;
use serde::Deserialize;

#[derive(Deserialize)]
struct AddParam {
    text: String,
}

#[derive(Deserialize)]
struct DeleteParam {
    id: u32,
}

#[derive(Error, Debug)]
enum MyError {
    #[error("Failed to render HTML")]
    AskamaError(#[from] askama::Error),

    #[error("Failed to render HTML")]
    ConnectionPoolError(#[from] r2d2::Error),

    #[error("Failed to render HTML")]
    SQliteError(#[from] rusqlite::Error),
}

impl ResponseError for MyError {}

#[get("/")]
async fn index(db: web::Data<Pool<SqliteConnectionManager>>) -> Result<HttpResponse, MyError> {
    let conn = db.get()?;
    let mut st = conn.prepare("SELECT id, text from todo")?;
    let rows = st.query_map(params![], |row| {
        let id = row.get(0)?;
        let text = row.get(1)?;
        Ok(TodoEntry { id, text })
    })?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    let html = IndexTemplate { entries };
    let response_body = html.render()?;

    Ok(HttpResponse::Ok().content_type("text/html").body(response_body))
}

#[actix_rt::main]
async fn main() -> Result<(), actix_web::Error> {
    let manager = SqliteConnectionManager::file("todo.db");
    let pool = Pool::new(manager).expect("Failed to initialize the connection pool.");
    let conn = pool.get().expect("Failed to bget connction from pool");
    conn.execute("CREATE TABLE IF NOT EXISTS todo (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        text TEXT NOT NULL
    )",
                 params![],
    ).expect("Failed to create a table todo");

    HttpServer::new(move || App::new().service(index).service(add_todo).service(delete_todo).data(pool.clone()))
        .bind("0.0.0.0:8080")?
        .run()
        .await?;

    Ok(())
}

struct TodoEntry {
    id: i32,
    text: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    entries: Vec<TodoEntry>,
}

#[post("/add")]
async fn add_todo(
    params: web::Form<AddParam>,
    db: web::Data<r2d2::Pool<SqliteConnectionManager>>,
) -> Result<HttpResponse, MyError> {
    let conn = db.get()?;
    conn.execute("INSERT INTO todo (text) VALUES (?)", &[&params.text])?;
    Ok(HttpResponse::SeeOther().header(header::LOCATION, "/").finish())
}

#[post("/delete")]
async fn delete_todo(
    params: web::Form<DeleteParam>,
    db: web::Data<r2d2::Pool<SqliteConnectionManager>>,
) -> Result<HttpResponse, MyError> {
    let conn = db.get()?;
    conn.execute("DELETE FROM todo where id = ?", &[params.id])?;
    Ok(HttpResponse::SeeOther().header(header::LOCATION, "/").finish())
}
