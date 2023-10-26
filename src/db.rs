use std::env;

// use askama::Result;
// pub use sqlx::Result;
use sqlx::{
    self,
    mysql::{MySqlPool, MySqlPoolOptions, MySqlQueryResult},
    // sqlite::{SqlitePoolOptions, SqliteQueryResult},
    FromRow,
};

// struct DBError(sqlx::Error)

// const DB_URL: &str = "mysql://devenv:@localhost/contacts";
const DB_URL: &str = env!("DATABASE_URL");

#[derive(Clone)]
pub struct DBConnection {
    pool: MySqlPool,
}

pub type DB = DBConnection;

impl DB {
    pub fn conn(&self) -> &MySqlPool {
        &self.pool
    }

    pub async fn new(pool_size: u32) -> Self {
        let pool = MySqlPoolOptions::new()
            .max_connections(pool_size)
            .connect(DB_URL)
            .await
            .unwrap();
        Self { pool }
    }

    pub async fn search_by_name(&self, term: &str) -> sqlx::Result<Vec<Contact>> {
        let reg = format!(r#"%{term}%"#);
        sqlx::query_as!(
            Contact,
            r#"
                select * from contacts
                where name like ?"#,
            reg
        )
        .fetch_all(&self.pool)
        .await
    }
    pub async fn get_all_contacts(&self) -> sqlx::Result<Vec<Contact>> {
        sqlx::query_as!(Contact, "select * from contacts")
            .fetch_all(&self.pool)
            .await
    }

    pub async fn edit_contact(
        &self,
        id: u32,
        name: &str,
        email: &str,
    ) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query!(
            "update contacts
            set name = ?, email = ?
            where id = ?",
            name,
            email,
            id
        )
        .execute(&self.pool)
        .await
    }

    pub async fn find_user_email(&self, email: &str) -> sqlx::Result<Option<i32>> {
        let res = sqlx::query!(
            "select id from users
            where email = ?",
            email
        )
        .fetch_optional(&self.pool)
        .await;
        match res {
            Ok(v) => match v {
                Some(v) => Ok(Some(v.id)),
                None => Ok(None),
            },
            Err(e) => Err(e),
        }
    }
    pub async fn find_email(&self, email: &str) -> sqlx::Result<Option<i32>> {
        let res = sqlx::query!(
            "select id from contacts
            where email = ?",
            email
        )
        .fetch_optional(&self.pool)
        .await;
        match res {
            Ok(v) => match v {
                Some(v) => Ok(Some(v.id)),
                None => Ok(None),
            },
            Err(e) => Err(e),
        }
    }

    pub async fn add_contact(&self, name: String, email: String) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query!(
            "insert into contacts (name, email)
            values (?, ?)",
            name,
            email
        )
        .execute(&self.pool)
        .await
    }
    pub async fn remove_contact(&self, id: u32) -> sqlx::Result<MySqlQueryResult> {
        sqlx::query!("delete from contacts where id = ?", id)
            .execute(&self.pool)
            .await
    }

    pub async fn get_contact(&self, id: u32) -> sqlx::Result<Contact> {
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
#[derive(Clone, FromRow, Debug)]
pub struct Contact {
    pub id: i32,
    pub name: String,
    pub email: String,
}
