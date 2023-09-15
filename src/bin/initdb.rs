use sqlx::migrate::MigrateDatabase;
use sqlx::{Sqlite, SqlitePool};

const DB_URL: &str = "sqlite://sqlite.db";

#[tokio::main]
async fn main() {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }
    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let _result = sqlx::query(
        "CREATE TABLE IF NOT EXISTS contacts (
            id INTEGER PRIMARY KEY NOT NULL,
            name VARCHAR(250) NOT NULL,
            email VARCHAR(250) UNIQUE NOT NULL
        );",
    )
    .execute(&db)
    .await
    .unwrap();
    let _result = sqlx::query(
        "INSERT INTO contacts
        (id, name, email)
        VALUES
      (0, 'John', 'g0@gmail.com'), 
      (1, 'Jane', 'g1@gmail.com'), 
      (2, 'Billy', 'g2@gmail.com'),
      (3, 'Miranda', 'g3@gmail.com');",
    )
    .execute(&db)
    .await
    .unwrap();
}
