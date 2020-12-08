#[macro_use]
mod sys_common;

use std::fs::OpenOptions;
use system_interface::fs::FileIoExt;

#[test]
fn allocate() {
    let dir = tempfile::tempdir().unwrap();
    let file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));

    assert_eq!(check!(file.metadata()).len(), 0);

    check!(file.allocate(1024, 1024));

    assert_eq!(check!(file.metadata()).len(), 1024 + 1024);

    check!(file.allocate(1024, 1024));

    assert_eq!(check!(file.metadata()).len(), 1024 + 1024);

    check!(file.allocate(4096, 4096));

    assert_eq!(check!(file.metadata()).len(), 4096 + 4096);
}
