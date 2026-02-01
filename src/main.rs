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
