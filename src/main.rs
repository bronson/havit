use chrono::DateTime;
use rusqlite::params;

mod migrations;

// THOUGHT: refinery+barrel get me 80% of what I would use Diesel for, and are way less overbearing.
//   To get 99%, is there any way for Barrel to somehow connect the table type to a struct? Close the loop?

struct File {
    id: i32,
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
    // TODO: how can I die if the database is too new?
    let report = migrations::runner().run(&mut conn).unwrap();
    println!("{:#?}", report);

    let name = ".gitignore";
    let metadata = std::fs::metadata(name).unwrap();

    fn conv(t: std::time::SystemTime) -> chrono::DateTime::<chrono::Local> {
        chrono::DateTime::<chrono::Local>::from(t)
    }

    let file = File {
        id: 0,
        name: name.to_string(),
        path: ".".to_string(),
        size: metadata.len() as i64, // unsigned to signed truncation
        mtime: conv(metadata.modified().unwrap()),
        atime: conv(metadata.accessed().unwrap()),
        ctime: conv(metadata.created().unwrap()),
        hash: "-".to_string()
    };

    conn.execute(
        "INSERT INTO files (name, path, size, ctime, mtime, atime, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![file.name, file.path, file.size, file.ctime, file.mtime, file.atime, file.hash]).unwrap();

    Ok(())
}
