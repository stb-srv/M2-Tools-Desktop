use sqlx::MySqlPool;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    pub mysql_pool: Mutex<Option<MySqlPool>>,
}
