use matrix_sdk::async_trait;

pub mod sqlite;

pub struct Response {
    pub id: i32,
    pub length: i64,
    pub bot_id: String,
    pub stored_at: String,
}

pub struct Answer {
    pub id: i32,
    pub bot_id: String,
    pub length: i64,
    pub stored_at: String,
}

#[async_trait]
pub trait BotRepository: Send + Sync {
    async fn store_response(&self, data: Response) -> anyhow::Result<()>;

    async fn store_answer(&self, data: Answer) -> anyhow::Result<()>;

    async fn get_response_count_from(&self, from: String, to: Option<String>) -> anyhow::Result<i64>;

    async fn get_answer_count_from(&self, from: String, to: Option<String>) -> anyhow::Result<i64>;
}
