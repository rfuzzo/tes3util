use crate::get_all_tags;
use fnv_rs::{Fnv64, FnvHasher};
use rusqlite::{params, Connection, Result};
use tes3::esp::traits::TableSchema;
use tes3::esp::EditorId;
use tes3::esp::SqlInfo;
//use sha1::{Digest, Sha1};
use std::{collections::HashMap, path::PathBuf};

use crate::as_json;
use crate::as_option;
use crate::create_from_tag;
use crate::parse_plugin;

struct PluginModel {
    id: String,
    name: String,
    crc: u32,
    load_order: u32,
}

pub fn sql_task(input: &Option<PathBuf>, output: &Option<PathBuf>) -> Result<()> {
    if let Some(output) = output {
        // create esp db
        let db = Connection::open(output)?;

        // create plugins db
        db.execute(
            "CREATE TABLE plugins (
            id   TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            crc INTEGER NOT NULL,
            load_order INTEGER NOT NULL
        )",
            (), // empty list of parameters.
        )?;

        let schemas = get_schemas();
        create_tables(&db, &schemas)?;

        // debug todo
        for tag in get_all_tags() {
            if let Some(instance) = create_from_tag(&tag) {
                let txt = instance.table_insert();
                println!("{}", txt);
            }
        }

        let mut plugins = HashMap::new();

        if let Some(input) = input {
            // populate db
            if let Ok(plugin) = parse_plugin(input) {
                let filename = input.file_name().unwrap().to_str().unwrap();
                let hash = Fnv64::hash(filename.as_bytes()).as_hex();
                //let mut hasher = Sha1::new();
                let plugin_model = PluginModel {
                    id: hash.to_owned(),
                    name: filename.to_string(),
                    crc: 0,        // todo
                    load_order: 0, // todo
                };
                // add plugin to db
                db.execute(
                    "INSERT INTO plugins (id, name, crc, load_order) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        plugin_model.id,
                        plugin_model.name,
                        plugin_model.crc,
                        plugin_model.load_order
                    ],
                )?;

                plugins.insert(hash, plugin);
            }
        }

        for (hash, plugin) in &plugins {
            for record in &plugin.objects {
                insert_into_db(&db, hash, record);
            }
        }
    }

    Ok(())
}

fn create_tables(conn: &Connection, schemas: &[TableSchema]) -> Result<()> {
    for schema in schemas {
        let columns = schema.columns.join(", ");
        let constraints = schema.constraints.join(", ");
        // TODO flags
        let sql = if constraints.is_empty() {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                id  TEXT PRIMARY KEY,
                mod TEXT NOT NULL,
                {},
                FOREIGN KEY(mod) REFERENCES plugins(id)
                )",
                schema.name, columns
            )
        } else {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                id  TEXT PRIMARY KEY,
                mod TEXT NOT NULL,
                {}, 
                FOREIGN KEY(mod) REFERENCES plugins(id),
                {}
                )",
                schema.name, columns, constraints
            )
        };

        println!("{}", sql);

        conn.execute(&sql, [])?;
    }
    Ok(())
}

fn get_schemas() -> Vec<TableSchema> {
    let mut schemas = Vec::new();
    for tag in get_all_tags() {
        if let Some(instance) = create_from_tag(&tag) {
            schemas.push(instance.table_schema());
        }
    }

    schemas
}

