///////////////////////////////////////////////////////////////////////////
// Deserialize

use std::{
    fs,
    io::{self, Error, ErrorKind},
    path::PathBuf,
};

use tes3::esp::Plugin;

use crate::{append_ext, is_extension};

/// Deserialize a human-readable file to esp
pub fn deserialize_plugin(
    input: &Option<PathBuf>,
    output: &Option<PathBuf>,
    overwrite: bool,
) -> io::Result<()> {
    let input_path: &PathBuf;
    // check no input
    if let Some(i) = input {
        input_path = i;
    } else {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "No input path specified.",
        ));
    }
    // check input path exists and check if file or directory
    if !input_path.exists() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Input path does not exist",
        ));
    } else if !input_path.is_file() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Input path is not a file",
        ));
    } else if !(is_extension(input_path, "json")
        || is_extension(input_path, "toml")
        || is_extension(input_path, "yaml"))
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Input path is not a valid file",
        ));
    }

    let mut output_path = PathBuf::from(input_path.clone().to_str().unwrap());
    if overwrite {
        if let Some(path_str) = input_path.to_str() {
            let path_str = path_str.to_owned().to_lowercase();
            if let Some(stem) = path_str.strip_suffix(".esp.yaml") {
                output_path = PathBuf::from(stem.to_string()).with_extension("esp");
            } else if let Some(stem) = path_str.strip_suffix(".esp.toml") {
                output_path = PathBuf::from(stem.to_string()).with_extension("esp");
            } else if let Some(stem) = path_str.strip_suffix(".esp.json") {
                output_path = PathBuf::from(stem.to_string()).with_extension("esp");
            } else {
                output_path = input_path.with_extension("esp");
            }
        } else {
            output_path = input_path.with_extension("esp");
        }
    } else {
        output_path = append_ext("esp", output_path);
    }

    // check no input
    if let Some(i) = output {
        output_path = i.to_path_buf();
    }

    let mut plugin = Plugin::new();
    if let Ok(text) = fs::read_to_string(input_path) {
        if is_extension(input_path, "toml") {
            let deserialized: Result<_, _> = toml::from_str(&text);
            if let Ok(t) = deserialized {
                plugin = t;
            } else {
                return Err(Error::other("Failed to convert from toml"));
            }
        } else if is_extension(input_path, "json") {
            let deserialized: Result<_, _> = serde_json::from_str(&text);
            if let Ok(t) = deserialized {
                plugin = t;
            } else {
                return Err(Error::other("Failed to convert from json"));
            }
        } else if is_extension(input_path, "yaml") {
            let deserialized: Result<_, _> = serde_yaml::from_str(&text);
            match deserialized {
                Ok(t) => {
                    plugin = t;
                }
                Err(e) => {
                    println!("{}", e);
                    return Err(Error::other("Failed to convert from yaml"));
                }
            }
        }

        plugin.save_path(output_path)
    } else {
        Err(Error::other(
            "Failed to read the input file",
        ))
    }
}
