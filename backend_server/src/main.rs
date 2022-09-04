mod users;
use std::collections::HashMap;

use actix_web::{get, post, web, App, Either, HttpServer, Responder};
use elo::{Elo, HashMapIter, Player};
use sqlx::{postgres::PgPoolOptions, PgPool};
use structopt::StructOpt;
use tokio::sync::RwLock;

use crate::users::User;

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

#[get("/health")]
async fn health() -> impl Responder {
    "OK"
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
    web::Json(
        elo.into_iter()
            .map(|player| player.clone().into())
            .collect::<Vec<User>>(),
    )
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
) -> Either<impl Responder, impl Responder> {
    let mut elo = elo.write().await;
    let new_player = name.into_inner().to_string();
    elo.add_player(new_player.clone());

    match elo[&new_player].clone().try_into() {
        Ok(user) => Either::Left(web::Json::<User>(user)),
        Err(_) => {
            return Either::Right(web::Json("Failed to convert player to user"));
        }
    }
}

#[post("/draw/{player1}/{player2}")]
async fn add_draw(
    elo: web::Data<
        RwLock<
            Elo<
                'static,
                HashMapIter<'static, std::string::String, Player>,
                HashMap<std::string::String, Player>,
            >,
        >,
    >,
    player1: web::Path<String>,
    player2: web::Path<String>,
) -> Either<impl Responder, impl Responder> {
    let mut elo = elo.write().await;

    let player1 = player1.into_inner().to_string();
    let player2 = player2.into_inner().to_string();

    match elo.add_game(&player1, &player2, false) {
        Ok(_) => {
            let p1: User = elo[&player1].clone().try_into().unwrap();
            let p2: User = elo[&player2].clone().try_into().unwrap();
            Either::Left(web::Json(vec![p1, p2]))
        }
        Err(error) => Either::Right(format!(
            "Failed to add draw between {} and {}, Error: {}",
            player1, player2, error
        )),
    }
}

#[post("/game/{winner}/{loser}")]
async fn add_game(
    elo: web::Data<
        RwLock<
            Elo<
                'static,
                HashMapIter<'static, std::string::String, Player>,
                HashMap<std::string::String, Player>,
            >,
        >,
    >,
    winner: web::Path<String>,
    loser: web::Path<String>,
) -> Either<impl Responder, impl Responder> {
    let mut elo = elo.write().await;

    let winner = winner.into_inner().to_string();
    let loser = loser.into_inner().to_string();

    //TODO: Fix return format
    match elo.add_game(&winner, &loser, false) {
        Ok(_) => {
            let p1: User = elo[&winner].clone().try_into().unwrap();
            let p2: User = elo[&loser].clone().try_into().unwrap();
            Either::Left(web::Json(vec![p1, p2]))
        }
        Err(error) => Either::Right(format!(
            "Failed to add game {} beat {}, Error: {}",
            winner, loser, error
        )),
    }
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


    // let users = sqlx::query("SELECT * FROM users").fetch_all(&pool).await?;

    let players = HashMap::new();

    // while let Some(user) = users.iter().next() {
    //     let user = users::User::from_row(user).unwrap();
    //     players.insert(
    //         user.name.clone(),
    //         Player::new(user.name, user.rating, user.number_of_games),
    //     );
    // }
    // println!("Created players");

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

    HttpServer::new(move || {
        App::new()
            .app_data(web_pool.clone())
            .app_data(web_elo.clone())
            .service(health)
            .service(get_players)
            .service(add_player)
            .service(add_draw)
            .service(add_game)
    })
    .bind(opt.listen_url)?
    .run()
    .await?;
    Ok(())
}
