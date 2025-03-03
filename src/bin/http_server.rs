use std::fs;
use std::path::Path;
use std::sync::Arc;

use actix_web::http::header::ContentType;
use actix_web::{get, middleware, web, App, HttpResponse, HttpServer, Responder};
use baibot::repository::sqlite::SqliteBotRepository;
use baibot::repository::BotRepository;
use chrono::NaiveDate;
use serde::Deserialize;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "debug");

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
   
    let db_path:   &Path = std::path::Path::new("./etc/database/db.sqlite");
    let sc: baibot::repository::sqlite::SqliteConn = baibot::repository::sqlite::SqliteConn::new(db_path);
    let sc_arc = Arc::new(sc);
    let repo: SqliteBotRepository = baibot::repository::sqlite::SqliteBotRepository::new(sc_arc.clone());

    let migrations_path= std::path::Path::new("./etc/database/tables");
    let files = fs::read_dir(migrations_path)?;
    
    
    println!("running migrations");
    
    for f in files {
        let content = fs::read_to_string(f.unwrap().path())?;

        let _ = sc_arc.clone().execute(content).await.unwrap();
    }


    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(repo.clone()))
            .wrap(middleware::Logger::default())
            .service(report_answer)
            .service(report_response)
            
            
    })
    .workers(1  )
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
    .map_err(|e| anyhow::Error::new(e))
}

#[derive(Deserialize)]
struct Param {
    from: String,
    to: Option<String>,
}

#[get("/report/answer")]
async fn report_answer(repo:web::Data<SqliteBotRepository>, q: web::Query<Param>) -> impl Responder {
    let mut is_data_valid = NaiveDate::parse_from_str(&q.from.clone(), "%Y-%m-%d").is_ok();

    if q.to.is_some() {
        let to = &q.to.clone().unwrap();
        is_data_valid = NaiveDate::parse_from_str(to, "%Y-%m-%d").is_ok();
    }

    if !is_data_valid {
        let err_msg = "{\"message\": \"Invalid date format\"}";
        return HttpResponse::UnprocessableEntity()
            .content_type(ContentType::json())
            .body(err_msg);
    }

    let res = repo.get_answer_count_from(q.from.clone(), q.to.clone()).await;
    let decoded = serde_json::to_string(&res.unwrap()).unwrap();

    HttpResponse::UnprocessableEntity()
        .content_type(ContentType::json())
        .body(decoded)
}

#[get("/report/response")]
async fn report_response(repo:web::Data<SqliteBotRepository>, q: web::Query<Param>) -> impl Responder {
    let mut is_data_valid = NaiveDate::parse_from_str(&q.from.clone(), "%Y-%m-%d").is_ok();

    if q.to.is_some() {
        let to = &q.to.clone().unwrap();
        is_data_valid = NaiveDate::parse_from_str(to, "%Y-%m-%d").is_ok();
    }

    if !is_data_valid {
        let err_msg = "{\"message\": \"Invalid date format\"}";
        return HttpResponse::UnprocessableEntity()
            .content_type(ContentType::json())
            .body(err_msg);
    }

    let res = repo.get_response_count_from(q.from.clone(), q.to.clone()).await;
    let decoded = serde_json::to_string(&res.unwrap()).unwrap();

    HttpResponse::UnprocessableEntity()
        .content_type(ContentType::json())
        .body(decoded)
}
