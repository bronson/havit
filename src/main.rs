use chrono::DateTime;
use clap::{App, Arg, ArgMatches, SubCommand};
use humansize::{file_size_opts, FileSize};
use rusqlite::params;
use walkdir::WalkDir;

use std::io::Read;
use std::time::Instant;

// TODO: replace println with writeln? https://github.com/BurntSushi/advent-of-code/issues/17
// TODO: extract prepared statements: https://github.com/rusqlite/rusqlite/blob/b8b1138fcf1ed29d50f5a3f9d94a9719e35146c2/src/statement.rs#L1275
// TODO: add Rayon (or Futures or Tokio or async_std) and Indicatif?

// TODO: the unique index doesn't work if we feed it different relative paths
//    abc/def   vs   ./abc/def   vs   ../abc/def
// Need to normalize the path before storing!

// extern crate libsqlite3_sys;

mod migrations;

// THOUGHT: refinery+barrel get me 80% of what I would use Diesel for, and are way less overbearing.
//   To get 99%, is there any way for Barrel to somehow connect the table type to a struct? Close the loop?

// Find duplicate hashes: `SELECT hash, count(*) c FROM files GROUP BY hash HAVING c > 1 ORDER BY c DESC;`

struct File<'a> {
    _id: i32,
    name: &'a str,
    path: &'a str,
    size: i64, // sqlite forces signed
    // ?? is putting "<chrono::Local>" everywhere really the best technique?
    ctime: DateTime<chrono::Local>,
    mtime: DateTime<chrono::Local>,
    atime: DateTime<chrono::Local>,
    hash: blake3::Hash,
}

static mut VERBOSITY: u64 = 0;

fn verbosity() -> u64 {
    unsafe { VERBOSITY }
}

