// apps/backend/migration/src/main.rs
use ::backend::{
    config::db::{DbOwner, DbProfile},
    infra::db::connect_db,
};
use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{Statement};
use std::env;

#[tokio::main]
async fn main() {
    // Select prod|test via env (your pattern)
    let profile = match env::var("MIGRATION_TARGET").as_deref() {
        Ok("test") => DbProfile::Test,
        _ => DbProfile::Prod,
    };

    // Subcommand: up | down | fresh | reset | refresh | status
    let cmd = env::args().nth(1).unwrap_or_else(|| "up".to_string());

    // Connect with owner privileges (can create/drop types/tables)
    let db = connect_db(profile.clone(), DbOwner::Owner)
        .await
        .expect("Failed to connect to database");

    // ----- Diagnostics (safe to keep, helpful when things look 'no-op') -----
    println!("▶ cmd={cmd}  profile={profile:?}");
    let stmt = Statement::from_string(
        db.get_database_backend(),
        String::from("select current_database() as name"),
    );
    if let Some(row) = db.query_one(stmt).await.expect("current_database() failed") {
        let db_name: String = row.try_get("", "name").expect("extract db name");
        println!("▶ connected to DB: {db_name}");
    } else {
        println!("▶ connected to DB: <unknown>");
    }
    let mig_count = <migration::Migrator as MigratorTrait>::migrations().len();
    println!("▶ runner sees {mig_count} migration(s)");
    // -----------------------------------------------------------------------

    // Dispatch to SeaORM's runner
    use migration::Migrator;
    let result = match cmd.as_str() {
        "up" => Migrator::up(&db, None).await,
        "down" => Migrator::down(&db, None).await,
        "fresh" => Migrator::fresh(&db).await,     // drop managed objs, then all up()
        "reset" => Migrator::reset(&db).await,     // all down(), then all up()  (best w/ enums)
        "refresh" => Migrator::refresh(&db).await, // down last, then up last
        "status" => Migrator::status(&db).await,
        other => {
            eprintln!("Unknown command: {other}. Use: up | down | fresh | reset | refresh | status");
            std::process::exit(2);
        }
    };

    match result {
        Ok(()) => println!("✅ {cmd} OK for {:?}", profile),
        Err(e) => {
            eprintln!("❌ {cmd} failed for {:?}: {e}", profile);
            std::process::exit(1);
        }
    }
}
