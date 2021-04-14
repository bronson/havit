use barrel::{types, Migration};

pub fn migration() -> String {
    let mut m = Migration::new();

    m.create_table("files", |t| {
        t.add_column("id", types::primary());
        t.add_column("name", types::varchar(255));  // should this be text() too?
        t.add_column("path", types::text());
        t.add_column("size", types::integer());      // sqlite will expand storage up to 8 bytes
        t.add_column("ctime", types::date());
        t.add_column("mtime", types::date());
        t.add_column("atime", types::date());
        t.add_column("hash", types::varchar(64));
    });

    // It seems bad to hard-code the backend into the migration.
    // This might be an answer:
    //   https://docs.rs/barrel/0.5.0-rc.1/barrel/connectors/trait.DatabaseExecutor.html
    m.make::<barrel::backend::Sqlite>()
}
