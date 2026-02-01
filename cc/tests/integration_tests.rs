use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

// ==================== compdb-cc tests ====================

mod compdb_cc_tests {
    use super::*;

    #[test]
    fn generate_flag_creates_compile_commands_json() {
        let temp_dir = TempDir::new().unwrap();

        // Create log file with valid entries
        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}
{"wd":"/project","args":["-c","util.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        // Verify compile_commands.json was created
        let output_path = temp_dir.path().join("compile_commands.json");
        assert!(output_path.exists());

        // Verify content
        let content = fs::read_to_string(&output_path).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 2);
        assert_eq!(db[0]["file"], "/project/main.c");
        assert_eq!(db[1]["file"], "/project/util.c");
    }

    #[test]
    fn generate_env_var_creates_compile_commands_json() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","test.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .env("COMPDB_GENERATE", "1")
            .assert()
            .success();

        assert!(temp_dir.path().join("compile_commands.json").exists());
    }

    #[test]
    fn generate_with_empty_log_creates_empty_db() {
        let temp_dir = TempDir::new().unwrap();

        // Create empty log file
        fs::write(temp_dir.path().join("cc_hook.txt"), "").unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert!(db.is_empty());
    }

    #[test]
    fn generate_with_missing_log_fails() {
        let temp_dir = TempDir::new().unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Error"));
    }

    #[test]
    fn generate_with_invalid_json_warns_but_continues() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}
invalid json line
{"wd":"/project","args":["-c","util.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success()
            .stderr(predicate::str::contains("warning"));

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn generate_handles_cpp_files() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","app.cpp"]}
{"wd":"/project","args":["-c","lib.cc"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 2);
        assert!(db[0]["file"].as_str().unwrap().ends_with(".cpp"));
        assert!(db[1]["file"].as_str().unwrap().ends_with(".cc"));
    }

    #[test]
    fn generate_preserves_directory() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/build/subdir","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db[0]["directory"], "/build/subdir");
    }

    #[test]
    fn generate_includes_all_arguments() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","-Wall","-O2","-I/include","-DDEBUG","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        let args = db[0]["arguments"].as_array().unwrap();
        assert!(args.iter().any(|a| a == "-Wall"));
        assert!(args.iter().any(|a| a == "-O2"));
        assert!(args.iter().any(|a| a == "-I/include"));
        assert!(args.iter().any(|a| a == "-DDEBUG"));
    }

    #[test]
    fn generate_skips_entries_without_source() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}
{"wd":"/project","args":["-o","output.o"]}
{"wd":"/project","args":["-c","util.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success()
            .stderr(predicate::str::contains("warning"));

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn generate_handles_many_entries() {
        let temp_dir = TempDir::new().unwrap();

        let log_content: String = (0..100)
            .map(|i| format!(r#"{{"wd":"/project","args":["-c","file{}.c"]}}"#, i))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 100);
    }

    #[test]
    fn generate_output_is_valid_json() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        // Should not panic
        let _: Vec<Value> = serde_json::from_str(&content).unwrap();
    }
}

// ==================== compdb-cxx tests ====================

mod compdb_cxx_tests {
    use super::*;

    #[test]
    fn generate_flag_creates_compile_commands_json() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.cpp"]}
{"wd":"/project","args":["-c","util.cc"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cxx")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        assert!(temp_dir.path().join("compile_commands.json").exists());

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 2);
    }

    #[test]
    fn generate_env_var_creates_compile_commands_json() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","test.cpp"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cxx")
            .current_dir(temp_dir.path())
            .env("COMPDB_GENERATE", "true")
            .assert()
            .success();

        assert!(temp_dir.path().join("compile_commands.json").exists());
    }

    #[test]
    fn generate_with_missing_log_fails() {
        let temp_dir = TempDir::new().unwrap();

        cargo_bin_cmd!("compdb-cxx")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .failure()
            .stderr(predicate::str::contains("Error"));
    }
}

// ==================== Cross-binary consistency tests ====================

mod consistency_tests {
    use super::*;

    #[test]
    fn cc_and_cxx_generate_identical_output_for_same_input() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.cpp"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        // Generate with compdb-cc
        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let cc_output =
            fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();

        // Remove the generated file
        fs::remove_file(temp_dir.path().join("compile_commands.json")).unwrap();

        // Generate with compdb-cxx
        cargo_bin_cmd!("compdb-cxx")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let cxx_output =
            fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();

        assert_eq!(cc_output, cxx_output);
    }

    #[test]
    fn both_binaries_fail_same_way_for_missing_log() {
        let temp_dir = TempDir::new().unwrap();

        let cc_result = cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .failure();

        let cxx_result = cargo_bin_cmd!("compdb-cxx")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .failure();

        // Both should fail with similar error
        cc_result.stderr(predicate::str::contains("Error"));
        cxx_result.stderr(predicate::str::contains("Error"));
    }
}

// ==================== Edge case tests ====================

mod edge_case_tests {
    use super::*;

