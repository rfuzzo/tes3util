# tes3util

A command-line tool for modding The Elder Scrolls III: Morrowind (TES3). This utility provides various commands for working with ESP/ESM plugin files, extracting and manipulating game data, and performing analysis on Morrowind mod files.

## Features

- **Dump**: Extract records from plugin files to human-readable formats
- **Pack**: Combine extracted records back into plugin files  
- **Serialize/Deserialize**: Convert between binary plugin formats and text formats (YAML, TOML, JSON)
- **Atlas Coverage**: Analyze mesh atlas coverage for optimization
- **SQL Database**: Export plugin data to SQLite databases for analysis

## Usage

```bash
tes3util <COMMAND>

Commands:
  dump            Dump records from a plugin to human-readable format
  pack            Pack records from a folder into a plugin file
  serialize       Serialize a plugin to a human-readable format  
  deserialize     Deserialize a text file back to a plugin format
  atlas-coverage  Generate atlas coverage analysis of meshes
  sql             Export plugin data to SQLite database
  help            Print help information

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Command Examples

#### Dump Plugin Records
```bash
# Dump all records from a plugin to YAML files
tes3util dump "MyMod.esp" -o ./output -f yaml

# Dump only specific record types
tes3util dump "MyMod.esp" -i CELL -i NPC_ -o ./output

# Exclude certain record types
tes3util dump "MyMod.esp" -e DIAL -e INFO -o ./output
```

#### Pack Records into Plugin
```bash
# Pack YAML files back into a plugin
tes3util pack ./input_folder -o "NewMod.esp" -f yaml
```

#### Serialize/Deserialize
```bash
# Convert plugin to YAML
tes3util serialize "MyMod.esp" -f yaml -o ./output

# Convert YAML back to plugin
tes3util deserialize "MyMod.esp.yaml" -o "MyMod_new.esp"

# Supported formats: yaml, toml, json
tes3util serialize "MyMod.esp" -f json -o ./output
```

#### Atlas Coverage Analysis
```bash
# Analyze mesh atlas coverage
tes3util atlas-coverage ./meshes_folder -o ./analysis_output
```

#### Export to Database
```bash
# Export plugin data to SQLite database
tes3util sql ./plugins_folder -o ./database_output
```

## Installation

### Prerequisites
- Rust (1.70+ recommended)
- Cargo package manager

### Building from Source
```bash
# Clone the repository
git clone <repository-url>
cd tes3util

# Build the project
cargo build --release

# The executable will be in target/release/tes3util
```

### Running Tests
```bash
# Run unit tests
cargo test

# Run integration tests (requires test assets)
cargo test --test integration_tests -- --ignored
```

## Supported Formats

The tool supports conversion between binary ESP/ESM files and the following text formats:
- **YAML** (default) - Human-readable, good for version control
- **TOML** - Configuration-friendly format
- **JSON** - Machine-readable, good for scripting

## Dependencies

- **clap** - Command-line argument parsing
- **serde** - Serialization framework
- **rusqlite** - SQLite database support
- **walkdir** - Directory traversal
- **rayon** - Parallel processing
- **tes3** - Local TES3 file format library

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## Acknowledgments

This tool is designed for The Elder Scrolls III: Morrowind modding community. It helps with plugin analysis, conversion, and batch processing of mod files.
