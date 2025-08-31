use std::{
    env,
    fs::{self},
    io::Error,
    path::PathBuf,
};

use tes3::esp::{Plugin, TES3Object, TypeInfo};

use crate::{append_ext, ESerializedType};

/// Pack a folder of serialized files into a plugin
pub fn pack(
    cinput_path: &Option<PathBuf>,
    output_path: &Option<PathBuf>,
    cformat: &Option<ESerializedType>,
) -> Result<(), Error> {
    // check input path, default is cwd
    let mut input_path = env::current_dir()?;
    if let Some(p) = cinput_path {
        input_path.clone_from(p);
    }

    let format = match cformat {
        Some(f) => f,
        None => &ESerializedType::Yaml,
    };

    let mut files = vec![];
    // get all files
    for entry in fs::read_dir(&input_path).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() && path.exists() {
            // match folder name with type_name
            //let folder_name = path.file_name().unwrap().to_str().unwrap();
            for file_entry in fs::read_dir(path).unwrap().flatten() {
                let file = file_entry.path();
                if file.is_file() && file.exists() {
                    if let Some(e) = file.extension() {
                        if e == format.to_string().as_str() {
                            files.push(file);
                        }
                    }
                }
            }
        }
    }

    // Deserialize records from files
    let mut records = vec![];
    for file_path in files {
        let result = fs::read_to_string(&file_path);
        if let Ok(text) = result {
            match format {
                ESerializedType::Yaml => {
                    let deserialized: Result<TES3Object, _> = serde_yaml_ng::from_str(&text);
                    if let Ok(object) = deserialized {
                        records.push(object);
                    } else {
                        println!("failed deserialization for {}", file_path.display());
                    }
                }
                ESerializedType::Toml => {
                    let deserialized: Result<TES3Object, _> = toml::from_str(&text);
                    if let Ok(object) = deserialized {
                        records.push(object);
                    } else {
                        println!("failed deserialization for {}", file_path.display());
                    }
                }
                ESerializedType::Json => {
                    let deserialized: Result<TES3Object, _> = serde_json::from_str(&text);
                    if let Ok(object) = deserialized {
                        records.push(object);
                    } else {
                        println!("failed deserialization for {}", file_path.display());
                    }
                }
            }
        }
    }

    let pos = records.iter().position(|e| e.tag_str() == "TES3").unwrap();
    let header = records.remove(pos);
    records.insert(0, header);

    // make plugin
    let mut plugin = Plugin::new();
    plugin.objects = records;

    // save
    let nam = input_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut i = input_path.join(nam);
    i = append_ext("esp", i);
    let mut output = i.as_path();
    if let Some(o) = output_path {
        output = o;
    }

    plugin.save_path(output)
}
