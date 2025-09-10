use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use sea_orm::DatabaseConnection;

#[allow(dead_code)]
static REGISTRY: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();

#[allow(dead_code)]
fn registry() -> &'static Mutex<HashSet<usize>> {
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

#[allow(dead_code)]
fn key(conn: &DatabaseConnection) -> usize {
    (conn as *const DatabaseConnection) as usize
}

#[allow(dead_code)]
pub fn register_mock_strict_connection(conn: &DatabaseConnection) {
    let mut set = registry().lock().expect("mock registry poisoned");
    set.insert(key(conn));
}

#[allow(dead_code)]
pub fn is_mock_strict(conn: &DatabaseConnection) -> bool {
    let set = registry().lock().expect("mock registry poisoned");
    set.contains(&key(conn))
}
