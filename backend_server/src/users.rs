use elo::Player;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub name: String,
    pub rating: usize,
    pub number_of_games: usize,
}

impl User {
    pub fn new(name: String) -> Self {
        Self {
            name,
            rating: 0,
            number_of_games: 0
        }
    }

    pub fn from_row(row: &sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            name: row.get::<String, &str>("name"),
            rating: row.get::<i32, &str>("rating").try_into().unwrap_or(0),
            number_of_games: row.get::<i32, &str>("number_of_games").try_into().unwrap_or(0),
        })
    }
}

impl From<Player> for User {
    fn from(player: Player) -> Self {
        Self {
            name: player.name().to_string(),
            rating: player.rating(),
            number_of_games: player.number_of_games(),
        }
    }
}
