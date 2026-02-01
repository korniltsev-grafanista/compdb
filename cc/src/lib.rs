pub mod wrapper;
pub mod generate;

use std::env;

const LOG_FILE: &str = "cc_hook.txt";

pub fn run_cc() {
    let compiler = env::var("COMPDB_CC").unwrap_or_else(|_| "clang".to_string());
    run_with_compiler(&compiler);
}

pub fn run_cxx() {
    let compiler = env::var("COMPDB_CXX").unwrap_or_else(|_| "clang++".to_string());
    run_with_compiler(&compiler);
}

fn run_with_compiler(compiler: &str) {
    let args: Vec<String> = env::args().collect();

    // Check for --generate flag
    let generate_flag = args.iter().any(|a| a == "--generate");

    // Check for environment variable
    let generate_env = env::var("COMPDB_GENERATE")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    if generate_flag || generate_env {
        if let Err(e) = generate::run(LOG_FILE) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        wrapper::run(LOG_FILE, compiler);
    }
}
