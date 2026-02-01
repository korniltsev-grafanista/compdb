pub mod wrapper;
pub mod generate;

use std::env;

pub fn run_cc() {
    let compiler = env::var("COMPDB_CC").unwrap_or_else(|_| "clang".to_string());
    run_with_compiler(&compiler);
}

pub fn run_cxx() {
    let compiler = env::var("COMPDB_CXX").unwrap_or_else(|_| "clang++".to_string());
    run_with_compiler(&compiler);
}

/// Determine the compiler to use for C compilation.
/// Uses COMPDB_CC environment variable if set, otherwise defaults to "clang".
pub fn get_cc_compiler() -> String {
    env::var("COMPDB_CC").unwrap_or_else(|_| "clang".to_string())
}

/// Determine the compiler to use for C++ compilation.
/// Uses COMPDB_CXX environment variable if set, otherwise defaults to "clang++".
pub fn get_cxx_compiler() -> String {
    env::var("COMPDB_CXX").unwrap_or_else(|_| "clang++".to_string())
}

/// Determine the log file path.
/// Requires COMPDB_LOG environment variable to be set.
pub fn get_log_file() -> Result<String, env::VarError> {
    env::var("COMPDB_LOG")
}

/// Check if generate mode is requested via --generate flag in args.
pub fn has_generate_flag(args: &[String]) -> bool {
    args.iter().any(|a| a == "--generate")
}

