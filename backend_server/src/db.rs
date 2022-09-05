use async_trait::async_trait;
use elo::{AsyncEloStorage, Player};
use sqlx::{any::AnyPoolOptions, AnyPool, Row};

pub struct Database {
    pool: AnyPool,
}

/// Sets up a new pool and creates the appropriate tables if they don't exist.
pub async fn fresh_database(db_url: &str) -> Result<AnyPool, sqlx::Error> {
    // set up pool
    let pool = AnyPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;

    // create the table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS players (
            name TEXT NOT NULL,
            rating INTEGER NOT NULL,
            number_of_games INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

impl Database {
    /// create a new database from a pool of connections
    pub async fn new(pool: AnyPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AsyncEloStorage for &Database {
    /// Add a player to the Database
    /// This should be called from another place, where users are added, so database should have a
    /// way of adding users, that in addition adds a player afterwards?
    async fn add_player(&self, player: Player) {
        sqlx::query("INSERT INTO players (name, rating, number_of_games) VALUES ($1, $2, $3)")
            .bind(&player.name())
            .bind(player.rating() as i32)
            .bind(player.number_of_games() as i32)
            .execute(&self.pool)
            .await
            .unwrap();
    }

    /// Update the player
    async fn update_player(&self, player: &Player) {
        sqlx::query("UPDATE players SET rating = $1, number_of_games = $2 WHERE name = $3")
            .bind(player.rating() as i32)
            .bind(player.number_of_games() as i32)
            .bind(&player.name())
            .execute(&self.pool)
            .await
            .unwrap();
    }

    /// Get a player from the database
    async fn get(&self, name: &str) -> Option<Player> {
        let player: Player =
            sqlx::query("SELECT name, rating, number_of_games FROM players WHERE name = $1")
                .bind(name)
                .fetch_one(&self.pool)
                .await
                .map(|row| {
                    let name: String = row.get("name");
                    let rating: i32 = row.get("rating");
                    let number_of_games: i32 = row.get("number_of_games");
                    Player::new(name, rating as usize, number_of_games as usize)
                })
                .unwrap();

        Some(player)
    }
}

impl Database {
    /// get a list of all players in the database
    pub async fn get_players(&self) -> Vec<Player> {
        sqlx::query("SELECT name, rating, number_of_games FROM players")
            .fetch_all(&self.pool)
            .await
            .unwrap()
            .into_iter()
            .map(|row| {
                let name: String = row.get("name");
                let rating: i32 = row.get("rating");
                let number_of_games: i32 = row.get("number_of_games");
                Player::new(name, rating as usize, number_of_games as usize)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use elo::AsyncElo;

    #[tokio::test]
    async fn abstraction() {
        // set up an in-memory database
        let db = Database::new(fresh_database("sqlite::memory:").await.unwrap()).await;
        let elo = AsyncElo::new(&db);

        // add users to this database
        elo.add_player("a").await;
        elo.add_player("b").await;

        // play a game
        elo.add_game("a", "b", false).await.unwrap();

        // check that the ratings updated correctly
        assert_eq!(elo.get_player("a").await.unwrap().rating(), 1016);
        assert_eq!(elo.get_player("b").await.unwrap().rating(), 984);

        // check that the number of games updated correctly
        assert_eq!(elo.get_player("a").await.unwrap().number_of_games(), 1);
        assert_eq!(elo.get_player("b").await.unwrap().number_of_games(), 1);
    }
}
