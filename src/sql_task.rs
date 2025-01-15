use std::{collections::HashMap, path::PathBuf};

use fnv_rs::{Fnv64, FnvHasher};
use rusqlite::{params, Connection};
use tes3::esp::{traits::TableSchema, EditorId, SqlInfo, SqlInfoMeta, TypeInfo};

use crate::*;

// todo sql
// check foreign keys in join tables
// check unique constraints in join tables
// todo sql color representation

#[macro_export]
macro_rules! SQL_BEGIN {
    ( $db:expr ) => {
        $db.execute("BEGIN", [])
            .expect("Could not begin transaction");
    };
}

#[macro_export]
macro_rules! SQL_COMMIT {
    ( $db:expr ) => {
        $db.execute("COMMIT", [])
            .expect("Could not commit transaction");
    };
}

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

    log::info!("Found plugins: {:?}", plugin_paths);

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
    {
        log::info!("Create tables");
        create_tables(&db, &get_schemas());
    }

    // create join tables
    {
        log::info!("Create join tables");
        create_join_tables(&db, &get_join_schemas());
    }

    // debug todo
    // for tag in get_all_tags() {
    //     if let Some(instance) = create_from_tag(&tag) {
    //         let txt = instance.table_insert_text();
    //         println!("{}", txt);
    //     }
    // }

    // populate plugins db
    log::info!("Generating plugin db");

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
                Err(e) => log::error!("Could not insert plugin into table {}", e),
            }

            plugins.push((filename, plugin));
        }
    }

    // populate records tables
    log::info!("Generating records db");

    for (name, plugin) in plugins.iter().rev() {
        // group by tag
        let mut groups = HashMap::new();
        for record in &plugin.objects {
            let tag = record.tag_str();
            let group = groups.entry(tag.to_string()).or_insert_with(Vec::new);
            group.push(record);
        }

        SQL_BEGIN!(db);

        for tag in get_all_tags_fk() {
            // skip headers
            if tag == "TES3" {
                continue;
            }

            if let Some(group) = groups.get(&tag) {
                log::info!("Processing records for tag: {}", tag);

                for record in group {
                    match record.table_insert(&db, name) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!(
                                "[{}] Error inserting record '{}': '{}'",
                                record.table_name(),
                                record.editor_id(),
                                e
                            );
                        }
                    }
                }
            }
        }

        SQL_COMMIT!(db);

        SQL_BEGIN!(db);

        for tag in get_all_tags_fk() {
            // skip headers
            if tag == "TES3" {
                continue;
            }

            if let Some(group) = groups.get(&tag) {
                log::info!("Processing join records for tag: {}", tag);

                for record in group {
                    match record.join_table_insert(&db, name) {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!(
                                "[{}] Error inserting join record '{}': '{}'",
                                record.table_name(),
                                record.editor_id(),
                                e
                            );
                        }
                    }
                }
            }
        }

        SQL_COMMIT!(db);

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

    Ok(())
}

fn create_tables(conn: &Connection, schemas: &[TableSchema]) {
    for schema in schemas {
        let columns = schema.columns.join(", ");
        let constraints = schema.constraints.join(", ");

        let sql = if constraints.is_empty() {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                id  TEXT PRIMARY KEY,
                mod TEXT NOT NULL,
                flags TEXT NOT NULL,
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
                flags TEXT NOT NULL,
                {}, 
                FOREIGN KEY(mod) REFERENCES plugins(name),
                {}
                )",
                schema.name, columns, constraints
            )
        };

        log::info!("Creating table {}: {}", schema.name, sql);

        match conn.execute(&sql, []) {
            Ok(_) => {}
            Err(e) => log::error!("Error creating table {}: {}", schema.name, e),
        }
    }
}

fn create_join_tables(conn: &Connection, schemas: &[TableSchema]) {
    for schema in schemas {
        let columns = schema.columns.join(", ");
        let constraints = schema.constraints.join(", ");

        let sql = if constraints.is_empty() {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                mod TEXT NOT NULL,
                {},
                FOREIGN KEY(mod) REFERENCES plugins(name)
                )",
                schema.name, columns
            )
        } else {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                mod TEXT NOT NULL,
                {}, 
                FOREIGN KEY(mod) REFERENCES plugins(name),
                {}
                )",
                schema.name, columns, constraints
            )
        };

        log::info!("Creating table {}: {}", schema.name, sql);

        match conn.execute(&sql, []) {
            Ok(_) => {}
            Err(e) => log::error!("Error creating table {}: {}", schema.name, e),
        }
    }
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

fn get_join_schemas() -> Vec<TableSchema> {
    let mut schemas = Vec::new();
    for x in get_all_join_objects() {
        schemas.push(x.table_schema());
    }

    schemas
}

#[test]
fn test_sql_task() -> std::io::Result<()> {
    init_logger(Path::new("log.txt")).expect("Could not initialize logger");

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
