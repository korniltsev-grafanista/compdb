use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CompileCommand {
    pub command: String,
    pub directory: String,
    pub file: String,
}

#[derive(Parser)]
#[command(name = "compdbfilter")]
#[command(about = "Filter compile_commands.json by regex patterns")]
struct Cli {
    /// Path to compile_commands.json
    #[arg(default_value = "./compile_commands.json")]
    path: PathBuf,

    /// Exclude files matching this regex (can be repeated)
    #[arg(short, long, value_name = "REGEX")]
    exclude: Vec<String>,

    /// Include files matching this regex even if excluded (can be repeated)
    #[arg(short, long, value_name = "REGEX")]
    include: Vec<String>,
}

/// Find the next available backup path that doesn't exist.
/// Returns paths like: file.bak, file.bak.1, file.bak.2, etc.
pub fn find_backup_path(original: &PathBuf) -> PathBuf {
    let base = format!("{}.bak", original.display());
    let base_path = PathBuf::from(&base);

    if !base_path.exists() {
        return base_path;
    }

    let mut counter = 1;
    loop {
        let numbered = PathBuf::from(format!("{}.{}", base, counter));
        if !numbered.exists() {
            return numbered;
        }
        counter += 1;
    }
}

/// Filter compile commands based on exclude and include regex patterns.
/// A command is kept if:
/// - It doesn't match any exclude pattern, OR
/// - It matches an exclude pattern BUT also matches an include pattern (override)
pub fn filter_commands(
    commands: Vec<CompileCommand>,
    exclude_patterns: &[Regex],
    include_patterns: &[Regex],
) -> Vec<CompileCommand> {
    commands
        .into_iter()
        .filter(|cmd| {
            let excluded = exclude_patterns.iter().any(|re| re.is_match(&cmd.file));
            if !excluded {
                return true;
            }
            // Check if included overrides exclusion
            include_patterns.iter().any(|re| re.is_match(&cmd.file))
        })
        .collect()
}

