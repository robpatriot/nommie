use std::env;

pub fn ensure_test_database() {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!("DATABASE_URL environment variable is required for tests");
    });
    
    if !database_url.ends_with("_test") {
        panic!("Tests must run against a test database (ending with '_test'). Current DATABASE_URL: {}", database_url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_guard() {
        // This test will fail if not running against a test database
        ensure_test_database();
    }
}
