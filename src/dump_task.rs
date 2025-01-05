use std::{
    fs::{self, File},
    io::{self, Error, ErrorKind, Write},
    path::{Path, PathBuf},
};

use tes3::esp::{EditorId, Script, TES3Object, TypeInfo};

use crate::{parse_plugin, ESerializedType};

/// Dump data from an esp into files
pub fn dump(
    input: &Option<PathBuf>,
    out_dir: &Option<PathBuf>,
    create: bool,
    include: &[String],
    exclude: &[String],
    serialized_type: &Option<ESerializedType>,
) -> io::Result<()> {
    let mut is_file = false;
    let mut is_dir = false;

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
    } else if input_path.is_file() {
        let ext = input_path.extension();
        if let Some(e) = ext {
            let e_str = e.to_str().unwrap().to_lowercase();
            if e_str == "esp" || e_str == "esm" || e_str == "omwaddon" {
                is_file = true;
            }
        }
    } else if input_path.is_dir() {
        is_dir = true;
    }

    // check output path, default is cwd
    let mut out_dir_path = &PathBuf::from("");
    if let Some(p) = out_dir {
        out_dir_path = p;
    }

    // check serialized type, default is yaml
    let mut stype = &ESerializedType::Yaml;
    if let Some(t) = serialized_type {
        stype = t;
    }

    // dump plugin file
    if is_file {
        if create {
            match dump_plugin(
                input_path,
                &out_dir_path.join(input_path.file_stem().unwrap()),
                include,
                exclude,
                stype,
            ) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        } else {
            match dump_plugin(input_path, out_dir_path, include, exclude, stype) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
    }

    // dump folder
    // input is a folder, it may contain many plugins (a.esp, b.esp)
    // dumps scripts into cwd/a/ and cwd/b
    // check if already exists?
    if is_dir {
        // get all plugins non-recursively
        let paths = fs::read_dir(input_path).unwrap();
        for entry in paths.flatten() {
            let path = entry.path();
            if path.is_file() && path.exists() {
                let ext = path.extension();
                if let Some(e) = ext {
                    let e_str = e.to_str().unwrap().to_lowercase();

                    if e_str == "esp" || e_str == "esm" || e_str == "omwaddon" {
                        // dump scripts into folders named after the plugin name
                        let plugin_name = path.file_stem().unwrap();
                        let out_path = &out_dir_path.join(plugin_name);

                        match dump_plugin(&path, out_path, include, exclude, stype) {
                            Ok(_) => {}
                            Err(e) => return Err(e),
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Dumps one plugin
fn dump_plugin(
    input: &PathBuf,
    out_dir_path: &Path,
    include: &[String],
    exclude: &[String],
    typ: &ESerializedType,
) -> Result<(), Error> {
    let plugin = parse_plugin(input);
    // parse plugin
    // write
    match plugin {
        Ok(p) => {
            for object in p.objects {
                // if (!include.is_empty() && include.contains(&object.tag_str().to_owned()))
                //     && !exclude.contains(&object.tag_str().to_owned())
                // first check for exclusion
                if exclude.contains(&object.tag_str().to_owned()) {
                    continue;
                }
                if !include.is_empty() && !include.contains(&object.tag_str().to_owned()) {
                    continue;
                }

                write_object(&object, out_dir_path, typ);
            }
        }
        Err(_) => {
            return Err(Error::new(ErrorKind::Other, "Plugin parsing failed."));
        }
    }
    Ok(())
}

fn write_object(object: &TES3Object, out_dir_path: &Path, serialized_type: &ESerializedType) {
    match object {
        TES3Object::Header(_) => {
            let name = format!("{}.{}", "Header", serialized_type);
            write_generic(object, &name, &out_dir_path.join("Header"), serialized_type)
                .unwrap_or_else(|e| println!("Writing failed: {}, {}", name, e));
        }

        TES3Object::Script(script) => {
            let nam = object.editor_id().to_string();
            let typ = object.type_name().to_string();

            let name = format!("{}.{}", nam, serialized_type);
            write_generic(object, &name, &out_dir_path.join(typ), serialized_type)
                .unwrap_or_else(|e| println!("Writing failed: {}, {}", name, e));

            write_script(script, &out_dir_path.join("Script"))
                .unwrap_or_else(|_| panic!("Writing failed: {}", script.id));
        }
        TES3Object::GameSetting(_)
        | TES3Object::Skill(_)
        | TES3Object::MagicEffect(_)
        | TES3Object::GlobalVariable(_)
        | TES3Object::Class(_)
        | TES3Object::Faction(_)
        | TES3Object::Race(_)
        | TES3Object::Sound(_)
        | TES3Object::Region(_)
        | TES3Object::Birthsign(_)
        | TES3Object::StartScript(_)
        | TES3Object::LandscapeTexture(_)
        | TES3Object::Spell(_)
        | TES3Object::Static(_)
        | TES3Object::Door(_)
        | TES3Object::MiscItem(_)
        | TES3Object::Weapon(_)
        | TES3Object::Container(_)
        | TES3Object::Creature(_)
        | TES3Object::Cell(_)
        | TES3Object::Bodypart(_)
        | TES3Object::Light(_)
        | TES3Object::Enchanting(_)
        | TES3Object::Npc(_)
        | TES3Object::Armor(_)
        | TES3Object::Clothing(_)
        | TES3Object::RepairItem(_)
        | TES3Object::Activator(_)
        | TES3Object::Apparatus(_)
        | TES3Object::Lockpick(_)
        | TES3Object::Probe(_)
        | TES3Object::Ingredient(_)
        | TES3Object::Book(_)
        | TES3Object::Alchemy(_)
        | TES3Object::LeveledItem(_)
        | TES3Object::LeveledCreature(_)
        | TES3Object::SoundGen(_)
        | TES3Object::Dialogue(_)
        | TES3Object::Landscape(_)
        | TES3Object::PathGrid(_)
        | TES3Object::DialogueInfo(_) => {
            let nam = object.editor_id().to_string();
            let typ = object.type_name().to_string();

            let name = format!("{}.{}", nam, serialized_type);
            write_generic(object, &name, &out_dir_path.join(typ), serialized_type)
                .unwrap_or_else(|e| println!("Writing failed: {}, {}", name, e));
        }
    }
}

/// Write a tes3object script to a file
fn write_script(script: &Script, out_dir: &Path) -> io::Result<()> {
    if !out_dir.exists() {
        // create directory
        match fs::create_dir_all(out_dir) {
            Ok(_) => {}
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Failed to create output directory.",
                ));
            }
        }
    }

    // get name
    let name = format!("{}.mwscript", script.id);
    // get script plaintext
    // write to file
    let output_path = out_dir.join(name);
    let file_or_error = File::create(output_path);
    match file_or_error {
        Ok(mut file) => match file.write_all(script.text.as_bytes()) {
            Ok(_) => {
                // todo verbosity
                //println!("SCPT written to: {}", output_path.display());
            }
            Err(_) => {
                return Err(Error::new(ErrorKind::Other, "File write failed"));
            }
        },
        Err(_) => {
            return Err(Error::new(ErrorKind::Other, "File create failed"));
        }
    }

    Ok(())
}

/// Write a generic tes3object to a file
fn write_generic(
    object: &TES3Object,
    name: &String,
    out_dir: &Path,
    typ: &ESerializedType,
) -> io::Result<()> {
    let text = match serialize(typ, object) {
        Ok(value) => value,
        Err(value) => return value,
    };

    write_to_file(out_dir, name, text)
}

/// Serialize a TES3Object to text
fn serialize(typ: &ESerializedType, object: &TES3Object) -> Result<String, Result<(), Error>> {
    let text = match typ {
        ESerializedType::Yaml => {
            let result = serde_yaml::to_string(object);
            match result {
                Ok(t) => t,
                Err(e) => {
                    return Err(Err(Error::new(ErrorKind::Other, e.to_string())));
                }
            }
        }
        ESerializedType::Toml => {
            let result = toml::to_string_pretty(&object);
            match result {
                Ok(t) => t,
                Err(e) => {
                    return Err(Err(Error::new(ErrorKind::Other, e.to_string())));
                }
            }
        }
        ESerializedType::Json => {
            let result = serde_json::to_string_pretty(&object);
            match result {
                Ok(t) => t,
                Err(e) => {
                    return Err(Err(Error::new(ErrorKind::Other, e.to_string())));
                }
            }
        }
    };
    Ok(text)
}

/// Convenience function to write TES3Object text to a file
fn write_to_file(out_dir: &Path, name: &String, text: String) -> Result<(), Error> {
    // create directory
    if !out_dir.exists() {
        match fs::create_dir_all(out_dir) {
            Ok(_) => {}
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "Failed to create output directory.",
                ));
            }
        }
    }

    // write to file
    let output_path = out_dir.join(name);
    let file_or_error = File::create(output_path);
    match file_or_error {
        Ok(mut file) => match file.write_all(text.as_bytes()) {
            Ok(_) => {
                // todo verbosity
                //println!("MISC writen to: {}", output_path.display());
                Ok(())
            }
            Err(_) => Err(Error::new(ErrorKind::Other, "File write failed")),
        },
        Err(_) => Err(Error::new(ErrorKind::Other, "File create failed")),
    }
}
