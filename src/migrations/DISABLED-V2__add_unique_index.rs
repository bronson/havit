// Disable the unique index, maybe forever.
// It doubles the size of the database and prevents some potentially useful use of duplicates.
// Maybe we should warn on duplicates, or have a check-for-dupes-in-db command?

// use barrel::{types, Migration};

pub fn migration() -> String {
    // let mut migration = Migration::new();
    // migration.change_table("files", |t| {
    //     t.add_index("unique_fullpath", types::index(vec!["name", "path"]).unique(true));
    // });
    // migration.make::<barrel::backend::Sqlite>()

    // hard-code the SQL until this is fixed: https://github.com/rust-db/barrel/issues/92
    "CREATE UNIQUE INDEX \"unique_fullpath\" ON \"files\" (\"name\", \"path\");".to_string()
}
