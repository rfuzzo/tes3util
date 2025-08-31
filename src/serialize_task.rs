use std::{
    fs::File,
    io::{self, Error, ErrorKind, Write},
    path::PathBuf,
};

use crate::{append_ext, is_extension, parse_plugin, ESerializedType};

/// Serialize a plugin to a human-readable format
pub fn serialize_plugin(
    input: &Option<PathBuf>,
    output: &Option<PathBuf>,
    cformat: &Option<ESerializedType>,
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
    if !input_path.exists()
        || (input_path.exists()
            && (!input_path.is_file()
                || !(is_extension(input_path, "esp")
                    || is_extension(input_path, "esm")
                    || is_extension(input_path, "omwaddon"))))
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Input path does not exist",
        ));
    }

    let format = match cformat {
        Some(f) => f,
        None => &ESerializedType::Yaml,
    };

    let mut output_path = PathBuf::from(input_path.clone().to_str().unwrap());
    // check no input
    if let Some(i) = output {
        output_path = i.to_path_buf();
    }
    output_path = append_ext(format.to_string(), output_path);

    let plugin_or_error = parse_plugin(input_path);
    // parse plugin
    // write
    match plugin_or_error {
        Ok(plugin) => {
            let text = match format {
                ESerializedType::Yaml => {
                    let result = serde_yaml_ng::to_string(&plugin);
                    match result {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(Error::other(e.to_string()));
                        }
                    }
                }
                ESerializedType::Toml => {
                    let result = toml::to_string_pretty(&plugin);
                    match result {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(Error::other(e.to_string()));
                        }
                    }
                }
                ESerializedType::Json => {
                    let result = serde_json::to_string_pretty(&plugin);
                    match result {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(Error::other(e.to_string()));
                        }
                    }
                }
            };

            File::create(output_path)?.write_all(text.as_bytes())
        }
        Err(_) => Err(Error::other("Plugin parsing failed.")),
    }
}
