use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, Error, Write},
    path::PathBuf,
};

use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;
use tes3::nif;
use walkdir::WalkDir;

use crate::{append_ext, is_extension};

fn read_file_contents(file_path: &String) -> io::Result<(String, Vec<String>)> {
    // load nif
    let path = PathBuf::from(&file_path);
    if let Ok(list) = get_textures_from_nif(&path.clone()) {
        return Ok((file_path.clone(), list));
    }

    Err(Error::other("Failed to read file contents"))
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

        let text = serde_yaml_ng::to_string(&map).unwrap();
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

        let text = serde_yaml_ng::to_string(&stats).unwrap();
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
