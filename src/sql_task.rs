use crate::get_all_tags;
use crate::get_all_tags_fk;
use crate::get_plugins_sorted;
use fnv_rs::{Fnv64, FnvHasher};
use rusqlite::{params, Connection, Result};
use std::{collections::HashMap, path::PathBuf};
use tes3::esp::traits::TableSchema;
use tes3::esp::EditorId;
use tes3::esp::SqlInfo;
use tes3::esp::TypeInfo;

use crate::create_from_tag;
use crate::parse_plugin;

struct PluginModel {
    name: String,
    crc: String,
    load_order: u32,
}

pub fn sql_task(input: &Option<PathBuf>, output: &Option<PathBuf>) -> std::io::Result<()> {
    let mut inputpath = PathBuf::new();
    if let Some(input) = input {
        inputpath = input.clone();
    }

    // if input is a directory, process all files
    // else process single file
    let plugin_paths = if inputpath.is_file() {
        vec![inputpath]
    } else if inputpath.is_dir() {
        get_plugins_sorted(&inputpath, false)
    } else {
        panic!("Invalid input path");
    };

    println!("Found plugins: {:?}", plugin_paths);

    let mut outputpath = PathBuf::from("./tes3.db3");
    if let Some(output) = output {
        outputpath = output.clone();
    }

    if outputpath.is_dir() {
        outputpath.push("tes3.db3");
    }

    // create esp db
    let db = Connection::open(outputpath).expect("Could not create db");

    // create plugins db
    db.execute(
        "CREATE TABLE plugins (
            name TEXT PRIMARY KEY,
            crc INTEGER NOT NULL,
            load_order INTEGER NOT NULL
        )",
        (), // empty list of parameters.
    )
    .expect("Could not create table");

    // create tables
    let schemas = get_schemas();
    match create_tables(&db, &schemas) {
        Ok(_) => {}
        Err(e) => {
            println!("Error creating tables: {}", e);
        }
    }

    // debug todo
    // for tag in get_all_tags() {
    //     if let Some(instance) = create_from_tag(&tag) {
    //         let txt = instance.table_insert_text();
    //         println!("{}", txt);
    //     }
    // }

    // populate plugins db
    println!("Generating plugin db");

    let mut plugins = Vec::new();
    for path in plugin_paths.iter() {
        if let Ok(plugin) = parse_plugin(path) {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let crc = Fnv64::hash(filename.as_bytes()).as_hex();

            let plugin_model = PluginModel {
                name: filename.to_string(),
                crc: crc.to_owned(), // todo
                load_order: 0,       // todo
            };

            // add plugin to db
            match db.execute(
                "INSERT INTO plugins (name, crc, load_order) VALUES (?1, ?2, ?3)",
                params![plugin_model.name, plugin_model.crc, plugin_model.load_order],
            ) {
                Ok(_) => {}
                Err(e) => println!("Could not insert plugin into table {}", e),
            }

            plugins.push((filename, plugin));
        }
    }

    // populate records tables
    println!("Generating records db");

    let mut errors = Vec::new();

    for (name, plugin) in plugins.iter().rev() {
        // group by tag
        let mut groups = HashMap::new();
        for record in &plugin.objects {
            let tag = record.tag_str();
            let group = groups.entry(tag.to_string()).or_insert_with(Vec::new);
            group.push(record);
        }

        for tag in get_all_tags_fk() {
            // skip headers
            if tag == "TES3" {
                continue;
            }

            if let Some(group) = groups.get(&tag) {
                println!("Processing tag: {}", tag);
                db.execute("BEGIN", [])
                    .expect("Could not begin transaction");

                for record in group {
                    match record.table_insert(&db, name) {
                        Ok(_) => {}
                        Err(e) => {
                            let error_msg = format!(
                                "[{}] Error inserting record '{}': '{}'",
                                record.table_name(),
                                record.editor_id(),
                                e
                            );
                            println!("{}", error_msg);
                            errors.push(error_msg);
                        }
                    }
                }

                db.execute("COMMIT", [])
                    .expect("Could not commit transaction");
            }
        }

        // db.execute("BEGIN", [])
        //     .expect("Could not begin transaction");
        // for tag in get_all_tags_deferred() {
        //     if let Some(group) = groups.get(&tag) {
        //         println!("Processing tag: {}", tag);

        //         for record in group {
        //             match record.table_insert(&db, name) {
        //                 Ok(_) => {}
        //                 Err(e) => {
        //                     let error_msg = format!(
        //                         "[{}] Error inserting record '{}': '{}'",
        //                         record.table_name(),
        //                         record.editor_id(),
        //                         e
        //                     );
        //                     println!("{}", error_msg);
        //                     errors.push(error_msg);
        //                 }
        //             }
        //         }
        //     }
        // }
        // db.execute("COMMIT", [])
        //     .expect("Could not commit transaction");
    }

    // serialize errors to file
    if !errors.is_empty() {
        let mut file = std::fs::File::create("errors.txt").expect("Could not create file");
        for error in errors {
            std::io::Write::write_all(&mut file, error.as_bytes()).unwrap();
            std::io::Write::write_all(&mut file, b"\n").unwrap();
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
                FOREIGN KEY(mod) REFERENCES plugins(name)
                )",
                schema.name, columns
            )
        } else {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                id  TEXT PRIMARY KEY,
                mod TEXT NOT NULL,
                {}, 
                FOREIGN KEY(mod) REFERENCES plugins(name),
                {}
                )",
                schema.name, columns, constraints
            )
        };

        //println!("{}", sql);

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

#[test]
fn test_sql_task() -> std::io::Result<()> {
    let input = std::path::Path::new("tests/assets/Morrowind.esm");
    let output = std::path::Path::new("./tes3.db3");
    // delete db if exists
    if output.exists() {
        std::fs::remove_file(output).expect("Could not delete file");
    }

    sql_task(&Some(input.into()), &Some(output.into()))
}

// testing
// .\tes3util.exe sql d:\GitHub\__rfuzzo\tes3util\tests\assets\Morrowind.esm -o D:\GitHub\__rfuzzo\tes3util\tes3.db3
