use std::path::{Path, PathBuf};

use atlas_task::atlas_coverage;
use deserialize_task::deserialize_plugin;
use dump_task::dump;
use pack_task::pack;
use serialize_task::serialize_plugin;
use tes3util::*;

#[test]
#[ignore]
fn test_serialize_to_yaml() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP");
    serialize_plugin(&Some(input.into()), &None, &Some(ESerializedType::Yaml))
}
#[test]
#[ignore]
fn test_serialize_to_toml() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP");
    serialize_plugin(&Some(input.into()), &None, &Some(ESerializedType::Toml))
}
#[test]
#[ignore]
fn test_serialize_to_json() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP");
    serialize_plugin(&Some(input.into()), &None, &Some(ESerializedType::Json))
}

#[test]
#[ignore]
fn test_deserialize_from_yaml() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP.yaml");
    deserialize_plugin(&Some(input.into()), &None, false)
}
#[test]
#[ignore]
fn test_deserialize_from_toml() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP.toml");
    deserialize_plugin(&Some(input.into()), &None, false)
}
#[test]
#[ignore]
fn test_deserialize_from_json() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP.json");
    deserialize_plugin(&Some(input.into()), &None, false)
}

#[test]
#[ignore]
fn test_dump_yaml() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP");
    let output = Path::new("tests/assets/out");
    dump(
        &Some(input.into()),
        &Some(output.into()),
        false,
        &[],
        &[],
        &Some(ESerializedType::Yaml),
    )
}
#[test]
#[ignore]
fn test_dump_toml() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP");
    let output = Path::new("tests/assets/out");
    dump(
        &Some(input.into()),
        &Some(output.into()),
        false,
        &[],
        &[],
        &Some(tes3util::ESerializedType::Toml),
    )
}
#[test]
#[ignore]
fn test_dump_json() -> std::io::Result<()> {
    let input = Path::new("tests/assets/Ashlander Crafting.ESP");
    let output = Path::new("tests/assets/out");
    dump(
        &Some(input.into()),
        &Some(output.into()),
        false,
        &[],
        &[],
        &Some(ESerializedType::Json),
    )
}

#[test]
#[ignore]
fn test_pack_yaml() -> std::io::Result<()> {
    let input = PathBuf::from("tests/assets/out");
    let output = PathBuf::from("tests/assets/out/test.yaml.esp");
    pack(&Some(input), &Some(output), &Some(ESerializedType::Yaml))
}
#[test]
#[ignore]
fn test_pack_toml() -> std::io::Result<()> {
    let input = PathBuf::from("tests/assets/out");
    let output = PathBuf::from("tests/assets/out/test.toml.esp");
    pack(&Some(input), &Some(output), &Some(ESerializedType::Toml))
}
#[test]
#[ignore]
fn test_pack_json() -> std::io::Result<()> {
    let input = PathBuf::from("tests/assets/out");
    let output = PathBuf::from("tests/assets/out/test.json.esp");
    pack(&Some(input), &Some(output), &Some(ESerializedType::Json))
}

#[test]
fn test_atlas_coverage() -> std::io::Result<()> {
    let input = Path::new("tests/assets");
    let output = Path::new("tests/assets/out");
    atlas_coverage(&Some(input.into()), &Some(output.into()))
}