/// Compile a list of regex pattern strings into Regex objects.
pub fn compile_patterns(patterns: &[String]) -> Result<Vec<Regex>, regex::Error> {
    patterns.iter().map(|p| Regex::new(p)).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Compile regex patterns
    let exclude_patterns = compile_patterns(&cli.exclude)?;
    let include_patterns = compile_patterns(&cli.include)?;

    // Read compile_commands.json
    let content = fs::read_to_string(&cli.path)?;
    let commands: Vec<CompileCommand> = serde_json::from_str(&content)?;
    let original_count = commands.len();

    // Create backup
    let backup_path = find_backup_path(&cli.path);
    fs::copy(&cli.path, &backup_path)?;
    eprintln!("Backup created: {}", backup_path.display());

    // Filter entries
    let filtered = filter_commands(commands, &exclude_patterns, &include_patterns);
    let filtered_count = filtered.len();

    // Write filtered result
    let output = serde_json::to_string_pretty(&filtered)?;
    fs::write(&cli.path, output)?;

    // Print statistics
    eprintln!(
        "Filtered: {} -> {} entries ({} removed)",
        original_count,
        filtered_count,
        original_count - filtered_count
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_cmd(file: &str) -> CompileCommand {
        CompileCommand {
            command: format!("gcc -c {}", file),
            directory: "/build".to_string(),
            file: file.to_string(),
        }
    }

    // Tests for compile_patterns
    mod compile_patterns_tests {
        use super::*;

        #[test]
        fn compiles_valid_patterns() {
            let patterns = vec!["^src/".to_string(), r"\.c$".to_string()];
            let result = compile_patterns(&patterns);
            assert!(result.is_ok());
            assert_eq!(result.unwrap().len(), 2);
        }

        #[test]
        fn compiles_empty_patterns() {
            let patterns: Vec<String> = vec![];
            let result = compile_patterns(&patterns);
            assert!(result.is_ok());
            assert!(result.unwrap().is_empty());
        }

        #[test]
        fn returns_error_for_invalid_regex() {
            let patterns = vec!["[invalid".to_string()];
            let result = compile_patterns(&patterns);
            assert!(result.is_err());
        }

        #[test]
        fn returns_error_for_any_invalid_in_list() {
            let patterns = vec!["valid".to_string(), "[invalid".to_string(), "also_valid".to_string()];
            let result = compile_patterns(&patterns);
            assert!(result.is_err());
        }

        #[test]
        fn compiles_complex_patterns() {
            let patterns = vec![
                r"^arch/(arm|x86)/".to_string(),
                r"drivers/.*\.c$".to_string(),
                r"\btest\b".to_string(),
            ];
            let result = compile_patterns(&patterns);
            assert!(result.is_ok());
        }
    }

    // Tests for filter_commands
    mod filter_commands_tests {
        use super::*;

        #[test]
        fn returns_all_when_no_patterns() {
            let commands = vec![make_cmd("a.c"), make_cmd("b.c"), make_cmd("c.c")];
            let result = filter_commands(commands.clone(), &[], &[]);
            assert_eq!(result.len(), 3);
        }

        #[test]
        fn excludes_matching_files() {
            let commands = vec![
                make_cmd("src/main.c"),
                make_cmd("tests/test.c"),
                make_cmd("src/util.c"),
            ];
            let exclude = vec![Regex::new("^tests/").unwrap()];
            let result = filter_commands(commands, &exclude, &[]);
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].file, "src/main.c");
            assert_eq!(result[1].file, "src/util.c");
        }

        #[test]
        fn excludes_with_multiple_patterns() {
            let commands = vec![
                make_cmd("src/main.c"),
                make_cmd("tests/test.c"),
                make_cmd("vendor/lib.c"),
            ];
            let exclude = vec![
                Regex::new("^tests/").unwrap(),
                Regex::new("^vendor/").unwrap(),
            ];
            let result = filter_commands(commands, &exclude, &[]);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].file, "src/main.c");
        }

        #[test]
        fn include_overrides_exclude() {
            let commands = vec![
                make_cmd("tests/unit.c"),
                make_cmd("tests/integration.c"),
                make_cmd("src/main.c"),
            ];
            let exclude = vec![Regex::new("^tests/").unwrap()];
            let include = vec![Regex::new("integration").unwrap()];
            let result = filter_commands(commands, &exclude, &include);
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].file, "tests/integration.c");
            assert_eq!(result[1].file, "src/main.c");
        }

        #[test]
        fn include_without_exclude_keeps_all() {
            let commands = vec![make_cmd("a.c"), make_cmd("b.c")];
            let include = vec![Regex::new("a").unwrap()];
            let result = filter_commands(commands.clone(), &[], &include);
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn multiple_includes_work() {
            let commands = vec![
                make_cmd("tests/unit.c"),
                make_cmd("tests/integration.c"),
                make_cmd("tests/e2e.c"),
                make_cmd("src/main.c"),
            ];
            let exclude = vec![Regex::new("^tests/").unwrap()];
            let include = vec![
                Regex::new("integration").unwrap(),
                Regex::new("e2e").unwrap(),
            ];
            let result = filter_commands(commands, &exclude, &include);
            assert_eq!(result.len(), 3);
        }

        #[test]
        fn handles_empty_commands() {
            let commands: Vec<CompileCommand> = vec![];
            let exclude = vec![Regex::new(".*").unwrap()];
            let result = filter_commands(commands, &exclude, &[]);
            assert!(result.is_empty());
        }

        #[test]
        fn excludes_all_with_wildcard() {
            let commands = vec![make_cmd("a.c"), make_cmd("b.c"), make_cmd("c.c")];
            let exclude = vec![Regex::new(".*").unwrap()];
            let result = filter_commands(commands, &exclude, &[]);
            assert!(result.is_empty());
        }

        #[test]
        fn include_can_restore_all_excluded() {
            let commands = vec![make_cmd("a.c"), make_cmd("b.c")];
            let exclude = vec![Regex::new(".*").unwrap()];
            let include = vec![Regex::new(".*").unwrap()];
            let result = filter_commands(commands.clone(), &exclude, &include);
            assert_eq!(result.len(), 2);
        }

        #[test]
        fn preserves_order() {
            let commands = vec![
                make_cmd("z.c"),
                make_cmd("a.c"),
                make_cmd("m.c"),
            ];
            let result = filter_commands(commands.clone(), &[], &[]);
            assert_eq!(result[0].file, "z.c");
            assert_eq!(result[1].file, "a.c");
            assert_eq!(result[2].file, "m.c");
        }

        #[test]
        fn partial_path_match() {
            let commands = vec![
                make_cmd("kernel/drivers/usb.c"),
                make_cmd("kernel/drivers/pci.c"),
                make_cmd("kernel/init.c"),
            ];
            let exclude = vec![Regex::new("drivers/").unwrap()];
            let result = filter_commands(commands, &exclude, &[]);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].file, "kernel/init.c");
        }

        #[test]
        fn case_sensitive_matching() {
            let commands = vec![
                make_cmd("src/Main.c"),
                make_cmd("src/main.c"),
            ];
            let exclude = vec![Regex::new("Main").unwrap()];
            let result = filter_commands(commands, &exclude, &[]);
            assert_eq!(result.len(), 1);
            assert_eq!(result[0].file, "src/main.c");
        }

        #[test]
        fn complex_regex_patterns() {
            let commands = vec![
                make_cmd("arch/x86/boot.c"),
                make_cmd("arch/arm/boot.c"),
                make_cmd("arch/arm64/boot.c"),
                make_cmd("kernel/main.c"),
            ];
            let exclude = vec![Regex::new(r"^arch/(arm|arm64)/").unwrap()];
            let result = filter_commands(commands, &exclude, &[]);
            assert_eq!(result.len(), 2);
            assert_eq!(result[0].file, "arch/x86/boot.c");
            assert_eq!(result[1].file, "kernel/main.c");
        }
    }

    // Tests for find_backup_path
    mod find_backup_path_tests {
        use super::*;

        #[test]
        fn returns_bak_when_no_backup_exists() {
            let temp_dir = TempDir::new().unwrap();
            let original = temp_dir.path().join("file.json");
            fs::write(&original, "content").unwrap();

            let backup = find_backup_path(&original);
            assert_eq!(backup, temp_dir.path().join("file.json.bak"));
        }

        #[test]
        fn returns_bak_1_when_bak_exists() {
            let temp_dir = TempDir::new().unwrap();
            let original = temp_dir.path().join("file.json");
            fs::write(&original, "content").unwrap();
            fs::write(temp_dir.path().join("file.json.bak"), "backup").unwrap();

            let backup = find_backup_path(&original);
            assert_eq!(backup, temp_dir.path().join("file.json.bak.1"));
        }

        #[test]
        fn returns_bak_2_when_bak_and_bak_1_exist() {
            let temp_dir = TempDir::new().unwrap();
            let original = temp_dir.path().join("file.json");
            fs::write(&original, "content").unwrap();
            fs::write(temp_dir.path().join("file.json.bak"), "backup").unwrap();
            fs::write(temp_dir.path().join("file.json.bak.1"), "backup1").unwrap();

            let backup = find_backup_path(&original);
            assert_eq!(backup, temp_dir.path().join("file.json.bak.2"));
        }

        #[test]
        fn skips_gaps_in_numbering() {
            let temp_dir = TempDir::new().unwrap();
            let original = temp_dir.path().join("file.json");
            fs::write(&original, "content").unwrap();
            fs::write(temp_dir.path().join("file.json.bak"), "backup").unwrap();
            // Skip .bak.1
            fs::write(temp_dir.path().join("file.json.bak.2"), "backup2").unwrap();

            let backup = find_backup_path(&original);
            // Should return .bak.1 since it's the first non-existing one
            assert_eq!(backup, temp_dir.path().join("file.json.bak.1"));
        }

        #[test]
        fn handles_long_filename() {
            let temp_dir = TempDir::new().unwrap();
            let original = temp_dir.path().join("very_long_filename_compile_commands.json");
            fs::write(&original, "content").unwrap();

            let backup = find_backup_path(&original);
            assert!(backup.to_str().unwrap().ends_with(".bak"));
        }

        #[test]
        fn handles_path_with_special_chars() {
            let temp_dir = TempDir::new().unwrap();
            let original = temp_dir.path().join("file-with-dashes.json");
            fs::write(&original, "content").unwrap();

            let backup = find_backup_path(&original);
            assert_eq!(backup, temp_dir.path().join("file-with-dashes.json.bak"));
        }

        #[test]
        fn handles_deeply_nested_path() {
            let temp_dir = TempDir::new().unwrap();
            let nested = temp_dir.path().join("a/b/c");
            fs::create_dir_all(&nested).unwrap();
            let original = nested.join("file.json");
            fs::write(&original, "content").unwrap();

            let backup = find_backup_path(&original);
            assert_eq!(backup, nested.join("file.json.bak"));
        }

        #[test]
        fn many_backups_increment_correctly() {
            let temp_dir = TempDir::new().unwrap();
            let original = temp_dir.path().join("file.json");
            fs::write(&original, "content").unwrap();
            fs::write(temp_dir.path().join("file.json.bak"), "backup").unwrap();
            for i in 1..=10 {
                fs::write(temp_dir.path().join(format!("file.json.bak.{}", i)), "backup").unwrap();
            }

            let backup = find_backup_path(&original);
            assert_eq!(backup, temp_dir.path().join("file.json.bak.11"));
        }
    }

    // Tests for CompileCommand serialization
    mod serialization_tests {
        use super::*;

        #[test]
        fn serializes_compile_command() {
            let cmd = make_cmd("test.c");
            let json = serde_json::to_string(&cmd).unwrap();
            assert!(json.contains("\"file\":\"test.c\""));
            assert!(json.contains("\"directory\":\"/build\""));
            assert!(json.contains("\"command\":\"gcc -c test.c\""));
        }

        #[test]
        fn deserializes_compile_command() {
            let json = r#"{"command":"gcc -c foo.c","directory":"/home/build","file":"foo.c"}"#;
            let cmd: CompileCommand = serde_json::from_str(json).unwrap();
            assert_eq!(cmd.file, "foo.c");
            assert_eq!(cmd.directory, "/home/build");
            assert_eq!(cmd.command, "gcc -c foo.c");
        }

        #[test]
        fn roundtrip_serialization() {
            let cmd = make_cmd("path/to/file.c");
            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: CompileCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, deserialized);
        }

        #[test]
        fn deserializes_array_of_commands() {
            let json = r#"[
                {"command":"gcc -c a.c","directory":"/build","file":"a.c"},
                {"command":"gcc -c b.c","directory":"/build","file":"b.c"}
            ]"#;
            let commands: Vec<CompileCommand> = serde_json::from_str(json).unwrap();
            assert_eq!(commands.len(), 2);
            assert_eq!(commands[0].file, "a.c");
            assert_eq!(commands[1].file, "b.c");
        }

        #[test]
        fn handles_unicode_in_paths() {
            let cmd = CompileCommand {
                command: "gcc -c файл.c".to_string(),
                directory: "/сборка".to_string(),
                file: "файл.c".to_string(),
            };
            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: CompileCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, deserialized);
        }

        #[test]
        fn handles_special_chars_in_command() {
            let cmd = CompileCommand {
                command: r#"gcc -DVERSION=\"1.0\" -c file.c"#.to_string(),
                directory: "/build".to_string(),
                file: "file.c".to_string(),
            };
            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: CompileCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, deserialized);
        }
    }
}
