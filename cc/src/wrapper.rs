use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;
use serde_json::json;
use fs2::FileExt;

/// Check if a command line represents a "configure" script execution.
/// Returns true if the first argument (the executable) ends with "/configure" or is exactly "configure".
fn is_configure_command(cmdline: &str) -> bool {
    // cmdline is null-separated, first element is the executable
    let exe = cmdline.split('\0').next().unwrap_or("");
    if exe.is_empty() {
        return false;
    }
    // Check if it's a configure script (either "./configure", "/path/to/configure", or just "configure")
    let path = Path::new(exe);
    path.file_name()
        .map(|name| name == "configure")
        .unwrap_or(false)
}

/// Get the parent PID of a given process by reading /proc/{pid}/stat.
fn get_parent_pid(pid: u32) -> Option<u32> {
    let stat_path = format!("/proc/{}/stat", pid);
    let content = fs::read_to_string(&stat_path).ok()?;

    // Format: pid (comm) state ppid ...
    // The comm field can contain spaces and parentheses, so we find the last ')' and parse from there
    let last_paren = content.rfind(')')?;
    let after_comm = &content[last_paren + 2..]; // Skip ") "
    let fields: Vec<&str> = after_comm.split_whitespace().collect();

    // First field after comm is state, second is ppid
    fields.get(1)?.parse().ok()
}

/// Get the command line of a process by reading /proc/{pid}/cmdline.
fn get_cmdline(pid: u32) -> Option<String> {
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    let mut file = File::open(&cmdline_path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;
    Some(content)
}

/// Check if any parent process in the process tree is running a "configure" script.
/// Traverses up the process tree until reaching init (PID 1) or finding a configure script.
fn has_configure_parent() -> bool {
    let mut pid = std::process::id();

    loop {
        let ppid = match get_parent_pid(pid) {
            Some(p) => p,
            None => return false,
        };

        // Stop at init
        if ppid == 0 || ppid == 1 {
            return false;
        }

        if let Some(cmdline) = get_cmdline(ppid) {
            if is_configure_command(&cmdline) {
                return true;
            }
        }

        pid = ppid;
    }
}

pub fn run(log_file: &str, compiler: &str) {
    let log_path = Path::new(&log_file);
    if !log_path.is_absolute() {
        eprintln!("Error: log file path must be absolute: {}", log_file);
        std::process::exit(1);
    }

    // Get command line arguments (excluding the program name)
    let args: Vec<String> = env::args().skip(1).collect();

    // Skip logging if we're running under a configure script
    if !has_configure_parent() {
        // Create lock file path next to the log file
        let lock_file_path = log_path.with_extension("lock");

        // Get current working directory
        let wd = env::current_dir().expect("Failed to get current directory");
        let wd_str = wd.to_string_lossy().to_string();

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
    }

    // Execute the compiler with the provided arguments
    let mut cmd = Command::new(compiler);
    cmd.args(&args);

    // Replace current process with the compiler
    let error = cmd.exec();

    // If exec returns, it means there was an error
    eprintln!("Failed to execute {}: {}", compiler, error);
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    mod is_configure_command_tests {
        use super::*;

        #[test]
        fn detects_configure_in_current_dir() {
            assert!(is_configure_command("./configure\0--prefix=/usr\0"));
        }

        #[test]
        fn detects_configure_with_absolute_path() {
            assert!(is_configure_command("/home/user/project/configure\0--enable-feature\0"));
        }

        #[test]
        fn detects_configure_without_args() {
            assert!(is_configure_command("./configure\0"));
        }

        #[test]
        fn detects_bare_configure() {
            assert!(is_configure_command("configure\0"));
        }

        #[test]
        fn rejects_empty_cmdline() {
            assert!(!is_configure_command(""));
        }

        #[test]
        fn rejects_non_configure_command() {
            assert!(!is_configure_command("/usr/bin/gcc\0-c\0main.c\0"));
        }

        #[test]
        fn rejects_configure_as_argument() {
            assert!(!is_configure_command("/bin/sh\0./configure\0"));
        }

        #[test]
        fn rejects_configure_substring() {
            assert!(!is_configure_command("./configure-cache\0"));
        }

        #[test]
        fn rejects_reconfigure() {
            assert!(!is_configure_command("./reconfigure\0"));
        }

        #[test]
        fn rejects_configure_prefix() {
            assert!(!is_configure_command("./configure.sh\0"));
        }
    }
}
