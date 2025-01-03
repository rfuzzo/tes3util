use fnv_rs::{Fnv64, FnvHasher};
use rusqlite::{params, Connection, Result};
use tes3::esp::EditorId;
//use sha1::{Digest, Sha1};
use std::{collections::HashMap, path::PathBuf};

use crate::as_option;
use crate::as_json;
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
    vec![
        TableSchema {
            name: "GMST",
            columns: vec!["value TEXT"], // TODO union
            constraints: vec![],
        },
        TableSchema {
            name: "GLOB",
            columns: vec!["value REAL"],
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
            name: "RACE",
            columns: vec![
                "name TEXT",
                "spells TEXT",
                "description TEXT",
                "data TEXT",
            ],
            constraints: vec![],
        },
        TableSchema {
            name: "MISC",
            columns: vec![
                "name TEXT",
                "script TEXT",
                "mesh TEXT",
                "icon TEXT",
                "weight REAL",
                "value INTEGER",
                "flags TEXT",
            ],
            constraints: vec!["FOREIGN KEY(script) REFERENCES SCPT(id)"],
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
            name: "NPC_",
            columns: vec![
                "name TEXT",
                "script TEXT",
                "mesh TEXT",
                "inventory TEXT",
                "spells TEXT",
                "ai_data TEXT",
                "ai_packages TEXT",
                "travel_destinations TEXT",
                "race TEXT",
                "class TEXT",
                "faction TEXT",
                "head TEXT",
                "hair TEXT",
                "npc_flags TEXT",
                "blood_type INTEGER",
                "data_level INTEGER",
                "data_stats TEXT",
                "data_disposition INTEGER",
                "data_reputation INTEGER",
                "data_rank INTEGER",
                "data_gold INTEGER",
            ],
            constraints: vec![
                "FOREIGN KEY(script) REFERENCES SCPT(id)",
                "FOREIGN KEY(class) REFERENCES CLAS(id)",
                "FOREIGN KEY(faction) REFERENCES FACT(id)",
                "FOREIGN KEY(race) REFERENCES RACE(id)",
            ],
        },
        TableSchema {
            name: "ACTI",
            columns: vec!["name TEXT", "script TEXT", "mesh TEXT"],
            constraints: vec!["FOREIGN KEY(script) REFERENCES SCPT(id)"],
        },
        TableSchema {
            name: "LEVI",
            columns: vec![
                "leveled_item_flags TEXT",
                "chance_none INTEGER",
                "items TEXT",
            ],
            constraints: vec![],
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
            db.execute(
                "INSERT INTO 
                GMST 
                (id, mod, value) 
                VALUES 
                (?1, ?2, ?3)",
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

            db.execute(
                "INSERT INTO 
                GLOB 
                (id, mod, value) 
                VALUES 
                (?1, ?2, ?3)",
                params![s.id, hash, value],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Class(s) => {
            db.execute(
                "INSERT INTO 
                CLAS 
                (id, mod, name, description, data) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5)",
                params![s.id, hash, s.name, s.description, as_json!(s.data)],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Faction(s) => {
            db.execute(
                "INSERT INTO 
                FACT 
                (id, mod, name, rank_names, reactions, data) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5, ?6)",
                params![s.id, hash, s.name, as_json!(s.rank_names), as_json!(s.reactions), as_json!(s.data)],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Race(s) => {
            db.execute(
                "INSERT INTO 
                RACE 
                (id, mod, name, spells, description, data) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5, ?6)",
                params![s.id, hash, s.name, as_json!(s.spells), s.description, as_json!(s.data)],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::MiscItem(s) => {
            db.execute(
                "INSERT INTO 
                MISC 
                (id, mod, name, script, mesh, icon, weight, value, flags) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
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
        tes3::esp::TES3Object::Static(s) => {
            db.execute(
                "INSERT INTO 
                STAT 
                (id, mod, mesh) 
                VALUES 
                (?1, ?2, ?3)",
                params![s.id, hash, s.mesh],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Npc(s) => {
           db.execute(
                "INSERT INTO 
                NPC_ 
                (id, mod, name, script, mesh, inventory, spells, ai_data, ai_packages, travel_destinations, 
                race, class, faction, head, hair, npc_flags, blood_type, data_level, data_stats, data_disposition, 
                data_reputation, data_rank, data_gold)
                VALUES 
                (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
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
                "INSERT INTO 
                ACTI 
                (id, mod, name, script, mesh) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5)",
                params![s.id, hash, s.name, as_option!(s.script), s.mesh],
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
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Region(s) => {
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
                    as_json!(s.weather_chances),
                    s.sleep_creature,
                    as_json!(s.sounds)
                ],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::LeveledItem(s) => {
            db.execute(
                "INSERT INTO 
                LEVI 
                (id, mod, leveled_item_flags, chance_none, items) 
                VALUES 
                (?1, ?2, ?3, ?4, ?5)",
                params![s.id, hash, as_json!(s.leveled_item_flags), s.chance_none, as_json!(s.items)],
            )
            .unwrap_or_else(|_| panic!("Could not insert into db {}", s.id));
        }
        tes3::esp::TES3Object::Cell(s) => {
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
