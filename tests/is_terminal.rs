#[macro_use]
mod sys_common;

#[cfg(feature = "cap_std_impls")]
use sys_common::io::tmpdir;
use system_interface::io::IsTerminal;

#[test]
#[cfg(feature = "cap_std_impls")]
fn cap_std_file_is_not_terminal() {
    let tmpdir = tmpdir();
    check!(tmpdir.create("file"));
    assert!(!check!(tmpdir.open("file")).is_terminal());
}

#[test]
fn std_file_is_not_terminal() {
    let tmpdir = tempfile::tempdir().unwrap();
    check!(std::fs::File::create(tmpdir.path().join("file")));
    assert!(!check!(std::fs::File::open(tmpdir.path().join("file"))).is_terminal());
}

#[test]
fn stdout_stderr_terminals() {
    assert_eq!(
        std::io::stdout().is_terminal(),
        atty::is(atty::Stream::Stdout)
    );
    assert_eq!(
        std::io::stderr().is_terminal(),
        atty::is(atty::Stream::Stderr)
    );
}