fn hash_file<P: AsRef<std::path::Path>>(path: P) -> std::io::Result<(blake3::Hash, usize)> {
    let mut hasher = blake3::Hasher::new();
    let mut file = std::fs::File::open(path)?;
    let mut total = 0;

    // Newbie q: it seems strange that we initialize the buffer to 0 and immediately overwrite the zeroes with data.
    // Maybe Rust should have a first-class buffer type that knows how many bytes are in it?
    // That would remove the chance of accessing uninitialized memory without needing to set every byte to 0.
    let mut buffer = [0; 65536];

    loop {
        match file.read(&mut buffer) {
            Ok(0) => return Ok((hasher.finalize(), total)),
            Ok(n) => {
                total += n;
                hasher.update(&buffer[..n]);
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

fn insert_file(conn: &rusqlite::Connection, entry: walkdir::DirEntry) -> usize {
    let metadata = entry.metadata().unwrap();

    // rusqlite can't persist a SystemTime so convert times to Chrono
    // TODO: should store null if any of these values aren't supplied. Get rid of unwrap().
    // TODO: how big a patch would it be to just have rusqlite support SystemTime?
    fn convert(t: std::time::SystemTime) -> chrono::DateTime<chrono::Local> {
        chrono::DateTime::<chrono::Local>::from(t)
    }

    let (hash, size) = hash_file(entry.path()).unwrap();

    let file = File {
        _id: 0,
        name: entry.file_name().to_str().unwrap(),
        path: entry.path().parent().unwrap().to_str().unwrap(),
        size: metadata.len() as i64, // unsigned to signed truncation
        mtime: convert(metadata.modified().unwrap()),
        atime: convert(metadata.accessed().unwrap()),
        ctime: convert(metadata.created().unwrap()),
        hash: hash,
    };

    let result = conn.execute(
        "INSERT INTO files (name, path, size, ctime, mtime, atime, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![file.name, file.path, file.size, file.ctime, file.mtime, file.atime, file.hash.to_hex().as_str()]);

    // Trying to match the actual error, but I just can't get Rust to accept `libsqlite3_sys`.
    //     https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=2fe63f8b28cafa8afe4fefd85c9502c8

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
        Err(_) => panic!("inserting {}/{}: {:#?}", file.path, file.name, result),
    }

    size
}

fn add_entries(conn: &rusqlite::Connection, file: &str) -> Result<usize, std::io::Error> {
    let mut total = 0;
    for entry in WalkDir::new(file).follow_links(true) {
        let entry = entry?;
        if entry.metadata().unwrap().is_file() {
            total += insert_file(&conn, entry);
        }
    }
    Ok(total)
}

fn check_file(conn: &rusqlite::Connection, entry: walkdir::DirEntry) -> usize {
    let path = entry.path().to_str().unwrap();
    let (hash, size) = hash_file(path).unwrap();

    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM files WHERE hash = :hash")
        .unwrap();
    let rows = stmt.query_and_then_named(&[(":hash", &hash.to_hex().as_str())], |row| row.get(0));
    let count: i64 = rows.unwrap().next().unwrap().unwrap();
    println!("{:#?}: {}", count, path);
    size
}

fn check_entries(conn: &rusqlite::Connection, file: &str) -> Result<usize, std::io::Error> {
    let mut total = 0;
    let walker = WalkDir::new(file)
        .follow_links(true)
        .sort_by_file_name()
        .contents_first(true);
    for entry in walker {
        let entry = entry?;
        if entry.metadata().unwrap().is_file() {
            total += check_file(&conn, entry);
        } else {
        }
    }
    Ok(total)
}

fn process_entries<F>(command: &str, matches: &ArgMatches, callback: F) -> Result<usize, std::io::Error>
where
    F: Fn(&str) -> Result<usize, std::io::Error>,
{
    let mut total = 0;
    if let Some(matches) = matches.subcommand_matches(command) {
        match matches.values_of("entries") {
            Some(v) => {
                for el in v {
                    total += callback(el)?;
                }
            }
            _ => total += callback(".")?
        }
    }
    Ok(total)
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("havit")
        .version("0.1")
        .about("Stores a file hierarchy in sqlite.")
        .author("bronson")
        .arg(
            Arg::with_name("database")
                .short("d")
                .long("db")
                .value_name("FILE")
                .default_value("havit.sqlite")
                .help("Specify the database file to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Increase verbosity"),
        )
        .subcommand(
            SubCommand::with_name("add")
                .about("Adds files and directories to the database")
                .arg(
                    Arg::with_name("entries")
                        .help("the dirs/files to add")
                        .multiple(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("check")
                .about("Checks which files are already in the database")
                .arg(
                    Arg::with_name("entries")
                        .help("the dirs/files to check")
                        .multiple(true),
                ),
        )
        .get_matches();

    // TODO: what's a better way to handle verbosity?
    // Passing it as an arg to every function that may need it isn't reasonable
    unsafe { VERBOSITY = matches.occurrences_of("verbose") };
    // if verbosity() > 3 {
    //     println!("MATCHES: {:#?}", matches);
    // }

    let dbfile = matches.value_of("database").unwrap();
    let mut conn = rusqlite::Connection::open(dbfile)?;

    // conn.trace(Some(|s| println!("TRACE: {}", s)));
    // conn.profile(Some(|s,d| println!("PROFILE {:?}: {}", d, s)));

    let report = migrations::runner().run(&mut conn).unwrap();
    let num_migrations = report.applied_migrations().len();
    // TODO: print a nice error if the db is newer (has more migrations) than the app.
    if num_migrations > 0 {
        if verbosity() > 1 {
            println!("MIGRATION STATUS: {:#?}", report);
        } else {
            println!(
                "Applied {} migration{}",
                num_migrations,
                if num_migrations != 1 { "s" } else { "" }
            );
        }
    }

    let mut total = 0;
    let start = Instant::now();

    // inserting in a transaction is 10X faster than one-at-a-time
    let tx = conn.transaction().unwrap();
    total += process_entries("add", &matches, |el| add_entries(&tx, el))?;
    total += process_entries("check", &matches, |el| check_entries(&tx, el))?;
    tx.commit().unwrap();

    let since = Instant::now().duration_since(start);
    eprintln!(
        "it took {:?} to process {:?} bytes ({}/sec)",
        since,
        total,
        ((total as f64 / since.as_secs_f64()) as usize)
            .file_size(file_size_opts::BINARY)
            .unwrap(),
    );

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        if verbosity() > 0 {
            eprintln!("ERROR: {:#?}", err);
        } else {
            eprintln!("{}", err);
        }

        // TODO: should return sensible error codes too
        std::process::exit(1);
    }
}
