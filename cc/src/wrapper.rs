use std::env;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use serde_json::json;
use fs2::FileExt;

pub fn run(log_file: &str, compiler: &str) {
    let log_path = Path::new(&log_file);
    if !log_path.is_absolute() {
        eprintln!("Error: log file path must be absolute: {}", log_file);
        std::process::exit(1);
    }

    // Create lock file path next to the log file
    let lock_file_path = log_path.with_extension("lock");

    // Get current working directory
    let wd = env::current_dir().expect("Failed to get current directory");
    let wd_str = wd.to_string_lossy().to_string();

    // Get command line arguments (excluding the program name)
    let args: Vec<String> = env::args().skip(1).collect();

    // Log the command execution
    let log_entry = json!({
        "wd": wd_str,
        "compiler": compiler,
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
        .open(log_file)
        .expect("Failed to open log file");

    // Write log entry
    writeln!(file, "{}", log_entry)
        .expect("Failed to write to log file");

    // Release the lock
    lock_file.unlock()
        .expect("Failed to release lock");

    // Execute the compiler with the provided arguments
    let mut cmd = Command::new(compiler);
    cmd.args(&args);

    // Replace current process with the compiler
    let error = cmd.exec();

    // If exec returns, it means there was an error
    eprintln!("Failed to execute {}: {}", compiler, error);
    std::process::exit(1);
}
