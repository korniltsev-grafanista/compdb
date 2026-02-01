use std::env;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use serde_json::json;
use fs2::FileExt; // Add fs2 crate for file locking

fn main() {
    // Get log file path from environment variable or use default
    let log_file = env::var("CC_HOOK_COMPDB_LOG_FILE").unwrap_or_else(|_| "cc_hook.txt".to_string());
    
    // Get compiler path from environment variable or use default
    let gcc_path = env::var("CC_HOOK_COMPDB_CC").unwrap_or_else(|_| "/usr/bin/gcc".to_string());
    
    // Create lock file path next to the log file
    let log_path = Path::new(&log_file);
    let lock_file_path = log_path.with_extension("lock");
    
    // Get current working directory
    let wd = env::current_dir().expect("Failed to get current directory");
    let wd_str = wd.to_string_lossy().to_string();
    
    // Get command line arguments (excluding the program name)
    let args: Vec<String> = env::args().skip(1).collect();
    
    // Log the command execution
    let log_entry = json!({
        "wd": wd_str,
        "args": args,
    });
    
    // Create or open the lock file
    let lock_file = File::create(&lock_file_path)
        .expect("Failed to create lock file");
    
    // Acquire an exclusive lock
    lock_file.lock_exclusive()
        .expect("Failed to acquire lock");
    
    // Open log file in append mode
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .expect("Failed to open log file");
    
    // Write log entry
    writeln!(file, "{}", log_entry.to_string())
        .expect("Failed to write to log file");
    
    // Release the lock
    lock_file.unlock()
        .expect("Failed to release lock");
    
    // Execute gcc with the provided arguments
    let mut cmd = Command::new(&gcc_path);
    cmd.args(&args);
    
    // Replace current process with gcc
    let error = cmd.exec();
    
    // If exec returns, it means there was an error
    eprintln!("Failed to execute {}: {}", gcc_path, error);
    std::process::exit(1);
} 