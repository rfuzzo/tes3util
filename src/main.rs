use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tes3util::{
    atlas_task::atlas_coverage, deserialize_task::deserialize_plugin, dump_task::dump,
    pack_task::pack, serialize_task::serialize_plugin, sql_task, ESerializedType,
};

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

        /// The extension to serialize to, default is yaml
        #[arg(short, long, value_enum)]
        format: Option<ESerializedType>,

        /// Create folder with plugin name, only available if input is a file
        #[arg(short, long)]
        create: bool,

        /// Include specific records
        #[arg(short, long)]
        include: Vec<String>,

        /// Exclude specific records
        #[arg(short, long)]
        exclude: Vec<String>,
    },

    /// Packs records from a folder into a plugin
    Pack {
        /// input path, may be a folder
        input: Option<PathBuf>,

        /// output path, may be a plugin
        output: Option<PathBuf>,

        /// The extension to serialize from, default is yaml
        #[arg(short, long, value_enum)]
        format: Option<ESerializedType>,
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
        format: Option<ESerializedType>,
    },

    /// Deserialize a text file from a human-readable format to a plugin
    Deserialize {
        /// input path, may be a file or a folder
        input: Option<PathBuf>,

        /// output file name, defaults to cwd
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite existing plugin
        #[arg(short = 'y', long)]
        overwrite: bool,
    },

    /// Atlas coverage of all meshes
    AtlasCoverage {
        /// input path, may be a folder, defaults to cwd
        input: Option<PathBuf>,

        /// output directory, defaults to cwd
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Sql
    Sql {
        /// input path, may be a folder, defaults to cwd
        input: Option<PathBuf>,

        /// output directory, defaults to cwd
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    // logger
    tes3util::init_logger(Path::new("log.txt")).expect("Could not initialize logger");

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
        Commands::Pack {
            input,
            output,
            format,
        } => match pack(input, output, format) {
            Ok(_) => println!("Done."),
            Err(err) => println!("Error packing plugin: {}", err),
        },
        Commands::Serialize {
            input,
            output,
            format,
        } => match serialize_plugin(input, output, format) {
            Ok(_) => println!("Done."),
            Err(err) => println!("Error serializing plugin: {}", err),
        },
        Commands::Deserialize {
            input,
            output,
            overwrite,
        } => match deserialize_plugin(input, output, *overwrite) {
            Ok(_) => println!("Done."),
            Err(err) => println!("Error deserializing file: {}", err),
        },
        Commands::AtlasCoverage { input, output } => match atlas_coverage(input, output) {
            Ok(_) => println!("Done."),
            Err(err) => println!("Error running atlas coverage: {}", err),
        },
        Commands::Sql { input, output } => match sql_task::sql_task(input, output) {
            Ok(_) => println!("Done."),
            Err(err) => println!("Error running sql command: {}", err),
        },
    }
}
