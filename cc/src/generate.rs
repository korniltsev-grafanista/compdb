use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use serde_json::{json, Value};

pub fn run(log_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Output file
    let dst = "compile_commands.json";

    // Read log file
    let file = File::open(log_file)?;
    let reader = BufReader::new(file);

    // Parse each line and build compilation database
    let mut db = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let it: Value = serde_json::from_str(&line)?;

        let wd = it["wd"].as_str().unwrap_or("");
        let args_value = &it["args"];

        if !args_value.is_array() {
            continue;
        }

        let mut args = vec!["/usr/bin/gcc".to_string()];
        for arg in args_value.as_array().unwrap() {
            if let Some(arg_str) = arg.as_str() {
                args.push(arg_str.to_string());
            }
        }

        // Find source files in arguments
        let mut srcs = Vec::new();
        for arg in &args {
            if arg.ends_with(".c") || arg.ends_with(".cc") || arg.ends_with(".cpp") {
                srcs.push(Path::new(wd).join(arg).to_string_lossy().to_string());
            }
        }

        // Add entry to compilation database if source files were found
        if !srcs.is_empty() {
            db.push(json!({
                "directory": wd,
                "arguments": args,
                "file": srcs.last().unwrap(),
            }));
        } else {
            eprintln!("warning no src {}", line);
        }
    }

    // Write compilation database to file
    let mut output = File::create(dst)?;
    output.write_all(serde_json::to_string_pretty(&db)?.as_bytes())?;

    Ok(())
}
