mod users;
use std::collections::HashMap;

use actix_web::{get, post, web, App, HttpServer, Responder};
use elo::{Elo, HashMapIter, Player};
use sqlx::{postgres::PgPoolOptions, PgPool};
use structopt::StructOpt;
use tokio::sync::RwLock;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "elo_leaderboard_system_backend",
    about = "Backend for the Elo Leaderboard System"
)]
struct Opt {
    #[structopt(short = "l", long = "listen_url", default_value = "0.0.0.0:8080")]
    listen_url: String,

    #[structopt(
        short = "d",
        long = "db_url",
        default_value = "postgres://postgres:example@db:5432"
    )]
    db_url: String,
}

#[get("/")]
async fn index() -> impl Responder {
    println!("Hello world!");
    "Hello world!"
}

#[get("/players")]
async fn get_players(
    elo: web::Data<
        RwLock<
            Elo<
                'static,
                HashMapIter<'static, std::string::String, Player>,
                HashMap<std::string::String, Player>,
            >,
        >,
    >,
) -> impl Responder {
    let elo = elo.read().await;
    let mut response = String::new();
    for player in elo.into_iter() {
        println!("{}: {}", player.name(), player.rating());
        response.push_str(&format!("{}: {}\n", player.name(), player.rating()));
    }
    response
}

#[post("/player/{name}")]
async fn add_player(
    elo: web::Data<
        RwLock<
            Elo<
                'static,
                HashMapIter<'static, std::string::String, Player>,
                HashMap<std::string::String, Player>,
            >,
        >,
    >,
    name: web::Path<String>,
    pool: web::Data<PgPool>,
) -> impl Responder {
    let mut elo = elo.write().await;
    let new_player = name.into_inner().to_string();
    elo.add_player(new_player.clone());

    sqlx::query("INSERT INTO users (name, rating, number_of_games) VALUES ($1, $2, $3)")
        .bind(&new_player)
        .bind(0)
        .bind(0)
        .execute(&**pool)
        .await
        .unwrap();

    format!("Added player {}", new_player)
}

async fn set_up_db(
    pool: PgPool,
) -> Result<
    Elo<
        'static,
        HashMapIter<'static, std::string::String, Player>,
        HashMap<std::string::String, Player>,
    >,
    sqlx::Error,
> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            name TEXT NOT NULL,
            rating INTEGER NOT NULL,
            number_of_games INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    println!("Created table users");

    let users = sqlx::query("SELECT * FROM users").fetch_all(&pool).await?;

    let mut players = HashMap::new();
    while let Some(user) = users.iter().next() {
        let user = users::User::from_row(user).unwrap();
        players.insert(
            user.name.clone(),
            Player::new(user.name, user.rating, user.number_of_games),
        );
    }
    println!("Created players");

    let elo = Elo::new(players);

    Ok(elo)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();

    let pool = PgPoolOptions::new().connect(&opt.db_url).await.unwrap();

    let elo = set_up_db(pool.to_owned()).await.unwrap();
    let web_pool = web::Data::new(pool);
    let web_elo = web::Data::new(RwLock::new(elo));

    println!("Starting server");
    HttpServer::new(move || {
        App::new()
            .app_data(web_pool.clone())
            .app_data(web_elo.clone())
            .service(get_players)
            .service(index)
            .service(add_player)
    })
    .bind(opt.listen_url)?
    .run()
    .await?;
    Ok(())
}
