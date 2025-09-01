use std::{collections::HashMap, path::PathBuf};

use fnv_rs::{Fnv64, FnvHasher};
use rusqlite::{params, Connection};
use tes3::esp::{
    traits::JoinTableSchema, traits::TableSchema, EditorId, SqlInfo, SqlInfoMeta, TypeInfo,
};

use crate::*;

struct PluginModel {
    name: String,
    crc: String,
    load_order: u32,
}

// ------------------------------
// Helpers
// ------------------------------
fn set_fast_pragmas(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Build-only settings (we're in-memory, so durability doesn’t matter)
    conn.pragma_update(None, "journal_mode", "OFF")?;
    conn.pragma_update(None, "synchronous", "OFF")?;
    conn.pragma_update(None, "foreign_keys", "OFF")?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "cache_size", -300_000i64)?;
    conn.pragma_update(None, "locking_mode", "EXCLUSIVE")?;
    conn.pragma_update(None, "mmap_size", 1_073_741_824i64)?;
    Ok(())
}

pub fn sql_task(input: &Option<PathBuf>, output: &Option<PathBuf>) -> std::io::Result<()> {
    // get current working directory
    let mut inputpath = PathBuf::from("./");

    if let Some(input) = input {
        inputpath = input.clone();
    }

    // if input is a directory, process all files
    // else process single file
    let plugin_paths = if inputpath.is_file() {
        vec![inputpath]
    } else {
        get_plugins_sorted(&inputpath, false)
    };

    log::info!("Found plugins: {:?}", plugin_paths);

    let mut output_path = PathBuf::from("./tes3.db3");
    if let Some(output) = output {
        output_path = output.clone();
    }

    if output_path.is_dir() {
        output_path.push("tes3.db3");
    }

    // delete db if exists
    if output_path.exists() {
        std::fs::remove_file(&output_path).expect("Could not delete file");
    }

    // A) Build in-memory (fast) ↓
    let mut mem = Connection::open_in_memory().expect("Could not open in-memory database");
    set_fast_pragmas(&mem).expect("Could not set fast pragmas");

    // --------------------------------------------------------------------------
    // A) create tables first
    let mut schema_tables = String::new();

    // create plugins db
    schema_tables.push_str(
        "CREATE TABLE _plugins (
            name TEXT PRIMARY KEY,
            crc INTEGER NOT NULL,
            load_order INTEGER NOT NULL
        );\n\n",
    );

    // create tables
    {
        log::info!("Create tables");
        create_tables(&mut schema_tables, &get_schemas());
    }

    // create join tables
    {
        log::info!("Create join tables");
        create_join_tables(&mut schema_tables, &get_join_schemas());
    }

    mem.execute_batch(&schema_tables)
        .expect("Could not create tables");

    // --------------------------------------------------------------------------
    // V Bulk-insert in ONE EXCLUSIVE TX
    {
        let mut tx = mem
            .transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)
            .expect("Could not begin transaction");

        // populate plugins db
        let mut plugins = Vec::new();
        {
            log::info!("Generating plugin db");

            let mut s = tx
                .prepare_cached("INSERT INTO _plugins (name, crc, load_order) VALUES (?1, ?2, ?3)")
                .expect("Could not prepare plugins statement");

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
                    match s.execute(params![
                        plugin_model.name,
                        plugin_model.crc,
                        plugin_model.load_order
                    ]) {
                        Ok(_) => {}
                        Err(e) => log::error!("Could not insert plugin into table {}", e),
                    }

                    plugins.push((filename, plugin));
                }
            }
        }

        // populate records tables
        {
            log::info!("Generating records db");

            for (name, plugin) in plugins.iter() {
                log::info!("> Processing plugin: {}", name);

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
                        log::debug!("Processing records for tag: {}", tag);

                        // prepare cached schema
                        let insert_schema_for_tag =
                            create_from_tag(&tag).unwrap().get_insert_schema();
                        let mut s = tx.prepare_cached(&insert_schema_for_tag).unwrap();

                        // insert records
                        for record in group {
                            match record.insert_sql_record(name, &mut s) {
                                Ok(_) => {}
                                Err(e) => {
                                    log::error!(
                                        "[{}] Error inserting {} record '{}': '{}'",
                                        name,
                                        record.table_name(),
                                        record.editor_id(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }

                for tag in get_all_tags_fk() {
                    // skip headers
                    if tag == "TES3" {
                        continue;
                    }

                    if let Some(group) = groups.get(&tag) {
                        log::debug!("Processing join records for tag: {}", tag);

                        for record in group {
                            match record.insert_join_sql_record(name, &mut tx) {
                                Ok(_) => {}
                                Err(e) => {
                                    log::error!(
                                        "[{}] Error inserting {} join record '{}': '{}'",
                                        name,
                                        record.table_name(),
                                        record.editor_id(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        log::info!("Done processing plugins");

        log::info!("Committing transaction");
        tx.commit().expect("Could not commit transaction");
    }

    // --------------------------------------------------------------------------
    // C) Validate FKs once (if you keep FKs in schema)
    mem.pragma_update(None, "foreign_keys", "ON").unwrap();

    // E) Compact & persist to file
    mem.execute(&format!("VACUUM INTO '{}';", output_path.display()), [])
        .expect("Failed to vacuum database");

    // F) Open final DB and set read-friendly pragmas
    let final_db = Connection::open(output_path).unwrap();
    final_db.pragma_update(None, "journal_mode", "WAL").unwrap();
    final_db
        .pragma_update(None, "synchronous", "NORMAL")
        .unwrap();

    // wait for user input
    log::info!("Done. Press Enter to exit.");
    std::io::stdin().read_line(&mut String::new()).unwrap();

    Ok(())
}

fn create_tables(schema_tables: &mut String, schemas: &[TableSchema]) {
    for schema in schemas {
        let columns = schema.columns.join(", ");
        let constraints = schema.constraints.join(", ");

        let sql = if constraints.is_empty() {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                id  TEXT COLLATE NOCASE PRIMARY KEY,
                mod TEXT NOT NULL,
                flags TEXT NOT NULL,
                {},
                FOREIGN KEY(mod) REFERENCES _plugins(name)
                );\n\n",
                schema.name, columns
            )
        } else {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                id  TEXT COLLATE NOCASE PRIMARY KEY,
                mod TEXT NOT NULL,
                flags TEXT NOT NULL,
                {}, 
                FOREIGN KEY(mod) REFERENCES _plugins(name),
                {}
                );\n\n",
                schema.name, columns, constraints
            )
        };

        log::debug!("Creating table {}: {}", schema.name, sql);
        schema_tables.push_str(&sql);
    }
}

fn create_join_tables(schema_tables: &mut String, schemas: &[JoinTableSchema]) {
    for schema in schemas {
        let columns = schema.columns.join(", ");
        let constraints = schema.constraints.join(", ");
        let parents = schema.parent_constraints.join(", ");
        let final_constraints = format!("{} {}", constraints, parents);

        let sql = if final_constraints.is_empty() {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                mod TEXT NOT NULL,
                {},
                FOREIGN KEY(mod) REFERENCES _plugins(name)
                );\n\n",
                schema.name, columns
            )
        } else {
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                mod TEXT NOT NULL,
                {}, 
                FOREIGN KEY(mod) REFERENCES _plugins(name),
                {}
                );\n\n",
                schema.name, columns, final_constraints
            )
        };

        log::debug!("Creating table {}: {}", schema.name, sql);
        schema_tables.push_str(&sql);
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

fn get_join_schemas() -> Vec<JoinTableSchema> {
    let mut schemas = Vec::new();
    for x in get_all_join_objects() {
        schemas.push(x.table_schema());
    }

    schemas
}

#[test]
fn test_sql_task() -> std::io::Result<()> {
    init_logger(Path::new("log.txt"), log::LevelFilter::Debug)
        .expect("Could not initialize logger");

    let input = std::path::Path::new("D:\\games\\Morrowind2\\Data Files");
    let output = std::path::Path::new("./tes3.db3");
    // delete db if exists
    if output.exists() {
        std::fs::remove_file(output).expect("Could not delete file");
    }

    sql_task(&Some(input.into()), &Some(output.into()))
}
#[test]
fn test_graph() {
    init_logger(Path::new("log.txt"), log::LevelFilter::Info).expect("Could not initialize logger");

    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();

    // records
    for tag in get_all_tags() {
        if let Some(instance) = create_from_tag(&tag) {
            edges.entry(instance.table_name()).or_default();

            // get foreign keys
            let fks = instance.table_constraints();

            // split to get table name
            for fk in fks {
                let splits = fk.split("REFERENCES").collect::<Vec<&str>>();
                let target_with_id = splits[1].trim();
                let target_table = target_with_id.split("(").collect::<Vec<&str>>()[0].trim();
                // add edge
                let edge = edges.entry(instance.table_name()).or_default();
                edge.push(target_table);
            }
        }
    }

    // join tables
    for instance in get_all_join_objects() {
        edges.entry(instance.table_name()).or_default();

        let fks = instance.table_constraints();
        for fk in fks {
            // split to get table name
            let splits = fk.split("REFERENCES").collect::<Vec<&str>>();
            let target_with_id = splits[1].trim();
            let target_table = target_with_id.split("(").collect::<Vec<&str>>()[0].trim();
            // add edge
            let edge = edges.entry(instance.table_name()).or_default();
            edge.push(target_table);
        }

        let parents = instance.table_parent_constraints();
        for parent in parents {
            // split to get table name
            let splits = parent.split("REFERENCES").collect::<Vec<&str>>();
            let target_with_id = splits[1].trim();
            let target_table = target_with_id.split("(").collect::<Vec<&str>>()[0].trim();
            // add reversed edge
            let edge = edges.entry(target_table).or_default();
            edge.push(instance.table_name());
        }
    }

    // create graphviz file
    let mut file = std::fs::File::create("graph.dot").expect("Could not create file");
    file.write_all(b"digraph G {\n")
        .expect("Could not write to file");

    for (k, v) in edges.iter() {
        let mut targets = String::new();
        for t in v {
            targets.push_str(t);
            targets.push(' ');
        }
        let line = format!("{} -> {{{}}}\n", k, targets);

        file.write_all(line.as_bytes())
            .expect("Could not write to file");
    }

    file.write_all(b"}").expect("Could not write to file");

    // run graphviz
    // dot -Tpng graph.dot -o graph.png

    use std::process::Command;
    let layouts = vec!["dot", "fdp"];
    for layout in layouts {
        let filename = format!("graph_{}.png", layout);
        let layout_command = format!("-K{}", layout);

        let _output = Command::new("dot")
            .arg(layout_command)
            .arg("-Tpng")
            .arg("graph.dot")
            .arg("-o")
            .arg(filename)
            .output()
            .expect("Could not run dot");
    }
}
