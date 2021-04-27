mod proper_error_messages {
    use assert_cmd::Command;
    use predicates::prelude::predicate;

    #[test]
    fn test_no_subcommand() {
        let mut cmd = Command::cargo_bin("havit").unwrap();
        cmd.arg("not-a-cmd")
            .assert().failure().code(1)
            .stderr(predicate::str::starts_with(
                "error: Found argument 'not-a-cmd' which wasn't expected, or isn't valid in this context\n"
            ));
    }

    #[test]
    fn test_missing_database() {
        let mut cmd = Command::cargo_bin("havit").unwrap();
        cmd.arg("--db=nodir/noexisty").arg("add").arg("nofile")
            .assert().failure().code(1)
            .stderr("unable to open database file: nodir/noexisty\n");
    }
}

// mod integration {
//     #[test]
//     fn maybe_it_works() {
//         assert_eq!("hi", "hiya");
//     }
// }
