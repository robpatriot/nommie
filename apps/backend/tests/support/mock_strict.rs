use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use sea_orm::DatabaseConnection;

static REGISTRY: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashSet<usize>> {
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

fn key(conn: &DatabaseConnection) -> usize {
    (conn as *const DatabaseConnection) as usize
}

pub fn register_mock_strict_connection(conn: &DatabaseConnection) {
    let mut set = registry().lock().expect("mock registry poisoned");
    set.insert(key(conn));
}

pub fn is_mock_strict(conn: &DatabaseConnection) -> bool {
    let set = registry().lock().expect("mock registry poisoned");
    set.contains(&key(conn))
}
