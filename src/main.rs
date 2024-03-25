use actix_web::{
    delete, get, http::StatusCode, post, web, App, HttpResponse, HttpResponseBuilder, HttpServer,
    Responder,
};
use askama::Template;
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
                id text primary key unique not null,
                url text not null
            )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    println!("Listening in http://0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(root)
            .service(create_shortcut)
            .service(delete_shortcut)
            .service(redirect)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[derive(askama::Template)]
#[template(path = "index.html")]
pub struct Homepage {
    pub shortcuts: Vec<GetShortcut>,
}

impl Homepage {
    pub async fn create_and_render(
        mut sqlite: sqlx::pool::PoolConnection<sqlx::sqlite::Sqlite>,
    ) -> HttpResponse {
        let shortcuts: Vec<GetShortcut> = sqlx::query_as("select id, url from shortcuts")
            .fetch_all(&mut *sqlite)
            .await
            .unwrap_or_default();

        let template = Homepage { shortcuts };

        HttpResponse::Ok()
            .content_type("text/html")
            .body(template.render().unwrap())
    }
}

#[get("/")]
async fn root(sqlite: web::Data<sqlx::sqlite::SqlitePool>) -> impl Responder {
    let sqlite = sqlite.acquire().await.unwrap();

    Homepage::create_and_render(sqlite).await
}

#[derive(Deserialize)]
pub struct Shortcut {
    pub id: String,
    pub url: url::Url,
}

#[delete("/_delete/{id}")]
async fn delete_shortcut(
    sqlite: web::Data<sqlx::sqlite::SqlitePool>,
    id: web::Path<String>,
) -> impl Responder {
    let mut sqlite = sqlite.acquire().await.unwrap();

    let results = sqlx::query("delete from shortcuts where id = ?")
        .bind(id.to_string())
        .execute(&mut *sqlite)
        .await;

    match results {
        Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
        Ok(_) => HttpResponse::NoContent().finish(),
    }
}

#[post("/")]
async fn create_shortcut(
    sqlite: web::Data<sqlx::sqlite::SqlitePool>,
    web::Form(shortcut): web::Form<Shortcut>,
) -> impl Responder {
    if shortcut.id.trim().is_empty() {
        return HttpResponse::BadRequest().body("Id cannot be empty");
    }

    let mut sqlite = sqlite.acquire().await.unwrap();

    let results = sqlx::query("insert into shortcuts (id, url) values (?, ?)")
        .bind(shortcut.id)
        .bind(shortcut.url.to_string())
        .execute(&mut *sqlite)
        .await;

    match results {
        Err(error) => HttpResponse::BadRequest().body(error.to_string()),
        Ok(_) => Homepage::create_and_render(sqlite).await,
    }
}

#[derive(sqlx::FromRow)]
pub struct GetShortcut {
    pub id: String,
    pub url: String,
}

#[get("/{id}")]
async fn redirect(
    sqlite: web::Data<sqlx::sqlite::SqlitePool>,
    id: web::Path<String>,
) -> impl Responder {
    let mut sqlite = sqlite.acquire().await.unwrap();

    let result: Result<Option<GetShortcut>, _> =
        sqlx::query_as("select id, url from shortcuts where id = ?")
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
