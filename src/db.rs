use std::env;

use sqlx::{self, sqlite::SqlitePoolOptions, FromRow, Result, SqlitePool};

const DB_URL: &str = env!("DATABASE_URL");

#[derive(Clone)]
pub struct DBConnection {
    pool: SqlitePool,
}

pub type DB = DBConnection;

impl DBConnection {
    pub async fn new(pool_size: u32) -> Self {
        let pool = SqlitePoolOptions::new()
            .max_connections(pool_size)
            .connect(DB_URL)
            .await
            .unwrap();
        Self { pool }
    }

    pub async fn get_all_contacts(&self) -> Result<Vec<Contact>> {
        sqlx::query_as!(Contact, "select * from contacts")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn get_contact(&self, id: u32) -> Result<Contact> {
        sqlx::query_as!(
            Contact,
            "select * from contacts
             where id = ?",
            id
        )
        .fetch_one(&self.pool)
        .await
    }
}

// DB is the database driver
// `'r` is the lifetime of the `Row` being decoded
#[derive(Clone, FromRow)]
pub struct Contact {
    pub id: i64,
    pub name: String,
    pub email: String,
}
