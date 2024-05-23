use std::{
    collections::HashMap,
    env, fmt,
    fs::{self, File},
    io::{self, Error, ErrorKind, Read, Write},
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;
use tes3::esp::{EditorId, Plugin, Script, TES3Object};
use tes3::{esp::TypeInfo, nif};
use walkdir::WalkDir;

#[derive(Default, Clone, ValueEnum)]
pub enum ESerializedType {
    #[default]
    Yaml,
    Toml,
    Json,
}
impl fmt::Display for ESerializedType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ESerializedType::Yaml => write!(f, "yaml"),
            ESerializedType::Toml => write!(f, "toml"),
            ESerializedType::Json => write!(f, "json"),
        }
    }
}

fn is_extension(path: &Path, extension: &str) -> bool {
    match path.extension() {
        Some(e) => {
            let l = e.to_ascii_lowercase();
            l == extension.to_lowercase().as_str()
        }
        None => false,
    }
}

// https://internals.rust-lang.org/t/pathbuf-has-set-extension-but-no-add-extension-cannot-cleanly-turn-tar-to-tar-gz/14187/11
pub fn append_ext(ext: impl AsRef<std::ffi::OsStr>, path: PathBuf) -> PathBuf {
    let mut os_string: std::ffi::OsString = path.into();
    os_string.push(".");
    os_string.push(ext.as_ref());
    os_string.into()
}

/// Parse the contents of the given path into a TES3 Plugin.
/// Whether to parse as JSON or binary is inferred from first character.
/// taken from: https://github.com/Greatness7/tes3conv
fn parse_plugin(path: &PathBuf) -> io::Result<Plugin> {
    let mut raw_data = vec![];
    File::open(path)?.read_to_end(&mut raw_data)?;

    let mut plugin = Plugin::new();

    match raw_data.first() {
        Some(b'T') => {
            // if it starts with a 'T' assume it's a TES3 file
            plugin.load_bytes(&raw_data)?;
        }
        _ => {
            // anything else is guaranteed to be invalid input
            return Err(Error::new(ErrorKind::InvalidData, "Invalid input."));
        }
    }

    // sort objects so that diffs are a little more useful
    //plugin.sort();    //TODO

    Ok(plugin)
}

///////////////////////////////////////////////////////////////////////////
// Serialize

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
                    let result = serde_yaml::to_string(&plugin);
                    match result {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(Error::new(ErrorKind::Other, e.to_string()));
                        }
                    }
                }
                ESerializedType::Toml => {
                    let result = toml::to_string_pretty(&plugin);
                    match result {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(Error::new(ErrorKind::Other, e.to_string()));
                        }
                    }
                }
                ESerializedType::Json => {
                    let result = serde_json::to_string_pretty(&plugin);
                    match result {
                        Ok(t) => t,
                        Err(e) => {
                            return Err(Error::new(ErrorKind::Other, e.to_string()));
                        }
                    }
                }
            };

            return File::create(output_path)?.write_all(text.as_bytes());
        }
        Err(_) => Err(Error::new(ErrorKind::Other, "Plugin parsing failed.")),
    }
}

///////////////////////////////////////////////////////////////////////////
// Dump

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

///////////////////////////////////////////////////////////////////////////
// Deserialize

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
                return Err(Error::new(ErrorKind::Other, "Failed to convert from toml"));
            }
        } else if is_extension(input_path, "json") {
            let deserialized: Result<_, _> = serde_json::from_str(&text);
            if let Ok(t) = deserialized {
                plugin = t;
            } else {
                return Err(Error::new(ErrorKind::Other, "Failed to convert from json"));
            }
        } else if is_extension(input_path, "yaml") {
            let deserialized: Result<_, _> = serde_yaml::from_str(&text);
            match deserialized {
                Ok(t) => {
                    plugin = t;
                }
                Err(e) => {
                    println!("{}", e);
                    return Err(Error::new(ErrorKind::Other, "Failed to convert from yaml"));
                }
            }
        }

        plugin.save_path(output_path)
    } else {
        Err(Error::new(
            ErrorKind::Other,
            "Failed to read the input file",
        ))
    }
}

///////////////////////////////////////////////////////////////////////////
// Pack

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
                    let deserialized: Result<TES3Object, _> = serde_yaml::from_str(&text);
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

