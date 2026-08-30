use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use clap::Parser;
use croc::cli::Croc;
use croc::commands::CrocCommands;
use croc::commands::hash_object::{HashObjectError, HashObjectType};
use tempfile::TempDir;

/// SHA-1 of the git object `blob 11\0hello world`.
const HELLO_WORLD_OID: &str = "95d09f2b10159347eece71399a7e2e907ea3df4f";
/// SHA-1 of the git object `blob 0\0`.
const EMPTY_OID: &str = "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391";

fn parse(args: &[&str]) -> Result<Croc, clap::Error> {
    Croc::try_parse_from(std::iter::once("croc").chain(args.iter().copied()))
}

fn hash_object_cmd(args: &[&str]) -> croc::commands::hash_object::HashObjectCommand {
    let CrocCommands::HashObject(cmd) = parse(args).unwrap().command else {
        panic!("expected a HashObject subcommand");
    };
    cmd
}

/// Parse and dispatch in-process; used for asserting on typed errors.
fn run_croc(args: &[&str]) -> anyhow::Result<()> {
    croc::run(parse(args)?)
}

/// Run the real binary with `dir` as the working directory.
fn croc_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_croc"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("failed to run croc")
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout was not utf-8")
}

/// A temp dir containing `file.txt` with the given contents.
fn fixture(contents: &[u8]) -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("file.txt");
    fs::write(&file, contents).unwrap();
    (tmp, file)
}

// ---------------------------------------------------------------- parsing

#[test]
fn file_argument_is_positional() {
    let cmd = hash_object_cmd(&["hash-object", "file.txt"]);
    assert_eq!(cmd.file, PathBuf::from("file.txt"));
}

#[test]
fn type_defaults_to_blob() {
    let cmd = hash_object_cmd(&["hash-object", "file.txt"]);
    assert!(matches!(cmd.kind, HashObjectType::Blob));
}

#[test]
fn type_accepts_short_and_long_flags() {
    for args in [
        ["hash-object", "-t", "tree", "file.txt"],
        ["hash-object", "--type", "tree", "file.txt"],
    ] {
        let cmd = hash_object_cmd(&args);
        assert!(matches!(cmd.kind, HashObjectType::Tree));
    }
}

#[test]
fn write_flag_defaults_off_and_can_be_set() {
    assert!(!hash_object_cmd(&["hash-object", "file.txt"]).write);
    assert!(hash_object_cmd(&["hash-object", "-w", "file.txt"]).write);
}

#[test]
fn file_argument_is_required() {
    let err = parse(&["hash-object"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
}

#[test]
fn unknown_object_type_is_rejected() {
    let err = parse(&["hash-object", "-t", "banana", "file.txt"]).unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
}

#[test]
fn subcommand_is_spelled_with_a_dash() {
    assert!(parse(&["hash-object", "file.txt"]).is_ok());
    assert!(parse(&["hash_object", "file.txt"]).is_err());
}

// ----------------------------------------------------------- error paths

#[test]
fn missing_file_reports_file_does_not_exist() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("nope.txt");

    let err = run_croc(&["hash-object", missing.to_str().unwrap()]).unwrap_err();

    let Some(HashObjectError::FileDoesNotExist(path)) = err.downcast_ref::<HashObjectError>()
    else {
        panic!("expected FileDoesNotExist, got: {err:?}");
    };
    assert_eq!(path, &missing);
}

#[test]
fn unreadable_path_is_not_reported_as_missing() {
    // Reading a directory fails with something other than NotFound; it must land
    // in the Io variant so the real cause survives.
    let tmp = TempDir::new().unwrap();

    let err = run_croc(&["hash-object", tmp.path().to_str().unwrap()]).unwrap_err();

    let Some(HashObjectError::Io { path, .. }) = err.downcast_ref::<HashObjectError>() else {
        panic!("expected Io, got: {err:?}");
    };
    assert_eq!(path, tmp.path());
}

#[test]
fn error_message_names_the_file() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("nope.txt");

    let err = run_croc(&["hash-object", missing.to_str().unwrap()]).unwrap_err();

    assert!(
        err.to_string().contains("nope.txt"),
        "message should name the file, got: {err}"
    );
}

#[test]
fn unimplemented_types_error_instead_of_panicking() {
    let (_tmp, file) = fixture(b"hello world");

    for kind in ["tree", "commit", "tag"] {
        let err = run_croc(&["hash-object", "-t", kind, file.to_str().unwrap()]).unwrap_err();
        assert!(
            matches!(
                err.downcast_ref::<HashObjectError>(),
                Some(HashObjectError::Unimplemented(_))
            ),
            "expected Unimplemented for -t {kind}, got: {err:?}"
        );
    }
}

#[test]
fn failures_exit_nonzero() {
    let tmp = TempDir::new().unwrap();
    let out = croc_in(tmp.path(), &["hash-object", "nope.txt"]);

    assert!(!out.status.success());
    assert!(out.stdout.is_empty(), "nothing should be printed on failure");
}

