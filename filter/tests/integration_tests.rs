use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use tempfile::TempDir;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct CompileCommand {
    command: String,
    directory: String,
    file: String,
}

fn sample_compile_db() -> Vec<CompileCommand> {
    vec![
        CompileCommand {
            command: "gcc -c src/main.c".to_string(),
            directory: "/build".to_string(),
            file: "src/main.c".to_string(),
        },
        CompileCommand {
            command: "gcc -c src/util.c".to_string(),
            directory: "/build".to_string(),
            file: "src/util.c".to_string(),
        },
        CompileCommand {
            command: "gcc -c tests/test.c".to_string(),
            directory: "/build".to_string(),
            file: "tests/test.c".to_string(),
        },
        CompileCommand {
            command: "gcc -c vendor/lib.c".to_string(),
            directory: "/build".to_string(),
            file: "vendor/lib.c".to_string(),
        },
    ]
}

#[test]
fn test_cli_default_path() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 4 -> 4 entries"));

    // Verify backup was created
    assert!(temp_dir.path().join("compile_commands.json.bak").exists());
}

#[test]
fn test_cli_custom_path() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("custom.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .arg(db_path.to_str().unwrap())
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 4 -> 4 entries"));

    // Verify backup was created
    assert!(temp_dir.path().join("custom.json.bak").exists());
}

#[test]
fn test_cli_exclude_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .arg("-e")
        .arg("^tests/")
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 4 -> 3 entries"));

    // Verify filtered content
    let content = fs::read_to_string(&db_path).unwrap();
    let filtered: Vec<CompileCommand> = serde_json::from_str(&content).unwrap();
    assert_eq!(filtered.len(), 3);
    assert!(!filtered.iter().any(|c| c.file.starts_with("tests/")));
}

#[test]
fn test_cli_multiple_exclude_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .arg("-e")
        .arg("^tests/")
        .arg("-e")
        .arg("^vendor/")
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 4 -> 2 entries"));

    // Verify filtered content
    let content = fs::read_to_string(&db_path).unwrap();
    let filtered: Vec<CompileCommand> = serde_json::from_str(&content).unwrap();
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].file, "src/main.c");
    assert_eq!(filtered[1].file, "src/util.c");
}

#[test]
fn test_cli_include_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let mut commands = sample_compile_db();
    commands.push(CompileCommand {
        command: "gcc -c tests/integration_test.c".to_string(),
        directory: "/build".to_string(),
        file: "tests/integration_test.c".to_string(),
    });
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .arg("-e")
        .arg("^tests/")
        .arg("-i")
        .arg("integration")
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 5 -> 4 entries"));

    // Verify filtered content
    let content = fs::read_to_string(&db_path).unwrap();
    let filtered: Vec<CompileCommand> = serde_json::from_str(&content).unwrap();
    assert_eq!(filtered.len(), 4);
    assert!(filtered.iter().any(|c| c.file == "tests/integration_test.c"));
    assert!(!filtered.iter().any(|c| c.file == "tests/test.c"));
}

#[test]
fn test_cli_backup_preservation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    // First run
    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let first_backup = temp_dir.path().join("compile_commands.json.bak");
    assert!(first_backup.exists());
    let first_backup_content = fs::read_to_string(&first_backup).unwrap();

    // Second run - should create .bak.1
    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .assert()
        .success();

    let second_backup = temp_dir.path().join("compile_commands.json.bak.1");
    assert!(second_backup.exists());
    // First backup should be unchanged
    assert_eq!(
        fs::read_to_string(&first_backup).unwrap(),
        first_backup_content
    );
}

#[test]
fn test_cli_missing_file() {
    cargo_bin_cmd!("compdb-filter")
        .arg("nonexistent.json")
        .assert()
        .failure();
}

#[test]
fn test_cli_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    fs::write(&db_path, "invalid json {").unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .assert()
        .failure();
}

#[test]
fn test_cli_invalid_regex() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .arg("-e")
        .arg("[invalid")
        .assert()
        .failure();
}

#[test]
fn test_cli_empty_database() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands: Vec<CompileCommand> = vec![];
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 0 -> 0 entries"));
}

#[test]
fn test_cli_all_filtered_out() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .arg("-e")
        .arg(".*")  // Exclude everything
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 4 -> 0 entries"));

    // Verify result is empty array
    let content = fs::read_to_string(&db_path).unwrap();
    let filtered: Vec<CompileCommand> = serde_json::from_str(&content).unwrap();
    assert_eq!(filtered.len(), 0);
}

#[test]
fn test_cli_long_option_names() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .arg("--exclude")
        .arg("^tests/")
        .arg("--include")
        .arg("integration")
        .assert()
        .success();
}

#[test]
fn test_cli_complex_filtering_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = vec![
        CompileCommand {
            command: "gcc -c kernel/init.c".to_string(),
            directory: "/build".to_string(),
            file: "kernel/init.c".to_string(),
        },
        CompileCommand {
            command: "gcc -c kernel/drivers/usb.c".to_string(),
            directory: "/build".to_string(),
            file: "kernel/drivers/usb.c".to_string(),
        },
        CompileCommand {
            command: "gcc -c kernel/drivers/pci.c".to_string(),
            directory: "/build".to_string(),
            file: "kernel/drivers/pci.c".to_string(),
        },
        CompileCommand {
            command: "gcc -c arch/x86/boot.c".to_string(),
            directory: "/build".to_string(),
            file: "arch/x86/boot.c".to_string(),
        },
        CompileCommand {
            command: "gcc -c arch/arm/boot.c".to_string(),
            directory: "/build".to_string(),
            file: "arch/arm/boot.c".to_string(),
        },
    ];
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    // Exclude all drivers and arm, but include USB driver
    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .arg("-e")
        .arg("drivers/")
        .arg("-e")
        .arg("arch/arm")
        .arg("-i")
        .arg("usb")
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 5 -> 3 entries"));

    let content = fs::read_to_string(&db_path).unwrap();
    let filtered: Vec<CompileCommand> = serde_json::from_str(&content).unwrap();
    assert_eq!(filtered.len(), 3);
    assert_eq!(filtered[0].file, "kernel/init.c");
    assert_eq!(filtered[1].file, "kernel/drivers/usb.c");
    assert_eq!(filtered[2].file, "arch/x86/boot.c");
}

#[test]
fn test_cli_stats_output_format() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("compile_commands.json");

    let commands = sample_compile_db();
    fs::write(&db_path, serde_json::to_string_pretty(&commands).unwrap()).unwrap();

    cargo_bin_cmd!("compdb-filter")
        .current_dir(temp_dir.path())
        .arg("-e")
        .arg("^vendor/")
        .assert()
        .success()
        .stderr(predicate::str::contains("Filtered: 4 -> 3 entries (1 removed)"));
}
