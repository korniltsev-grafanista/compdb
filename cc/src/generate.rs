use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use serde_json::{json, Value};

/// Default compiler used when log entry doesn't specify one (for backwards compatibility).
const DEFAULT_COMPILER: &str = "/usr/bin/gcc";

/// Parse a single log entry and return a compilation database entry if valid.
/// Returns None if the entry has no source files or invalid format.
pub fn parse_log_entry(line: &str, wd_override: Option<&str>) -> Option<Value> {
    let it: Value = serde_json::from_str(line).ok()?;

    let wd = wd_override.unwrap_or_else(|| it["wd"].as_str().unwrap_or(""));
    let compiler = it["compiler"].as_str().unwrap_or(DEFAULT_COMPILER);
    let args_value = &it["args"];

    if !args_value.is_array() {
        return None;
    }

    let mut args = vec![compiler.to_string()];
    for arg in args_value.as_array().unwrap() {
        if let Some(arg_str) = arg.as_str() {
            args.push(arg_str.to_string());
        }
    }

    // Find source files in arguments
    let srcs = find_source_files(&args, wd);

    if srcs.is_empty() {
        return None;
    }

    Some(json!({
        "directory": wd,
        "arguments": args,
        "file": srcs.last().unwrap(),
    }))
}

/// Find source files in the arguments list, returning their full paths.
pub fn find_source_files(args: &[String], wd: &str) -> Vec<String> {
    args.iter()
        .filter(|arg| arg.ends_with(".c") || arg.ends_with(".cc") || arg.ends_with(".cpp"))
        .map(|arg| Path::new(wd).join(arg).to_string_lossy().to_string())
        .collect()
}

/// Generate a compilation database from a log file.
/// Writes output to the specified destination file.
pub fn generate_db(log_file: &str, dst: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let file = File::open(log_file)?;
    let reader = BufReader::new(file);

    let mut db = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(entry) = parse_log_entry(&line, None) {
            db.push(entry);
        } else {
            eprintln!("warning no src {}", line);
        }
    }

    let mut output = File::create(dst)?;
    output.write_all(serde_json::to_string_pretty(&db)?.as_bytes())?;

    Ok(db)
}

