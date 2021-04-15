use chrono::DateTime;
use rusqlite::params;

// extern crate libsqlite3_sys;

mod migrations;

// THOUGHT: refinery+barrel get me 80% of what I would use Diesel for, and are way less overbearing.
//   To get 99%, is there any way for Barrel to somehow connect the table type to a struct? Close the loop?

struct File {
    _id: i32,
    name: String,
    path: String,
    size: i64, // sqlite forces signed
    ctime: DateTime::<chrono::Local>,  // ?? is putting "<chrono::Local>" everywhere really the best technique?
    mtime: DateTime::<chrono::Local>,
    atime: DateTime::<chrono::Local>,
    hash: String
}

fn main() -> std::io::Result<()> {
    let mut conn = rusqlite::Connection::open("havit.sqlite").unwrap();

    // ensure database is fully migrated (usually a no-op)
    // TODO: how can I die if the database is too new? Answer: I think Refinery already does this!
    let report = migrations::runner().run(&mut conn).unwrap();
    println!("{:#?}", report);

    let name = ".gitignore";
    let metadata = std::fs::metadata(name).unwrap();

    // rusqlite can't persist a SystemTime so use Chrono
    // TODO: should store null if any of these values aren't supplied. Get rid of unwrap().
    // TODO: how big a patch would it be to have rusqlite support SystemTime?
    fn convert(t: std::time::SystemTime) -> chrono::DateTime::<chrono::Local> {
        chrono::DateTime::<chrono::Local>::from(t)
    }

    let file = File {
        _id: 0,
        name: name.to_string(),
        path: ".".to_string(),
        size: metadata.len() as i64, // unsigned to signed truncation
        mtime: convert(metadata.modified().unwrap()),
        atime: convert(metadata.accessed().unwrap()),
        ctime: convert(metadata.created().unwrap()),
        hash: "-".to_string()
    };

    let result = conn.execute(
        "INSERT INTO files (name, path, size, ctime, mtime, atime, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![file.name, file.path, file.size, file.ctime, file.mtime, file.atime, file.hash]);

    // Trying to match the actual error, but can't get Rust to evalute libsqlite3_sys.

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
        Ok(rows) => panic!("More than one row {} for {}/{} !?  Result: {:#?}", rows, file.path, file.name, result),
        Err(_) => panic!("Error inserting {}/{}: {:#?}", file.path, file.name, result)
    }

    Ok(())
}
