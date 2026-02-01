# compdbfilter

Filter `compile_commands.json` by regex patterns.

## Installation

```bash
cargo install --git https://github.com/korniltsev-grafanista/compdb.git compdbfilter
```

Or from a local clone:

```bash
git clone https://github.com/korniltsev-grafanista/compdb.git
cd compdb
cargo install --path compdbfilter
```

## Usage

```bash
compdbfilter [OPTIONS] [PATH]
```

### Arguments

- `PATH` - Path to compile_commands.json (default: `./compile_commands.json`)

### Options

- `-e, --exclude <REGEX>` - Exclude files matching this regex (can be repeated)
- `-i, --include <REGEX>` - Include files matching this regex even if excluded (can be repeated)

### Examples

Exclude all test files:

```bash
compdbfilter -e '^tests/'
```

Exclude tests and vendor, but keep integration tests:

```bash
compdbfilter -e '^tests/' -e '^vendor/' -i 'integration'
```

Filter a specific file:

```bash
compdbfilter /path/to/compile_commands.json -e 'drivers/'
```

The tool automatically creates a backup (`.bak`, `.bak.1`, etc.) before modifying the file.