///////////////////////////////////////////////////////////////////////////
/// AtlasCoverage

fn read_file_contents(file_path: &String) -> io::Result<(String, Vec<String>)> {
    // load nif
    let path = PathBuf::from(&file_path);
    if let Ok(list) = get_textures_from_nif(&path.clone()) {
        return Ok((file_path.clone(), list));
    }

    Err(Error::new(ErrorKind::Other, "Failed to read file contents"))
}

pub fn atlas_coverage(input: &Option<PathBuf>, output: &Option<PathBuf>) -> io::Result<()> {
    // check output path, default is cwd
    let mut out_dir_path = env::current_dir()?;
    if let Some(p) = output {
        p.clone_into(&mut out_dir_path);
    }

    // check input path, default is cwd
    let mut input_path = env::current_dir()?;
    if let Some(p) = input {
        p.clone_into(&mut input_path);
    }

    // map of textures by nif file
    let mut map_none: HashMap<String, Vec<String>> = HashMap::new();
    let mut map_some: HashMap<String, Vec<String>> = HashMap::new();

    // log parse nif files
    println!("Parsing nif files in: {}", input_path.display());

    // get all .nif or .NIF files in the input folder recursively in a list
    let mut nif_files = Vec::new();
    for entry in WalkDir::new(input_path).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path().to_owned();
            if is_extension(&path, "nif") {
                nif_files.push(entry.path().to_string_lossy().into_owned());
            }
        }
    }

    // iterate over nif files
    // Read file contents in parallel
    let contents: Vec<_> = nif_files
        .par_iter() // Parallel iterator
        .map(read_file_contents) // Read file contents
        .collect::<Vec<_>>();

    // iterate over results
    for result in contents {
        match result {
            Ok((file, list)) => {
                // if any entries in the list have "textures\atl" in them, add to map_some
                // else add to map_none
                let mut found = false;
                for texture in &list {
                    if texture.contains("textures\\atl") {
                        found = true;
                        break;
                    }
                }

                if found {
                    map_some.insert(file, list);
                } else {
                    map_none.insert(file, list);
                }
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    // print maps count
    println!(
        "Nif files with textures in textures\\atl: {}",
        map_some.len()
    );
    println!(
        "Nif files without textures in textures\\atl: {}",
        map_none.len()
    );

    // serialize map to output folder
    {
        println!("Serializing to: {}", out_dir_path.display());
        // create output folder
        if !out_dir_path.exists() {
            fs::create_dir_all(&out_dir_path)?;
        }
        let mut output_path = out_dir_path.join("atlas_coverage");
        output_path = append_ext("yaml", output_path);
        // serialize to yaml
        // make a new object with the two maps
        let mut map = HashMap::new();
        map.insert("with_atl", &map_some);
        map.insert("without_atl", &map_none);

        let text = serde_yaml::to_string(&map).unwrap();
        let mut file = File::create(output_path)?;
        file.write_all(text.as_bytes())?;
    }

    // serialize some statistics
    {
        println!("Serializing stats to: {}", out_dir_path.display());
        let mut stats = HashMap::new();
        stats.insert("with_atl", map_some.len().to_string());
        stats.insert("without_atl", map_none.len().to_string());
        // coverage
        let total = map_some.len() + map_none.len();
        let coverage = (map_some.len() as f32 / total as f32) * 100.0;
        stats.insert("coverage", coverage.to_string());

        let text = serde_yaml::to_string(&stats).unwrap();
        let mut file = File::create(out_dir_path.join("atlas_coverage_stats.yaml"))?;
        file.write_all(text.as_bytes())?;
    }

    Ok(())
}

fn get_textures_from_nif(path: &PathBuf) -> Result<Vec<String>, Error> {
    let mut list = Vec::new();

    let mut stream = nif::NiStream::new();
    stream.load_path(path)?;

    for texture in stream.objects_of_type::<nif::NiSourceTexture>() {
        match &texture.source {
            nif::TextureSource::External(e) => {
                list.push(e.to_string().to_lowercase());
            }
            nif::TextureSource::Internal(_i) => {
                list.push(String::from("internal"));
            }
        }
    }

    Ok(list)
}
