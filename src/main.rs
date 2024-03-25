use actix_web::{
    get,
    http::StatusCode,
    post,
    web::{self, Redirect},
    App, HttpResponse, HttpResponseBuilder, HttpServer, Responder,
};
use serde::Deserialize;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect("./db.sqlite")
        .await
        .unwrap();

    sqlx::query(
        r#"
            create table if not exists shortcuts (
                id text not null,
                url text not null
            )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(root)
            .service(create_shortcut)
            .service(redirect)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[get("/")]
async fn root() -> &'static str {
    "Hello, World!"
}

#[post("/")]
async fn create_shortcut(
    sqlite: web::Data<sqlx::sqlite::SqlitePool>,
    web::Form(shortcut): web::Form<Shortcut>,
) -> impl Responder {
    let mut sqlite = sqlite.acquire().await.unwrap();

    let results = sqlx::query("insert into shortcuts (id, url) values (?, ?)")
        .bind(shortcut.id)
        .bind(shortcut.url.to_string())
        .execute(&mut *sqlite)
        .await;

    match results {
        Err(error) => HttpResponseBuilder::new(StatusCode::BAD_REQUEST).body(error.to_string()),
        Ok(_) => HttpResponse::new(StatusCode::CREATED),
    }
}

#[derive(sqlx::FromRow)]
pub struct GetShortcut {
    pub url: String,
}

#[get("/{id}")]
async fn redirect(
    sqlite: web::Data<sqlx::sqlite::SqlitePool>,
    id: web::Path<String>,
) -> impl Responder {
    let mut sqlite = sqlite.acquire().await.unwrap();

    let result: Result<Option<GetShortcut>, _> =
        sqlx::query_as("select url from shortcuts where id = ?")
            .bind(id.into_inner())
            .fetch_optional(&mut *sqlite)
            .await;

    match result {
        Err(_) => HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR),
        Ok(data) => match data {
            None => HttpResponse::new(StatusCode::NOT_FOUND),
            Some(data) => HttpResponseBuilder::new(StatusCode::PERMANENT_REDIRECT)
                .insert_header(("Location", data.url))
                .finish(),
        },
    }
}

#[derive(Deserialize)]
pub struct Shortcut {
    pub id: String,
    pub url: url::Url,
}
