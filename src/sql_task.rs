use fnv_rs::{Fnv64, FnvHasher};
use rusqlite::{params, Connection, Result};
use tes3::esp::EditorId;
//use sha1::{Digest, Sha1};
use std::{collections::HashMap, path::PathBuf};

use crate::parse_plugin;

struct PluginModel {
    id: String,
    name: String,
    crc: u32,
    load_order: u32,
}

struct TableSchema {
    name: &'static str,
    columns: Vec<&'static str>,
    constraints: Vec<&'static str>,
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

        create_tables(&db, &get_schemas())?;

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
    vec![
        TableSchema {
            name: "GMST",
            columns: vec!["value TEXT"],
            constraints: vec![],
        },
        TableSchema {
            name: "GLOB",
            columns: vec!["global_type TEXT", "value REAL"],
            constraints: vec![],
        },
        TableSchema {
            name: "CLAS",
            columns: vec!["name TEXT", "description TEXT", "data TEXT"],
            constraints: vec![],
        },
        TableSchema {
            name: "FACT",
            columns: vec![
                "name TEXT",
                "rank_names TEXT",
                "reactions TEXT",
                "data TEXT",
            ],
            constraints: vec![],
        },
        TableSchema {
            name: "STAT",
            columns: vec!["mesh TEXT"],
            constraints: vec![],
        },
        TableSchema {
            name: "SCPT",
            columns: vec!["text TEXT"],
            constraints: vec![],
        },
        TableSchema {
            name: "REGN",
            columns: vec![
                "name TEXT",
                "weather_chances TEXT",
                "sleep_creature TEXT",
                "sounds TEXT",
            ],
            constraints: vec![],
        },
        TableSchema {
            name: "ACTI",
            columns: vec!["name TEXT", "script TEXT", "mesh TEXT"],
            constraints: vec!["FOREIGN KEY(script) REFERENCES SCPT(id)"],
        },
        TableSchema {
            name: "CELL",
            columns: vec![
                "name TEXT",
                "data_flags TEXT",
                "data_grid TEXT",
                "region TEXT",
                "water_height REAL",
                "cell_references TEXT",
            ],
            constraints: vec!["FOREIGN KEY(region) REFERENCES REGN(id)"],
        },
    ]
}

fn insert_into_db(db: &Connection, hash: &str, record: &tes3::esp::TES3Object) {
    match record {
        tes3::esp::TES3Object::GameSetting(s) => {
            let value_str = serde_json::to_string_pretty(&s.value).unwrap();

            db.execute(
                "INSERT INTO 
                GMST 
                (id, mod, value) 
                VALUES 
                (?1, ?2, ?3)",
                params![s.id, hash, value_str],
            )
            .expect("Could not insert into db");
        }
        tes3::esp::TES3Object::GlobalVariable(s) => {
            let global_type = serde_json::to_string_pretty(&s.global_type).unwrap();

            db.execute(
                "INSERT INTO 
                GLOB 
                (id, mod, global_type, value) 
                VALUES 
                (?1, ?2, ?3, ?4)",
                params![s.id, hash, global_type, s.value],
            )
            .expect("Could not insert into db");
        }
        tes3::esp::TES3Object::Class(s) => {
            let data = serde_json::to_string_pretty(&s.data).unwrap();

            db.execute(
                "INSERT INTO 
                CLAS 
                (id, mod, name, description, data) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5)",
                params![s.id, hash, s.name, s.description, data],
            )
            .expect("Could not insert into db");
        }
        tes3::esp::TES3Object::Faction(s) => {
            let rank_names = serde_json::to_string_pretty(&s.rank_names).unwrap();
            let reactions = serde_json::to_string_pretty(&s.reactions).unwrap();
            let data = serde_json::to_string_pretty(&s.data).unwrap();

            db.execute(
                "INSERT INTO 
                FACT 
                (id, mod, name, rank_names, reactions, data) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5, ?6)",
                params![s.id, hash, s.name, rank_names, reactions, data],
            )
            .expect("Could not insert into db");
        }
        tes3::esp::TES3Object::Static(s) => {
            db.execute(
                "INSERT INTO 
                STAT 
                (id, mod, mesh) 
                VALUES 
                (?1, ?2, ?3)",
                params![s.id, hash, s.mesh],
            )
            .expect("Could not insert into db");
        }
        tes3::esp::TES3Object::Activator(s) => {
            let mut script_id = Some(s.script.to_owned());
            if let Some(ref s) = script_id {
                if s.is_empty() {
                    script_id = None;
                }
            }
            db.execute(
                "INSERT INTO 
                ACTI 
                (id, mod, name, script, mesh) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5)",
                params![s.id, hash, s.name, script_id.to_owned(), s.mesh],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Script(s) => {
            db.execute(
                "INSERT INTO 
                SCPT 
                (id, mod, text) 
                VALUES 
                (?1, ?2, ?3)",
                params![s.id, hash, s.text],
            )
            .expect("Could not insert into db");
        }
        tes3::esp::TES3Object::Region(s) => {
            let weather_chances = serde_json::to_string_pretty(&s.weather_chances).unwrap();
            let sounds = serde_json::to_string_pretty(&s.sounds).unwrap();

            db.execute(
                "INSERT INTO 
                REGN 
                (id, mod, name, weather_chances, sleep_creature, sounds) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    s.id,
                    hash,
                    s.name,
                    weather_chances,
                    s.sleep_creature,
                    sounds
                ],
            )
            .expect("Could not insert into db");
        }
        tes3::esp::TES3Object::Cell(s) => {
            let data_flags = serde_json::to_string_pretty(&s.data.flags).unwrap();
            let data_grid = serde_json::to_string_pretty(&s.data.grid).unwrap();
            let references =
                serde_json::to_string_pretty(&s.references.values().collect::<Vec<_>>()).unwrap();
            let id = s.editor_id().to_string();

            db.execute(
                "INSERT INTO 
                CELL 
                (id, mod, name, data_flags, data_grid, region, water_height, cell_references) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    id,
                    hash,
                    s.name,
                    data_flags,
                    data_grid,
                    s.region,
                    s.water_height,
                    references
                ],
            )
            .expect("Could not insert into db");
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
