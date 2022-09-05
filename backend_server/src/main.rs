mod db;
mod users;

use db::{fresh_database, Database};

use actix_web::{get, post, web, App, Either, Either::*, HttpServer, Responder};
use serde_json::json;
use structopt::StructOpt;

use serde::Deserialize;

use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::{SwaggerUi, Url};

use elo::AsyncElo;

use crate::users::User;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "elo_leaderboard_system_backend",
    about = "Backend for the Elo Leaderboard System"
)]
struct Opt {
    #[structopt(long, short, default_value = "0.0.0.0:8080")]
    listen_url: String,

    #[structopt(long, short, default_value = "postgres://postgres:example@db:5432")]
    db_url: String,
}

#[utoipa::path(
    context_path = "/api",
    responses(
        (status=200, description="OK", body=String,
         example = json!({"status": "OK"}))
    )
)]
#[get("/health")]
async fn health() -> impl Responder {
    web::Json(json!({"status": "OK"}))
}

#[utoipa::path(
    context_path = "/api",
    responses(
        (status=200, description="List of Players", body=String),
    )
)]
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

#[utoipa::path(
    context_path = "/api",
    responses(
        (status=200, description="The newly created player", body=User),
        (status=400, description="Player already exists", body=User),
    ),
    params(
        ("name", description = "Name of the player to be added")
    )
)]
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
        Ok(user) => Left(web::Json::<User>(user)),
        Err(_) => Right(web::Json("Failed to convert player to user")),
    }
}

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({"is_draw": false, "player1": "jens", "player2": "arne"}))]
struct GameParams {
    is_draw: bool,
    player1: String,
    player2: String,
}

#[utoipa::path(
    context_path = "/api",
    responses(
        (status=200, description="The game was added. Information on the players involved returned.", body=Vec<User>),
        (status=400, description="Player(s) does not exist", body=String),
    ),
    request_body(
        description = "The game to be added",
        content = GameParams,
        content_type = "application/json"
    )
)]
#[post("/game")]
async fn add_game(
    db: web::Data<Database>,
    params: web::Json<GameParams>,
) -> Either<impl Responder, impl Responder> {
    let db = &*db.into_inner();
    let elo = AsyncElo::new(db);
    let game = params.into_inner();

    match elo
        .add_game(&game.player1, &game.player2, game.is_draw)
        .await
    {
        Ok(_) => {
            let p1: User = elo
                .get_player(&game.player1)
                .await
                .unwrap()
                .try_into()
                .unwrap();
            let p2: User = elo
                .get_player(&game.player2)
                .await
                .unwrap()
                .try_into()
                .unwrap();
            Left(web::Json(vec![p1, p2]))
        }
        Err(error) => Right(format!(
            "Failed to add game {} beat {}, Error: {}",
            game.player1, game.player2, error
        )),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let db = Database::new(fresh_database(&opt.db_url).await?).await;

    let web_db = web::Data::new(db);

    #[derive(OpenApi)]
    #[openapi(paths(health, get_players, add_player, add_game))]
    struct ApiDoc;

    HttpServer::new(move || {
        App::new()
            .app_data(web_db.clone())
            .service(
                web::scope("/api")
                    .service(health)
                    .service(get_players)
                    .service(add_player)
                    .service(add_game),
            )
            .service(SwaggerUi::new("/swagger-ui/{_:.*}").urls(vec![(
                Url::new("api", "/api-doc/openapi.json"),
                ApiDoc::openapi(),
            )]))
    })
    .bind(opt.listen_url)?
    .run()
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::*;

    use actix_web::{
        body::MessageBody,
        http::{
            self,
            header::{ContentType, Header},
        },
        test,
    };

    #[actix_web::test]
    async fn test_health() {
        let app = test::init_service(App::new().service(health)).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.headers().get(ContentType::name()).unwrap(),
            "application/json"
        );
        // convert the body to a JSON object
        let response: serde_json::Value = serde_json::from_str(
            std::str::from_utf8(&resp.into_body().try_into_bytes().unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(response, json!({"status": "OK"}));
    }
}
