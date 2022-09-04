mod db;
mod users;

use db::{fresh_database, Database};

use actix_web::{get, post, web, App, Either, HttpServer, Responder};
use structopt::StructOpt;

use elo::AsyncElo;

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
async fn get_players(db: web::Data<Database>) -> impl Responder {
    web::Json(
        db.get_players()
            .await
            .into_iter()
            .map(|p| p.into())
            .collect::<Vec<User>>(),
    )
}

#[post("/player/{name}")]
async fn add_player(
    db: web::Data<Database>,
    name: web::Path<String>,
) -> Either<impl Responder, impl Responder> {
    let db = &*db.into_inner();
    let elo = AsyncElo::new(db);
    let new_player = name.into_inner();

    elo.add_player(&new_player).await;

    match elo.get_player(&new_player).await.unwrap().try_into() {
        Ok(user) => Either::Left(web::Json::<User>(user)),
        Err(_) => Either::Right(web::Json("Failed to convert player to user")),
    }
}

#[post("/draw/{player1}/{player2}")]
async fn add_draw(
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Either<impl Responder, impl Responder> {
    let db = &*db.into_inner();
    let elo = AsyncElo::new(db);

    let (player1, player2) = path.into_inner();

    match elo.add_game(&player1, &player2, false).await {
        Ok(_) => {
            let p1: User = elo.get_player(&player1).await.unwrap().try_into().unwrap();
            let p2: User = elo.get_player(&player2).await.unwrap().try_into().unwrap();
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
    db: web::Data<Database>,
    path: web::Path<(String, String)>,
) -> Either<impl Responder, impl Responder> {
    let db = &*db.into_inner();
    let elo = AsyncElo::new(db);

    let (winner, loser) = path.into_inner();

    match elo.add_game(&winner, &loser, false).await {
        Ok(_) => {
            let p1: User = elo.get_player(&winner).await.unwrap().try_into().unwrap();
            let p2: User = elo.get_player(&loser).await.unwrap().try_into().unwrap();
            Either::Left(web::Json(vec![p1, p2]))
        }
        Err(error) => Either::Right(format!(
            "Failed to add game {} beat {}, Error: {}",
            winner, loser, error
        )),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let db = Database::new(fresh_database(&opt.db_url).await?).await;

    let web_db = web::Data::new(db);

    HttpServer::new(move || {
        App::new()
            .app_data(web_db.clone())
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
