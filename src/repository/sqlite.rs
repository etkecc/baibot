use std::{path::Path, sync::Arc};
use anyhow::Ok;
use tokio::sync::Mutex;

use matrix_sdk::async_trait;

use super::BotRepository;

pub struct SqliteConn {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl SqliteConn {
    pub fn new(p: &Path) -> Self {
        Self {
            conn: Arc::new( 
            Mutex::new(rusqlite::Connection::open(p).unwrap()),
            )
        }
    }

    pub async fn execute(&self, query: String) -> anyhow::Result<()> {
        let lock = self.conn.lock().await;

        lock.execute(&query, [])?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct SqliteBotRepository {
    conn: Arc<SqliteConn>
}

impl SqliteBotRepository {
    pub fn new(conn: Arc<SqliteConn>) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl BotRepository for SqliteBotRepository {
    async fn store_response(&self, data: super::Response) -> anyhow::Result<()> {
        self.conn.conn.lock().await.execute(
            "INSERT INTO responses (content_length, bot_id, stored_at) VALUES (?1, ?2, ?3)",
            (data.length, data.bot_id, data.stored_at),
        )?;

        Ok(())
    }

    async fn store_answer(&self, data: super::Answer) -> anyhow::Result<()> {
        self.conn.conn.lock().await.execute(
            "INSERT INTO answers (content_length, bot_id, stored_at) VALUES (?1, ?2, ?3)",
            (data.length, data.bot_id, data.stored_at),
        )?;

        Ok(())
    }

    async fn get_response_count_from(&self, from: String, to: Option<String>) -> anyhow::Result<i64> {
        let to_date = match to {
            None => chrono::Utc::now().date_naive().to_string(),
            Some(d) => d 
        };

        let lock = self.conn.conn.lock().await;

        let stmt = lock
            .prepare("SELECT SUM(content_length) FROM responses WHERE stored_at BETWEEN ?1 AND ?2");
        let sum: Option<i64> = stmt.unwrap().query_row([from, to_date], |row| row.get(0))?;
        
        Ok(sum.unwrap_or_default())
    }

    async fn get_answer_count_from(&self, from: String, to: Option<String>) -> anyhow::Result<i64> {
        let to_date = match to {
            None => chrono::Utc::now().date_naive().to_string(),
            Some(d) => d 
        };
        
        let lock = self.conn.conn.lock().await;
        
        let stmt = lock
            .prepare("SELECT SUM(content_length) FROM answers WHERE stored_at BETWEEN ?1 AND ?2");
        let sum: Option<i64> = stmt.unwrap().query_row([from, to_date], |row| row.get(0))?;
        
        Ok(sum.unwrap_or_default())
    }
}
