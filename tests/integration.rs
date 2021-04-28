// TODO?: use the ctor crate to run all these tests in one tmpdir? (save all the setup & teardown?)
//        https://stackoverflow.com/questions/58006033/how-to-run-setup-code-before-any-tests-run-in-rust

mod proper_error_messages {
    use assert_cmd::Command;
    use predicates::prelude::predicate;

    fn command() -> Command {
        Command::cargo_bin("havit").unwrap()
    }

    #[test]
    fn test_no_subcommand() {
        command().arg("not-a-cmd")
            .assert().failure().code(1)
            .stderr(predicate::str::starts_with(
                // TODO?: maybe improve this error message
                "error: Found argument 'not-a-cmd' which wasn't expected, or isn't valid in this context\n"
            ));
    }

    #[test]
    fn test_missing_database() {
        // `add nofile` is probably unnecessary but ensures we don't error on missing args first
        command().arg("--db=nodir/noexisty").arg("add").arg("nofile")
            .assert().failure().code(1)
            .stderr("unable to open database file: nodir/noexisty\n");
    }

    #[test]
    fn test_missing_file_for_add() {
        let tmp = assert_fs::TempDir::new().unwrap();
        assert!(std::env::set_current_dir(tmp.path()).is_ok());
        command().arg("add").arg("nofile")
            .assert().failure().code(1)
            // TODO: definitely improve this error message
            .stderr("IO error for operation on nofile: No such file or directory (os error 2)\n");
    }

    #[test]
    fn test_missing_file_for_check() {
        let tmp = assert_fs::TempDir::new().unwrap();
        assert!(std::env::set_current_dir(tmp.path()).is_ok());
        command().arg("check").arg("nofile")
            .assert().failure().code(1)
            // TODO: definitely improve this error message
            .stderr("IO error for operation on nofile: No such file or directory (os error 2)\n");
    }
}
