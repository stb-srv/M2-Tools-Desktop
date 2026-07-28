use rusqlite::Connection;
use sqlx::MySqlPool;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;

pub struct AppState {
    pub mysql_pool: Mutex<Option<MySqlPool>>,
    pub settings_db: StdMutex<Connection>,
}
