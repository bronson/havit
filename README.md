# Havit

Stores a file hierarchy in a sqlite database. You can then deploy data science tools on it. Or just use SQL queries.

Mostly this is a project to learn Rust.

## Install

* Install Rustup
* clone this repo
* `cargo build --release`
* Copy `target/release/havit` somewhere on your path.

TODO: maybe `cargo make`?

## Use

* `havit add <file-or-dir>...`
* `havit check <file-or-dir>...`
