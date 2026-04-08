use std::fs;
use std::path::Path;
use std::process::Command;

use mempal_core::db::Database;
use serde_json::Value;
use tempfile::tempdir;

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).expect("fixture file should be written");
}

fn run_cli(home: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mempal"))
        .env("HOME", home)
        .args(args)
        .output()
        .expect("cli should run")
}

#[test]
fn test_e2e_init_ingest_search() {
    let home = tempdir().expect("home temp dir should be created");
    let project = tempdir().expect("project temp dir should be created");
    let src_auth = project.path().join("src").join("auth");
    fs::create_dir_all(&src_auth).expect("project directories should be created");
    write_file(
        &project.path().join("README.md"),
        "database decision: we decided to use PostgreSQL for analytics.",
    );
    write_file(&src_auth.join("mod.rs"), "pub fn login() {}");

    let init = run_cli(
        home.path(),
        &["init", project.path().to_str().expect("valid path")],
    );
    assert!(init.status.success(), "init failed: {:?}", init);
    let init_stdout = String::from_utf8(init.stdout).expect("stdout should be utf8");
    assert!(init_stdout.contains("auth"));

    let db_path = home.path().join(".mempal").join("palace.db");
    let db = Database::open(&db_path).expect("database should open");
    let taxonomy_count: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM taxonomy", [], |row| row.get(0))
        .expect("taxonomy count query should succeed");
    assert!(taxonomy_count > 0);

    let ingest = run_cli(
        home.path(),
        &[
            "ingest",
            project.path().to_str().expect("valid path"),
            "--wing",
            "myproject",
        ],
    );
    assert!(ingest.status.success(), "ingest failed: {:?}", ingest);
    let ingest_stdout = String::from_utf8(ingest.stdout).expect("stdout should be utf8");
    assert!(ingest_stdout.contains("chunks"));

    let search = run_cli(
        home.path(),
        &[
            "search",
            "database decision postgresql analytics",
            "--json",
            "--wing",
            "myproject",
        ],
    );
    assert!(search.status.success(), "search failed: {:?}", search);
    let search_stdout = String::from_utf8(search.stdout).expect("stdout should be utf8");
    let results: Value =
        serde_json::from_str(&search_stdout).expect("search output should be JSON");
    let first = results
        .as_array()
        .and_then(|items| items.first())
        .expect("search should return at least one result");
    let source_file = first
        .get("source_file")
        .and_then(Value::as_str)
        .expect("result should include source_file");
    assert!(source_file.ends_with("README.md"));
}
