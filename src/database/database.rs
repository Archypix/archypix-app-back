use diesel::mysql::MysqlConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use std::env;

pub type DBPool = Pool<ConnectionManager<MysqlConnection>>;
pub type DBConn = PooledConnection<ConnectionManager<MysqlConnection>>;

pub fn get_connection() -> MysqlConnection {
    let url = get_database_url();
    MysqlConnection::establish(&url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", url))
}

pub fn get_connection_pool() -> Pool<ConnectionManager<MysqlConnection>> {
    let url = get_database_url();
    let manager = ConnectionManager::<MysqlConnection>::new(url.clone());

    Pool::builder()
        .test_on_check_out(true)
        .build(manager)
        .expect(&*format!("Could not build connection pool to database url: {}", url))
}

pub fn get_database_url() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}