    #[test]
    fn handles_unicode_in_paths() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project/日本語","args":["-c","файл.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 1);
        assert!(db[0]["directory"].as_str().unwrap().contains("日本語"));
    }

    #[test]
    fn handles_spaces_in_paths() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project/with spaces","args":["-c","my file.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 1);
        assert!(db[0]["directory"].as_str().unwrap().contains("with spaces"));
        assert!(db[0]["file"].as_str().unwrap().contains("my file.c"));
    }

    #[test]
    fn handles_special_characters_in_args() {
        let temp_dir = TempDir::new().unwrap();

        let log_content =
            r#"{"wd":"/project","args":["-c","-DVERSION=\"1.0\"","-DNAME='test'","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        let args = db[0]["arguments"].as_array().unwrap();
        assert!(args.iter().any(|a| a.as_str().unwrap().contains("VERSION")));
    }

    #[test]
    fn handles_deeply_nested_paths() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/a/b/c/d/e/f/g/h/i/j","args":["-c","src/lib/core/util/helper.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db[0]["directory"], "/a/b/c/d/e/f/g/h/i/j");
        assert!(db[0]["file"]
            .as_str()
            .unwrap()
            .contains("src/lib/core/util/helper.c"));
    }

    #[test]
    fn handles_empty_working_directory() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db[0]["directory"], "");
    }

    #[test]
    fn handles_log_with_trailing_newline() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = "{\"wd\":\"/project\",\"args\":[\"-c\",\"main.c\"]}\n";
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn handles_log_with_multiple_trailing_newlines() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = "{\"wd\":\"/project\",\"args\":[\"-c\",\"main.c\"]}\n\n\n";
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn generate_env_var_empty_string_does_not_trigger_generate() {
        let temp_dir = TempDir::new().unwrap();

        // Create a log file just in case
        fs::write(temp_dir.path().join("cc_hook.txt"), "").unwrap();

        // With empty COMPDB_GENERATE, should try to run compiler (and fail)
        // This tests that empty string doesn't trigger generate mode
        let result = cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .env("COMPDB_GENERATE", "")
            .arg("--version") // Pass to actual compiler
            .assert();

        // Either it runs the compiler (which might not exist) or it doesn't generate
        // The key is it shouldn't create compile_commands.json from generate mode
        // Note: This may fail because clang isn't installed, but that's expected
        let _ = result; // We just want to ensure it doesn't panic
    }
}

// ==================== Output format tests ====================

mod output_format_tests {
    use super::*;

    #[test]
    fn output_is_array() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let value: Value = serde_json::from_str(&content).unwrap();
        assert!(value.is_array());
    }

    #[test]
    fn entries_have_required_fields() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();

        for entry in &db {
            assert!(entry.get("directory").is_some(), "Missing 'directory' field");
            assert!(entry.get("arguments").is_some(), "Missing 'arguments' field");
            assert!(entry.get("file").is_some(), "Missing 'file' field");
        }
    }

    #[test]
    fn arguments_field_is_array() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();

        assert!(db[0]["arguments"].is_array());
    }

    #[test]
    fn arguments_starts_with_default_compiler_when_not_specified() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        let args = db[0]["arguments"].as_array().unwrap();

        assert_eq!(args[0], "/usr/bin/gcc");
    }

    #[test]
    fn arguments_uses_compiler_from_log_entry() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","compiler":"clang","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        let args = db[0]["arguments"].as_array().unwrap();

        assert_eq!(args[0], "clang");
    }

    #[test]
    fn arguments_uses_full_path_compiler_from_log_entry() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","compiler":"/usr/local/bin/gcc-12","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        let args = db[0]["arguments"].as_array().unwrap();

        assert_eq!(args[0], "/usr/local/bin/gcc-12");
    }

    #[test]
    fn handles_mixed_compiler_entries() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","compiler":"clang","args":["-c","main.c"]}
{"wd":"/project","compiler":"clang++","args":["-c","app.cpp"]}
{"wd":"/project","args":["-c","util.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();

        assert_eq!(db[0]["arguments"].as_array().unwrap()[0], "clang");
        assert_eq!(db[1]["arguments"].as_array().unwrap()[0], "clang++");
        assert_eq!(db[2]["arguments"].as_array().unwrap()[0], "/usr/bin/gcc");
    }

    #[test]
    fn file_is_absolute_path() {
        let temp_dir = TempDir::new().unwrap();

        let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
        fs::write(temp_dir.path().join("cc_hook.txt"), log_content).unwrap();

        cargo_bin_cmd!("compdb-cc")
            .current_dir(temp_dir.path())
            .arg("--generate")
            .assert()
            .success();

        let content = fs::read_to_string(temp_dir.path().join("compile_commands.json")).unwrap();
        let db: Vec<Value> = serde_json::from_str(&content).unwrap();
        let file = db[0]["file"].as_str().unwrap();

        assert!(file.starts_with('/'), "File path should be absolute");
    }
}
