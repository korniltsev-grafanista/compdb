mod wrapper;
mod generate;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for --generate flag
    let generate_flag = args.iter().any(|a| a == "--generate");

    // Check for environment variable
    let generate_env = env::var("CC_HOOK_COMPDB_GENERATE")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    if generate_flag || generate_env {
        // Extract log file path from args after --generate, or use env/default
        let log_file = args.iter()
            .skip_while(|a| *a != "--generate")
            .nth(1)
            .cloned()
            .or_else(|| env::var("CC_HOOK_COMPDB_LOG_FILE").ok())
            .unwrap_or_else(|| "cc_hook.txt".to_string());

        if let Err(e) = generate::run(&log_file) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        wrapper::run();
    }
}