fn insert_into_db(db: &Connection, hash: &str, record: &tes3::esp::TES3Object) {
    match record {
        tes3::esp::TES3Object::GameSetting(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![s.id, hash, as_json!(s.value)],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::GlobalVariable(s) => {
            let value = match s.value {
                tes3::esp::GlobalValue::Float(f) => f.to_string(),
                tes3::esp::GlobalValue::Short(s) => s.to_string(),
                tes3::esp::GlobalValue::Long(l) => l.to_string(),
            };

            db.execute(s.table_insert().as_str(), params![s.id, hash, value])
                .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Class(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![s.id, hash, s.name, s.description, as_json!(s.data)],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Faction(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![
                    s.id,
                    hash,
                    s.name,
                    as_json!(s.rank_names),
                    as_json!(s.reactions),
                    as_json!(s.data.favored_attributes),
                    as_json!(s.data.requirements),
                    as_json!(s.data.favored_skills),
                    as_json!(s.data.flags)
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Race(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![
                    s.id,
                    hash,
                    s.name,
                    as_json!(s.spells),
                    s.description,
                    as_json!(s.data)
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::MiscItem(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![
                    s.id,
                    hash,
                    s.name,
                    as_option!(s.script),
                    s.mesh,
                    s.icon,
                    s.data.weight,
                    s.data.value,
                    as_json!(s.data.flags)
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Weapon(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![
                    s.id,
                    hash,
                    s.name,
                    as_option!(s.script),
                    s.mesh,
                    s.icon,
                    s.enchanting,
                    s.data.weight,
                    s.data.value,
                    as_json!(s.data.weapon_type),
                    s.data.health,
                    s.data.speed,
                    s.data.reach,
                    s.data.enchantment,
                    s.data.chop_min,
                    s.data.chop_max,
                    s.data.slash_min,
                    s.data.slash_max,
                    s.data.thrust_min,
                    s.data.thrust_max,
                    as_json!(s.data.flags)
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Static(s) => {
            db.execute(s.table_insert().as_str(), params![s.id, hash, s.mesh])
                .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Npc(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![
                    s.id,
                    hash,
                    s.name,
                    as_option!(s.script),
                    s.mesh,
                    as_json!(s.inventory),
                    as_json!(s.spells),
                    as_json!(s.ai_data),
                    as_json!(s.ai_packages),
                    as_json!(s.travel_destinations),
                    s.race,
                    s.class,
                    as_option!(s.faction),
                    s.head,
                    s.hair,
                    as_json!(s.npc_flags),
                    s.blood_type,
                    s.data.level,
                    as_json!(s.data.stats),
                    s.data.disposition,
                    s.data.reputation,
                    s.data.rank,
                    s.data.gold
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Activator(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![s.id, hash, s.name, as_option!(s.script), s.mesh],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Script(s) => {
            db.execute(s.table_insert().as_str(), params![s.id, hash, s.text])
                .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Region(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![
                    s.id,
                    hash,
                    s.name,
                    s.weather_chances.clear,
                    s.weather_chances.cloudy,
                    s.weather_chances.foggy,
                    s.weather_chances.overcast,
                    s.weather_chances.rain,
                    s.weather_chances.thunder,
                    s.weather_chances.ash,
                    s.weather_chances.blight,
                    s.weather_chances.snow,
                    s.weather_chances.blizzard,
                    s.sleep_creature,
                    as_json!(s.map_color),
                    as_json!(s.sounds)
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::LeveledItem(s) => {
            db.execute(
                s.table_insert().as_str(),
                params![
                    s.id,
                    hash,
                    as_json!(s.leveled_item_flags),
                    s.chance_none,
                    as_json!(s.items)
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Cell(s) => {
            let references =
                serde_json::to_string_pretty(&s.references.values().collect::<Vec<_>>()).unwrap();
            let id = s.editor_id().to_string();

            db.execute(
                s.table_insert().as_str(),
                params![
                    id,
                    hash,
                    s.name,
                    as_json!(s.data.flags),
                    as_json!(s.data.grid),
                    s.region,
                    s.water_height,
                    references
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", id));
        }
        _ => {}
    }
}

#[test]
fn test_sql_task() -> Result<()> {
    let input = std::path::Path::new("tests/assets/Morrowind.esm");
    let output = std::path::Path::new("./tes3.db3");
    // delete db if exists
    if output.exists() {
        std::fs::remove_file(output).expect("Could not delete file");
    }

    sql_task(&Some(input.into()), &Some(output.into()))
}