/// Check if generate mode is requested via COMPDB_GENERATE environment variable.
pub fn has_generate_env() -> bool {
    env::var("COMPDB_GENERATE")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

/// Determine if generate mode should be used.
pub fn should_generate(args: &[String]) -> bool {
    has_generate_flag(args) || has_generate_env()
}

fn run_with_compiler(compiler: &str) {
    let args: Vec<String> = env::args().collect();
    let log_file = match get_log_file() {
        Ok(path) => path,
        Err(_) => {
            eprintln!("Error: COMPDB_LOG environment variable is required");
            std::process::exit(1);
        }
    };

    if should_generate(&args) {
        if let Err(e) = generate::run(&log_file) {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    } else {
        wrapper::run(&log_file, compiler);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== get_cc_compiler tests ====================

    mod get_cc_compiler_tests {
        use super::*;
        use std::sync::Mutex;

        // Mutex to prevent parallel test interference with env vars
        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        #[test]
        fn returns_clang_by_default() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::remove_var("COMPDB_CC");
            assert_eq!(get_cc_compiler(), "clang");
        }

        #[test]
        fn returns_custom_compiler_from_env() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_CC", "gcc");
            let result = get_cc_compiler();
            env::remove_var("COMPDB_CC");
            assert_eq!(result, "gcc");
        }

        #[test]
        fn returns_full_path_compiler_from_env() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_CC", "/usr/local/bin/gcc-12");
            let result = get_cc_compiler();
            env::remove_var("COMPDB_CC");
            assert_eq!(result, "/usr/local/bin/gcc-12");
        }
    }

    // ==================== get_cxx_compiler tests ====================

    mod get_cxx_compiler_tests {
        use super::*;
        use std::sync::Mutex;

        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        #[test]
        fn returns_clangpp_by_default() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::remove_var("COMPDB_CXX");
            assert_eq!(get_cxx_compiler(), "clang++");
        }

        #[test]
        fn returns_custom_compiler_from_env() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_CXX", "g++");
            let result = get_cxx_compiler();
            env::remove_var("COMPDB_CXX");
            assert_eq!(result, "g++");
        }

        #[test]
        fn returns_full_path_compiler_from_env() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_CXX", "/usr/local/bin/g++-12");
            let result = get_cxx_compiler();
            env::remove_var("COMPDB_CXX");
            assert_eq!(result, "/usr/local/bin/g++-12");
        }
    }

    // ==================== get_log_file tests ====================

    mod get_log_file_tests {
        use super::*;
        use std::sync::Mutex;

        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        #[test]
        fn returns_error_when_env_not_set() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::remove_var("COMPDB_LOG");
            assert!(get_log_file().is_err());
        }

        #[test]
        fn returns_custom_path_from_env() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_LOG", "custom_log.txt");
            let result = get_log_file();
            env::remove_var("COMPDB_LOG");
            assert_eq!(result.unwrap(), "custom_log.txt");
        }

        #[test]
        fn returns_absolute_path_from_env() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_LOG", "/tmp/build/compile_log.txt");
            let result = get_log_file();
            env::remove_var("COMPDB_LOG");
            assert_eq!(result.unwrap(), "/tmp/build/compile_log.txt");
        }

        #[test]
        fn returns_relative_path_from_env() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_LOG", "../logs/cc_hook.txt");
            let result = get_log_file();
            env::remove_var("COMPDB_LOG");
            assert_eq!(result.unwrap(), "../logs/cc_hook.txt");
        }
    }

    // ==================== has_generate_flag tests ====================

    mod has_generate_flag_tests {
        use super::*;

        #[test]
        fn returns_true_when_flag_present() {
            let args = vec![
                "compdb-cc".to_string(),
                "--generate".to_string(),
            ];
            assert!(has_generate_flag(&args));
        }

        #[test]
        fn returns_false_when_flag_absent() {
            let args = vec![
                "compdb-cc".to_string(),
                "-c".to_string(),
                "main.c".to_string(),
            ];
            assert!(!has_generate_flag(&args));
        }

        #[test]
        fn returns_true_when_flag_at_end() {
            let args = vec![
                "compdb-cc".to_string(),
                "-c".to_string(),
                "main.c".to_string(),
                "--generate".to_string(),
            ];
            assert!(has_generate_flag(&args));
        }

        #[test]
        fn returns_false_for_empty_args() {
            let args: Vec<String> = vec![];
            assert!(!has_generate_flag(&args));
        }

        #[test]
        fn returns_false_for_similar_flags() {
            let args = vec![
                "compdb-cc".to_string(),
                "--generated".to_string(),
                "-generate".to_string(),
                "generate".to_string(),
            ];
            assert!(!has_generate_flag(&args));
        }
    }

    // ==================== has_generate_env tests ====================

    mod has_generate_env_tests {
        use super::*;
        use std::sync::Mutex;

        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        #[test]
        fn returns_false_when_env_not_set() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::remove_var("COMPDB_GENERATE");
            assert!(!has_generate_env());
        }

        #[test]
        fn returns_true_when_env_is_set() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_GENERATE", "1");
            let result = has_generate_env();
            env::remove_var("COMPDB_GENERATE");
            assert!(result);
        }

        #[test]
        fn returns_true_when_env_is_true() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_GENERATE", "true");
            let result = has_generate_env();
            env::remove_var("COMPDB_GENERATE");
            assert!(result);
        }

        #[test]
        fn returns_false_when_env_is_empty() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_GENERATE", "");
            let result = has_generate_env();
            env::remove_var("COMPDB_GENERATE");
            assert!(!result);
        }

        #[test]
        fn returns_true_for_any_non_empty_value() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_GENERATE", "yes");
            let result = has_generate_env();
            env::remove_var("COMPDB_GENERATE");
            assert!(result);
        }
    }

    // ==================== should_generate tests ====================

    mod should_generate_tests {
        use super::*;
        use std::sync::Mutex;

        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        #[test]
        fn returns_true_when_flag_present() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::remove_var("COMPDB_GENERATE");
            let args = vec!["compdb-cc".to_string(), "--generate".to_string()];
            assert!(should_generate(&args));
        }

        #[test]
        fn returns_true_when_env_set() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_GENERATE", "1");
            let args = vec!["compdb-cc".to_string(), "-c".to_string()];
            let result = should_generate(&args);
            env::remove_var("COMPDB_GENERATE");
            assert!(result);
        }

        #[test]
        fn returns_true_when_both_flag_and_env() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::set_var("COMPDB_GENERATE", "1");
            let args = vec!["compdb-cc".to_string(), "--generate".to_string()];
            let result = should_generate(&args);
            env::remove_var("COMPDB_GENERATE");
            assert!(result);
        }

        #[test]
        fn returns_false_when_neither() {
            let _guard = ENV_MUTEX.lock().unwrap();
            env::remove_var("COMPDB_GENERATE");
            let args = vec!["compdb-cc".to_string(), "-c".to_string(), "main.c".to_string()];
            assert!(!should_generate(&args));
        }
    }
}
