use sqlx::{migrate, migrate::MigrateDatabase, FromRow, Pool, Sqlite, SqlitePool};
use std::{
    env,
    error::Error,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::info;

use crate::protocols;

#[derive(FromRow, Debug)]
pub struct Metrics {
    pub connections: Option<i32>,
    pub received_messages: Option<i32>,
    pub queued_messages: Option<i32>,
    pub delivered_messages: Option<i32>,
}

#[derive(FromRow, Debug)]
pub struct Connection {
    pub id: i64,
    pub uid: String,
    pub last_seen: i64,
}

#[derive(FromRow, Debug)]
pub struct ReceivedMessage {
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
            .expect("Could not create the sqlite db");
        info!("Created new sqlite db")
    } else {
        info!("Using an existing sqlite db")
    }

    let pool = SqlitePool::connect(&db_url)
        .await
        .expect("Could not connect to the sqlite db");

    migrate!()
        .run(&pool)
        .await
        .expect("Could not migrate the db");

    pool
}

pub async fn get_metrics(pool: &Pool<Sqlite>) -> Result<Metrics, Box<dyn Error + Send + Sync>> {
    let metrics = sqlx::query_as::<_, Metrics>(
        r#" SELECT 
            (SELECT COUNT(*) FROM connections) as connections,
            (SELECT COUNT(*) FROM received_messages) as received_messages,
            (SELECT COUNT(*) FROM queued_messages) as queued_messages,
            (SELECT COUNT(*) FROM delivered_messages) as delivered_messages
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(metrics)
}

pub async fn add_connection(
    pool: &Pool<Sqlite>,
    uid: &str,
) -> Result<Connection, Box<dyn Error + Send + Sync>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let id = sqlx::query("INSERT INTO connections ( uid, last_seen ) VALUES ( ?1, ?2 )")
        .bind(uid)
        .bind(now)
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
    let conn = sqlx::query_as::<_, Connection>("SELECT * FROM connections WHERE uid = ?1")
        .bind(uid)
        .fetch_one(pool)
        .await?;

    Ok(conn)
}

pub async fn update_connection(
    pool: &Pool<Sqlite>,
    uid: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    sqlx::query("UPDATE connections SET last_seen = ?1 WHERE id = ?2")
        .bind(now)
        .bind(uid)
        .execute(pool)
        .await?;

    Ok(())
}

// pub async fn delete_connection(
//     pool: &Pool<Sqlite>,
//     uid: &str,
// ) -> Result<(), Box<dyn Error + Send + Sync>> {
//     sqlx::query("DELETE FROM connections WHERE uid = ?1")
//         .bind(uid)
//         .execute(pool)
//         .await?;

//     Ok(())
// }

pub async fn add_received_message(
    pool: &Pool<Sqlite>,
    msg: &protocols::SensorMsg,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    sqlx::query("INSERT INTO received_messages ( uid, data, created_at ) VALUES ( ?1, ?2, ?3 )")
        .bind(&msg.uid)
        .bind(&msg.data)
        .bind(&msg.timestamp)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_last_received_messages(
    pool: &Pool<Sqlite>,
    limit: i64,
) -> Result<Vec<ReceivedMessage>, Box<dyn Error + Send + Sync>> {
    let messages = sqlx::query_as::<_, ReceivedMessage>(
        "SELECT * FROM received_messages ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(messages)
}

pub async fn add_queued_message(
    pool: &Pool<Sqlite>,
    msg: String,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    sqlx::query("INSERT INTO queued_messages ( message, created_at ) VALUES ( ?1, ?2 )")
        .bind(msg)
        .bind(now)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_new_queued_messages(
    pool: &Pool<Sqlite>,
) -> Result<Vec<QueuedMessage>, Box<dyn Error + Send + Sync>> {
    let messages = sqlx::query_as::<_, QueuedMessage>(
        "SELECT * FROM queued_messages WHERE id NOT IN ( SELECT queued_message_id FROM delivered_messages ) ORDER BY created_at ASC")
        .bind(0)
        .fetch_all(pool)
        .await?;

    Ok(messages)
}

pub async fn add_delivered_message(
    pool: &Pool<Sqlite>,
    uid: &str,
    queued_message_id: &i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    sqlx::query("INSERT INTO delivered_messages ( uid, queued_message_id ) VALUES ( ?1, ?2 )")
        .bind(uid)
        .bind(queued_message_id)
        .execute(pool)
        .await?;

    Ok(())
}
