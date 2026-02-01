# compdb

Tools for working with `compile_commands.json`.

## Installation

```bash
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --git https://github.com/korniltsev-grafanista/compdb.git filter
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --git https://github.com/korniltsev-grafanista/compdb.git cc
```

This installs three binaries:
- `compdb-filter` - Filter compile_commands.json by regex patterns
- `compdb-cc` - C compiler wrapper for generating compile_commands.json
- `compdb-cxx` - C++ compiler wrapper for generating compile_commands.json

## compdb-filter

Filter `compile_commands.json` by regex patterns.

### Usage

```bash
compdb-filter [OPTIONS] [PATH]
```

### Arguments

- `PATH` - Path to compile_commands.json (default: `./compile_commands.json`)

### Options

- `-e, --exclude <REGEX>` - Exclude files matching this regex (can be repeated)
- `-i, --include <REGEX>` - Include files matching this regex even if excluded (can be repeated)

## compdb-cc / compdb-cxx

Compiler wrappers that log compilation commands for generating `compile_commands.json`.

### How It Works

1. `compdb-cc` and `compdb-cxx` act as drop-in replacements for your C/C++ compiler
2. They log each compilation command to a file
3. After the build completes, run with `--generate` to create `compile_commands.json`

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `COMPDB_LOG` | Yes | Absolute path to the log file (e.g., `/tmp/compdb.log`) |
| `COMPDB_CC` | No | C compiler to use (default: `clang`) |
| `COMPDB_CXX` | No | C++ compiler to use (default: `clang++`) |
| `COMPDB_GENERATE` | No | Set to any non-empty value to generate `compile_commands.json` |

### Usage

```bash
# Set up environment
export COMPDB_LOG=/tmp/compdb.log
export COMPDB_CC=gcc
export COMPDB_CXX=g++

# Build your project using the wrappers as compilers
./configure CC=compdb-cc CXX=compdb-cxx
make

# Generate compile_commands.json
compdb-cc --generate
```
