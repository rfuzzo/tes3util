use clap::{Parser, Subcommand};
use mwscript::{deserialize_plugin, dump, serialize_plugin, ESerializedType};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version)]
#[command(about = "A commandline tool for modding TES3 - Morrowind", long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Dump records from a plugin
    Dump {
        /// input path, may be a plugin or a folder
        input: Option<PathBuf>,

        /// output directory to dump records to, defaults to cwd
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Create folder with plugin name, only available if input is a file
        #[arg(short, long)]
        create: bool,

        /// Include specific records
        #[arg(short, long)]
        include: Vec<String>,

        /// Exclude specific records
        #[arg(short, long)]
        exclude: Vec<String>,

        /// The extension to serialize to, default is yaml
        #[arg(short, long, value_enum)]
        format: ESerializedType,
    },

    /// Serialize a plugin to a human-readable format
    Serialize {
        /// input path, may be a plugin or a folder
        input: Option<PathBuf>,

        /// output directory, defaults to cwd
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// The extension to serialize to, default is yaml
        #[arg(short, long, value_enum)]
        format: ESerializedType,
    },

    /// Deserialize a text file from a human-readable format to a plugin
    Deserialize {
        /// input path, may be a file or a folder
        input: Option<PathBuf>,

        /// output directory, defaults to cwd
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    match &Cli::parse().commands {
        Commands::Dump {
            input,
            output,
            create,
            include,
            exclude,
            format,
        } => match dump(input, output, *create, include, exclude, format) {
            Ok(_) => println!("Done."),
            Err(err) => println!("Error dumping scripts: {}", err),
        },
        Commands::Serialize {
            input,
            output,
            format,
        } => match serialize_plugin(input, output, format) {
            Ok(_) => println!("Done."),
            Err(err) => println!("Error serializing plugin: {}", err),
        },
        Commands::Deserialize { input, output } => match deserialize_plugin(input, output) {
            Ok(_) => println!("Done."),
            Err(err) => println!("Error deserializing file: {}", err),
        },
    }
}