// -------------------------------------------------------------- hashing
// These describe git-compatible behaviour: the hash covers the object
// header (`blob <len>\0`), not just the file contents.

#[test]
fn hashes_the_object_not_the_raw_contents() {
    let (tmp, file) = fixture(b"hello world");
    let out = croc_in(tmp.path(), &["hash-object", file.to_str().unwrap()]);

    assert!(out.status.success());
    assert_eq!(stdout_of(&out).trim(), HELLO_WORLD_OID);
}

#[test]
fn hashes_an_empty_file() {
    let (tmp, file) = fixture(b"");
    let out = croc_in(tmp.path(), &["hash-object", file.to_str().unwrap()]);

    assert_eq!(stdout_of(&out).trim(), EMPTY_OID);
}

#[test]
fn matches_real_git() {
    let (tmp, file) = fixture(b"hello world");

    let ours = croc_in(tmp.path(), &["hash-object", file.to_str().unwrap()]);
    let theirs = Command::new("git")
        .current_dir(tmp.path())
        .args(["hash-object", file.to_str().unwrap()])
        .output()
        .expect("git must be installed for this test");

    assert_eq!(stdout_of(&ours).trim(), stdout_of(&theirs).trim());
}

#[test]
fn prints_forty_lowercase_hex_characters() {
    let (tmp, file) = fixture(b"anything");
    let out = croc_in(tmp.path(), &["hash-object", file.to_str().unwrap()]);
    let oid = stdout_of(&out);

    assert!(oid.ends_with('\n'), "output should end with a newline");
    let oid = oid.trim();
    assert_eq!(oid.len(), 40);
    assert!(oid.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
}

#[test]
fn identical_contents_hash_the_same_under_different_names() {
    let tmp = TempDir::new().unwrap();
    let a = tmp.path().join("a.txt");
    let b = tmp.path().join("nested_name_b.txt");
    fs::write(&a, b"same bytes").unwrap();
    fs::write(&b, b"same bytes").unwrap();

    let oid_a = stdout_of(&croc_in(tmp.path(), &["hash-object", a.to_str().unwrap()]));
    let oid_b = stdout_of(&croc_in(tmp.path(), &["hash-object", b.to_str().unwrap()]));

    assert_eq!(oid_a.trim(), oid_b.trim());
    assert!(!oid_a.trim().is_empty());
}

#[test]
fn handles_non_utf8_contents() {
    let (tmp, file) = fixture(&[0x00, 0xff, 0xfe, 0x80, 0x0a]);
    let out = croc_in(tmp.path(), &["hash-object", file.to_str().unwrap()]);

    assert!(out.status.success(), "binary files must hash without error");
    assert_eq!(stdout_of(&out).trim().len(), 40);
}

#[test]
fn trailing_newline_changes_the_hash() {
    let (tmp_a, a) = fixture(b"hello world");
    let (tmp_b, b) = fixture(b"hello world\n");

    let oid_a = stdout_of(&croc_in(tmp_a.path(), &["hash-object", a.to_str().unwrap()]));
    let oid_b = stdout_of(&croc_in(tmp_b.path(), &["hash-object", b.to_str().unwrap()]));

    assert_ne!(oid_a.trim(), oid_b.trim());
}

// ---------------------------------------------------------------- -w

#[test]
fn write_stores_a_loose_object_at_the_sharded_path() {
    let (tmp, file) = fixture(b"hello world");
    croc_in(tmp.path(), &["init"]);

    let out = croc_in(tmp.path(), &["hash-object", "-w", file.to_str().unwrap()]);
    assert!(out.status.success());

    let (dir, rest) = HELLO_WORLD_OID.split_at(2);
    let object = tmp.path().join(".croc").join("objects").join(dir).join(rest);

    assert!(object.is_file(), "expected an object at {}", object.display());
    assert!(!fs::read(&object).unwrap().is_empty());
}

#[test]
fn writing_the_same_object_twice_is_idempotent() {
    let (tmp, file) = fixture(b"hello world");
    croc_in(tmp.path(), &["init"]);

    let first = croc_in(tmp.path(), &["hash-object", "-w", file.to_str().unwrap()]);
    let second = croc_in(tmp.path(), &["hash-object", "-w", file.to_str().unwrap()]);

    assert!(second.status.success(), "re-writing an object must not fail");
    assert_eq!(stdout_of(&first).trim(), HELLO_WORLD_OID);
    assert_eq!(stdout_of(&second).trim(), HELLO_WORLD_OID);
}

#[test]
fn without_write_nothing_is_stored() {
    let (tmp, file) = fixture(b"hello world");
    croc_in(tmp.path(), &["init"]);

    croc_in(tmp.path(), &["hash-object", file.to_str().unwrap()]);

    let objects = tmp.path().join(".croc").join("objects");
    let entries = fs::read_dir(&objects).unwrap().count();
    assert_eq!(entries, 0, "hash-object without -w must not write anything");
}
