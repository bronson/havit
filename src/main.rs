use chrono::DateTime;
use clap::{App, Arg, SubCommand};
use rusqlite::params;
use walkdir::WalkDir;

// TODO: index doesn't work if we feed it different relative paths
//    abc/def   vs   ./abc/def   vs   ../abc/def
// Need to normalize the path before storing!

// extern crate libsqlite3_sys;

mod migrations;

// THOUGHT: refinery+barrel get me 80% of what I would use Diesel for, and are way less overbearing.
//   To get 99%, is there any way for Barrel to somehow connect the table type to a struct? Close the loop?

struct File<'a> {
    _id: i32,
    name: &'a str,
    path: &'a str,
    size: i64, // sqlite forces signed
    // ?? is putting "<chrono::Local>" everywhere really the best technique?
    ctime: DateTime<chrono::Local>,
    mtime: DateTime<chrono::Local>,
    atime: DateTime<chrono::Local>,
    hash: String,
}

fn insert_file(conn: &rusqlite::Connection, entry: walkdir::DirEntry) {
    let metadata = entry.metadata().unwrap();

    // rusqlite can't persist a SystemTime so convert times to Chrono
    // TODO: should store null if any of these values aren't supplied. Get rid of unwrap().
    // TODO: how big a patch would it be to just have rusqlite support SystemTime?
    fn convert(t: std::time::SystemTime) -> chrono::DateTime<chrono::Local> {
        chrono::DateTime::<chrono::Local>::from(t)
    }

    let file = File {
        _id: 0,
        name: entry.file_name().to_str().unwrap(),
        path: entry.path().parent().unwrap().to_str().unwrap(),
        size: metadata.len() as i64, // unsigned to signed truncation
        mtime: convert(metadata.modified().unwrap()),
        atime: convert(metadata.accessed().unwrap()),
        ctime: convert(metadata.created().unwrap()),
        hash: "-".to_string(),
    };

    // Apparently in sqlite, inserting in a transaction runs almost as fast as a bulk insert.
    // That's easier than cobbling together some bulk insert code.

    let result = conn.execute(
        "INSERT INTO files (name, path, size, ctime, mtime, atime, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![file.name, file.path, file.size, file.ctime, file.mtime, file.atime, file.hash]);

    // Trying to match the actual error, but I just can't get Rust to accept `libsqlite3_sys`.

    // let rows = match result {
    //     Ok(rows) => rows,

    //     Err(e @ rusqlite::Error::SqliteFailure(
    //         libsqlite3_sys::Error { extended_code: libsqlite3_sys::SQLITE_CONSTRAINT_UNIQUE, .. }, _),
    //     ) => panic!("constraint violation: {:#?}", e),

    //     Err(rusqlite::Error::SqliteFailure(
    //         libsqlite3_sys::Error { extended_code: libsqlite3_sys::SQLITE_ERROR, .. }, msg),
    //     ) => panic!("no such table: {:#?}", msg.unwrap()),

    //     _ => panic!("some other error: {:#?}", result)
    // };

    match result {
        Ok(1) => (),
        Ok(rows) => panic!(
            "More than one row {} for {}/{} !?  Result: {:#?}",
            rows, file.path, file.name, result
        ),
        Err(_) => panic!("Error inserting {}/{}: {:#?}", file.path, file.name, result),
    }
}

fn add_entries(conn: &rusqlite::Connection, file: &str) {
    for entry in WalkDir::new(file).follow_links(true) {
        let entry = entry.unwrap();
        if entry.metadata().unwrap().is_file() {
            insert_file(&conn, entry);
        }
    }
}

fn main() -> std::io::Result<()> {
    let matches = App::new("havit")
        .version("0.1")
        .about("Stores a file hierarchy in sqlite.")
        .author("bronson")
        .subcommand(
            // init, check
            SubCommand::with_name("add")
                .about("Adds files and directories to the database")
                .arg(
                    Arg::with_name("entries")
                        .help("the dirs/files to add")
                        .multiple(true),
                ),
        )
        .get_matches();

    let mut conn = rusqlite::Connection::open("havit.sqlite").unwrap();

    let report = migrations::runner().run(&mut conn).unwrap();
    // TODO: print a nice error if the db is newer (has more migrations) than the app.
    if report.applied_migrations().len() > 0 {
        println!("{:#?}", report);
    }

    let tx = conn.transaction().unwrap();
    if let Some(matches) = matches.subcommand_matches("add") {
        match matches.values_of("entries") {
            Some(v) => {
                for el in v {
                    add_entries(&tx, el);
                }
            }
            _ => add_entries(&tx, "."),
        }
    }
    tx.commit().unwrap();

    Ok(())
}
