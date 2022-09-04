use std::marker::PhantomData;

use sqlx::{postgres::PgPoolOptions, PgPool};
use elo::{Elo, EloStorage, Player};

pub struct Database {
    pool: PgPool
}

impl Database {
    pub async fn set_up(db_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(db_url)
            .await?;

        Ok(Self { pool })
    }
}

pub struct PlayerIter<'a> {
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Iterator for PlayerIter<'a> {
    type Item = &'a Player;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl<'a> EloStorage<'a, PlayerIter<'a>> for Database {
    fn add_player(&mut self, player: elo::Player) {
        todo!()
    }

    fn update_player(&mut self, player: &elo::Player) {
        todo!()
    }

    fn get(&self, name: &str) -> Option<&elo::Player> {
        todo!()
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut elo::Player> {
        todo!()
    }

    fn iter(&self) -> PlayerIter {
        todo!()
    }
}
