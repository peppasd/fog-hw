use sqlx::{migrate, migrate::MigrateDatabase, FromRow, Pool, Sqlite, SqlitePool};
use std::{
    env,
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::protocol;

#[derive(FromRow, Debug)]
pub struct Connection {
    pub id: i64,
    pub uid: String,
    pub last_seen: i64,
}

#[derive(FromRow, Debug)]
pub struct SentMessage {
    pub id: i64,
    pub uid: String,
    pub data: f64,
    pub created_at: i64,
}

#[derive(FromRow, Debug)]
pub struct QueuedMessage {
    pub id: i64,
    pub message: String,
    pub created_at: i64,
}

pub async fn initialize_db() -> Pool<Sqlite> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");

    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        Sqlite::create_database(&db_url)
            .await
            .expect("Could not create sqlite db");
    }

    let pool = SqlitePool::connect(&db_url)
        .await
        .expect("Could not connect to sqlite db");

    migrate!().run(&pool).await.expect("Could not migrate db");

    pool
}

pub async fn add_connection(
    pool: &Pool<Sqlite>,
    uid: &str,
) -> Result<Connection, Box<dyn Error + Send + Sync>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let id = sqlx::query!(
        r#"
INSERT INTO connections ( uid, last_seen )
VALUES ( ?1, ?2 )
        "#,
        uid,
        now
    )
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(Connection {
        id: id,
        uid: uid.to_string(),
        last_seen: now,
    })
}

pub async fn get_connection(
    pool: &Pool<Sqlite>,
    uid: &str,
) -> Result<Connection, Box<dyn Error + Send + Sync>> {
    let conn = sqlx::query_as!(
        Connection,
        r#" SELECT * FROM connections WHERE uid = ?1 "#,
        uid
    )
    .fetch_one(pool)
    .await?;

    Ok(conn)
}

pub async fn update_connection(
    pool: &Pool<Sqlite>,
    uid: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    sqlx::query!(
        r#" UPDATE connections SET last_seen = ?1 WHERE id = ?2 "#,
        now,
        uid
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_connection(
    pool: &Pool<Sqlite>,
    uid: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    sqlx::query!(r#" DELETE FROM connections WHERE uid = ?1 "#, uid)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn add_sent_message(
    pool: &Pool<Sqlite>,
    msg: &protocol::SensorMsg,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    sqlx::query!(
        r#" INSERT INTO sent_messages ( uid, data, created_at ) VALUES ( ?1, ?2, ?3 ) "#,
        msg.uid,
        msg.data,
        msg.timestamp
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_last_sent_messages(
    pool: &Pool<Sqlite>,
    limit: i64,
) -> Result<Vec<SentMessage>, Box<dyn Error + Send + Sync>> {
    let messages = sqlx::query_as!(
        SentMessage,
        r#" SELECT * FROM sent_messages ORDER BY created_at DESC LIMIT ?1 "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(messages)
}

pub async fn add_queued_message(
    pool: &Pool<Sqlite>,
    msg: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    sqlx::query!(
        r#" INSERT INTO queued_messages ( message, created_at ) VALUES ( ?1, ?2 ) "#,
        msg,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_new_queued_messages(
    pool: &Pool<Sqlite>,
    last_seen: &i64,
) -> Result<Vec<QueuedMessage>, Box<dyn Error + Send + Sync>> {
    let messages = sqlx::query_as!(
        QueuedMessage,
        r#" SELECT * FROM queued_messages WHERE created_at > ?1 "#,
        last_seen
    )
    .fetch_all(pool)
    .await?;

    Ok(messages)
}