pub fn run(log_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    generate_db(log_file, "compile_commands.json")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ==================== find_source_files tests ====================

    mod find_source_files_tests {
        use super::*;

        #[test]
        fn finds_c_files() {
            let args = vec![
                "/usr/bin/gcc".to_string(),
                "-c".to_string(),
                "main.c".to_string(),
            ];
            let result = find_source_files(&args, "/project");
            assert_eq!(result, vec!["/project/main.c"]);
        }

        #[test]
        fn finds_cc_files() {
            let args = vec![
                "/usr/bin/gcc".to_string(),
                "-c".to_string(),
                "main.cc".to_string(),
            ];
            let result = find_source_files(&args, "/project");
            assert_eq!(result, vec!["/project/main.cc"]);
        }

        #[test]
        fn finds_cpp_files() {
            let args = vec![
                "/usr/bin/gcc".to_string(),
                "-c".to_string(),
                "main.cpp".to_string(),
            ];
            let result = find_source_files(&args, "/project");
            assert_eq!(result, vec!["/project/main.cpp"]);
        }

        #[test]
        fn finds_multiple_source_files() {
            let args = vec![
                "/usr/bin/gcc".to_string(),
                "-c".to_string(),
                "main.c".to_string(),
                "util.c".to_string(),
                "helper.cpp".to_string(),
            ];
            let result = find_source_files(&args, "/project");
            assert_eq!(result.len(), 3);
            assert!(result.contains(&"/project/main.c".to_string()));
            assert!(result.contains(&"/project/util.c".to_string()));
            assert!(result.contains(&"/project/helper.cpp".to_string()));
        }

        #[test]
        fn returns_empty_for_no_source_files() {
            let args = vec![
                "/usr/bin/gcc".to_string(),
                "-c".to_string(),
                "-o".to_string(),
                "output.o".to_string(),
            ];
            let result = find_source_files(&args, "/project");
            assert!(result.is_empty());
        }

        #[test]
        fn handles_empty_args() {
            let args: Vec<String> = vec![];
            let result = find_source_files(&args, "/project");
            assert!(result.is_empty());
        }

        #[test]
        fn handles_nested_paths() {
            let args = vec![
                "/usr/bin/gcc".to_string(),
                "-c".to_string(),
                "src/lib/util.c".to_string(),
            ];
            let result = find_source_files(&args, "/project");
            assert_eq!(result, vec!["/project/src/lib/util.c"]);
        }

        #[test]
        fn ignores_header_files() {
            let args = vec![
                "/usr/bin/gcc".to_string(),
                "-I".to_string(),
                "include".to_string(),
                "-c".to_string(),
                "main.c".to_string(),
                "header.h".to_string(),
            ];
            let result = find_source_files(&args, "/project");
            assert_eq!(result, vec!["/project/main.c"]);
        }

        #[test]
        fn ignores_object_files() {
            let args = vec![
                "/usr/bin/gcc".to_string(),
                "-c".to_string(),
                "main.c".to_string(),
                "-o".to_string(),
                "main.o".to_string(),
            ];
            let result = find_source_files(&args, "/project");
            assert_eq!(result, vec!["/project/main.c"]);
        }
    }

    // ==================== parse_log_entry tests ====================

    mod parse_log_entry_tests {
        use super::*;

        #[test]
        fn parses_valid_c_entry() {
            let line = r#"{"wd":"/project","args":["-c","main.c"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            assert_eq!(entry["directory"], "/project");
            assert_eq!(entry["file"], "/project/main.c");
        }

        #[test]
        fn parses_valid_cpp_entry() {
            let line = r#"{"wd":"/project","args":["-c","main.cpp"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            assert_eq!(entry["file"], "/project/main.cpp");
        }

        #[test]
        fn parses_valid_cc_entry() {
            let line = r#"{"wd":"/project","args":["-c","main.cc"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            assert_eq!(entry["file"], "/project/main.cc");
        }

        #[test]
        fn returns_none_for_no_source_files() {
            let line = r#"{"wd":"/project","args":["-o","output.o"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_none());
        }

        #[test]
        fn returns_none_for_invalid_json() {
            let line = "not valid json";
            let result = parse_log_entry(line, None);
            assert!(result.is_none());
        }

        #[test]
        fn returns_none_for_non_array_args() {
            let line = r#"{"wd":"/project","args":"not an array"}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_none());
        }

        #[test]
        fn returns_none_for_missing_args() {
            let line = r#"{"wd":"/project"}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_none());
        }

        #[test]
        fn uses_wd_override_when_provided() {
            let line = r#"{"wd":"/original","args":["-c","main.c"]}"#;
            let result = parse_log_entry(line, Some("/override"));
            assert!(result.is_some());
            let entry = result.unwrap();
            assert_eq!(entry["directory"], "/override");
            assert_eq!(entry["file"], "/override/main.c");
        }

        #[test]
        fn uses_empty_string_for_missing_wd() {
            let line = r#"{"args":["-c","main.c"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            assert_eq!(entry["directory"], "");
        }

        #[test]
        fn uses_default_compiler_when_not_specified() {
            let line = r#"{"wd":"/project","args":["-c","main.c","-O2"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            let args = entry["arguments"].as_array().unwrap();
            assert_eq!(args[0], "/usr/bin/gcc");
            assert_eq!(args[1], "-c");
            assert_eq!(args[2], "main.c");
            assert_eq!(args[3], "-O2");
        }

        #[test]
        fn uses_compiler_from_log_entry() {
            let line = r#"{"wd":"/project","compiler":"clang","args":["-c","main.c"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            let args = entry["arguments"].as_array().unwrap();
            assert_eq!(args[0], "clang");
            assert_eq!(args[1], "-c");
            assert_eq!(args[2], "main.c");
        }

        #[test]
        fn uses_full_path_compiler_from_log_entry() {
            let line = r#"{"wd":"/project","compiler":"/usr/local/bin/gcc-12","args":["-c","main.c"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            let args = entry["arguments"].as_array().unwrap();
            assert_eq!(args[0], "/usr/local/bin/gcc-12");
        }

        #[test]
        fn uses_clangpp_compiler_from_log_entry() {
            let line = r#"{"wd":"/project","compiler":"clang++","args":["-c","main.cpp"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            let args = entry["arguments"].as_array().unwrap();
            assert_eq!(args[0], "clang++");
        }

        #[test]
        fn handles_complex_compiler_flags() {
            let line = r#"{"wd":"/project","args":["-c","-Wall","-Wextra","-I/include","-DDEBUG=1","src/main.c","-o","main.o"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            assert_eq!(entry["file"], "/project/src/main.c");
        }

        #[test]
        fn uses_last_source_file_when_multiple() {
            let line = r#"{"wd":"/project","args":["-c","first.c","second.c","third.c"]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            assert_eq!(entry["file"], "/project/third.c");
        }

        #[test]
        fn skips_non_string_args() {
            let line = r#"{"wd":"/project","args":["-c",123,"main.c",null]}"#;
            let result = parse_log_entry(line, None);
            assert!(result.is_some());
            let entry = result.unwrap();
            assert_eq!(entry["file"], "/project/main.c");
        }
    }

    // ==================== generate_db tests ====================

    mod generate_db_tests {
        use super::*;

        fn create_log_file(dir: &TempDir, content: &str) -> String {
            let log_path = dir.path().join("cc_hook.txt");
            fs::write(&log_path, content).unwrap();
            log_path.to_string_lossy().to_string()
        }

        #[test]
        fn generates_empty_db_for_empty_log() {
            let temp_dir = TempDir::new().unwrap();
            let log_file = create_log_file(&temp_dir, "");
            let dst = temp_dir.path().join("compile_commands.json");

            let result = generate_db(&log_file, dst.to_str().unwrap());
            assert!(result.is_ok());
            let db = result.unwrap();
            assert!(db.is_empty());

            let content = fs::read_to_string(&dst).unwrap();
            assert_eq!(content.trim(), "[]");
        }

        #[test]
        fn generates_single_entry_db() {
            let temp_dir = TempDir::new().unwrap();
            let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
            let log_file = create_log_file(&temp_dir, log_content);
            let dst = temp_dir.path().join("compile_commands.json");

            let result = generate_db(&log_file, dst.to_str().unwrap());
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.len(), 1);
            assert_eq!(db[0]["directory"], "/project");
            assert_eq!(db[0]["file"], "/project/main.c");
        }

        #[test]
        fn generates_multiple_entry_db() {
            let temp_dir = TempDir::new().unwrap();
            let log_content = r#"{"wd":"/project","args":["-c","main.c"]}
{"wd":"/project","args":["-c","util.c"]}
{"wd":"/project/lib","args":["-c","helper.cpp"]}"#;
            let log_file = create_log_file(&temp_dir, log_content);
            let dst = temp_dir.path().join("compile_commands.json");

            let result = generate_db(&log_file, dst.to_str().unwrap());
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.len(), 3);
            assert_eq!(db[0]["file"], "/project/main.c");
            assert_eq!(db[1]["file"], "/project/util.c");
            assert_eq!(db[2]["file"], "/project/lib/helper.cpp");
        }

        #[test]
        fn skips_entries_without_source_files() {
            let temp_dir = TempDir::new().unwrap();
            let log_content = r#"{"wd":"/project","args":["-c","main.c"]}
{"wd":"/project","args":["-o","output.o"]}
{"wd":"/project","args":["-c","util.c"]}"#;
            let log_file = create_log_file(&temp_dir, log_content);
            let dst = temp_dir.path().join("compile_commands.json");

            let result = generate_db(&log_file, dst.to_str().unwrap());
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.len(), 2);
        }

        #[test]
        fn skips_invalid_json_lines() {
            let temp_dir = TempDir::new().unwrap();
            let log_content = r#"{"wd":"/project","args":["-c","main.c"]}
not valid json
{"wd":"/project","args":["-c","util.c"]}"#;
            let log_file = create_log_file(&temp_dir, log_content);
            let dst = temp_dir.path().join("compile_commands.json");

            let result = generate_db(&log_file, dst.to_str().unwrap());
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.len(), 2);
        }

        #[test]
        fn returns_error_for_missing_log_file() {
            let temp_dir = TempDir::new().unwrap();
            let log_file = temp_dir.path().join("nonexistent.txt");
            let dst = temp_dir.path().join("compile_commands.json");

            let result = generate_db(log_file.to_str().unwrap(), dst.to_str().unwrap());
            assert!(result.is_err());
        }

        #[test]
        fn writes_pretty_printed_json() {
            let temp_dir = TempDir::new().unwrap();
            let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
            let log_file = create_log_file(&temp_dir, log_content);
            let dst = temp_dir.path().join("compile_commands.json");

            generate_db(&log_file, dst.to_str().unwrap()).unwrap();

            let content = fs::read_to_string(&dst).unwrap();
            assert!(content.contains('\n'));
            assert!(content.contains("  "));
        }

        #[test]
        fn handles_mixed_file_types() {
            let temp_dir = TempDir::new().unwrap();
            let log_content = r#"{"wd":"/project","args":["-c","main.c"]}
{"wd":"/project","args":["-c","app.cc"]}
{"wd":"/project","args":["-c","util.cpp"]}"#;
            let log_file = create_log_file(&temp_dir, log_content);
            let dst = temp_dir.path().join("compile_commands.json");

            let result = generate_db(&log_file, dst.to_str().unwrap());
            assert!(result.is_ok());
            let db = result.unwrap();
            assert_eq!(db.len(), 3);
            assert!(db[0]["file"].as_str().unwrap().ends_with(".c"));
            assert!(db[1]["file"].as_str().unwrap().ends_with(".cc"));
            assert!(db[2]["file"].as_str().unwrap().ends_with(".cpp"));
        }

        #[test]
        fn preserves_complex_arguments() {
            let temp_dir = TempDir::new().unwrap();
            let log_content = r#"{"wd":"/project","args":["-c","-Wall","-O2","-I/usr/include","-DDEBUG","main.c"]}"#;
            let log_file = create_log_file(&temp_dir, log_content);
            let dst = temp_dir.path().join("compile_commands.json");

            let result = generate_db(&log_file, dst.to_str().unwrap());
            assert!(result.is_ok());
            let db = result.unwrap();
            let args = db[0]["arguments"].as_array().unwrap();
            assert!(args.iter().any(|a| a == "-Wall"));
            assert!(args.iter().any(|a| a == "-O2"));
            assert!(args.iter().any(|a| a == "-I/usr/include"));
            assert!(args.iter().any(|a| a == "-DDEBUG"));
        }
    }

    // ==================== run function tests ====================

    mod run_tests {
        use super::*;

        #[test]
        fn run_creates_compile_commands_json_in_cwd() {
            let temp_dir = TempDir::new().unwrap();
            let original_dir = std::env::current_dir().unwrap();

            // Change to temp dir
            std::env::set_current_dir(temp_dir.path()).unwrap();

            // Create log file
            let log_content = r#"{"wd":"/project","args":["-c","main.c"]}"#;
            fs::write("cc_hook.txt", log_content).unwrap();

            let result = run("cc_hook.txt");
            assert!(result.is_ok());
            assert!(temp_dir.path().join("compile_commands.json").exists());

            // Restore original directory
            std::env::set_current_dir(original_dir).unwrap();
        }

        #[test]
        fn run_returns_error_for_missing_log() {
            let result = run("/nonexistent/cc_hook.txt");
            assert!(result.is_err());
        }
    }
}
