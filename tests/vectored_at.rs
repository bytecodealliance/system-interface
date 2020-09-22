#[macro_use]
mod sys_common;

use std::{
    fs::OpenOptions,
    io::{IoSlice, IoSliceMut},
};
#[cfg(any(not(windows), feature = "cap_std_impls"))]
use sys_common::io::tmpdir;
use system_interface::fs::FileIoExt;

#[cfg(any(not(windows), feature = "cap_std_impls"))]
#[test]
fn cap_read_exact_vectored_at() {
    let tmpdir = tmpdir();
    let mut file = check!(tmpdir.open_with(
        "file",
        cap_std::fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
    ));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    let mut bufs = vec![IoSliceMut::new(&mut buf0), IoSliceMut::new(&mut buf1)];
    check!(file.read_exact_vectored_at(&mut bufs, 4));
    assert_eq!(check!(file.stream_position()), 26);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[test]
fn read_exact_vectored_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    let mut bufs = vec![IoSliceMut::new(&mut buf0), IoSliceMut::new(&mut buf1)];
    check!(file.read_exact_vectored_at(&mut bufs, 4));
    assert_eq!(check!(file.stream_position()), 26);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[test]
fn read_vectored_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    let mut bufs = vec![IoSliceMut::new(&mut buf0), IoSliceMut::new(&mut buf1)];
    check!(file.read_vectored_at(&mut bufs, 4));
    assert_eq!(check!(file.stream_position()), 26);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[test]
fn read_exact_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    check!(file.read_exact_at(&mut buf0, 4));
    check!(file.read_exact_at(&mut buf1, 12));
    assert_eq!(check!(file.stream_position()), 26);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[test]
fn read_exact_vectored() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    let mut bufs = vec![IoSliceMut::new(&mut buf0), IoSliceMut::new(&mut buf1)];
    check!(file.seek(std::io::SeekFrom::Start(4)));
    check!(file.read_exact_vectored(&mut bufs));
    assert_eq!(check!(file.stream_position()), 20);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[test]
fn read_exact() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    check!(file.seek(std::io::SeekFrom::Start(4)));
    check!(file.read_exact(&mut buf0));
    check!(file.read_exact(&mut buf1));
    assert_eq!(check!(file.stream_position()), 20);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[test]
fn read_vectored() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    let mut bufs = vec![IoSliceMut::new(&mut buf0), IoSliceMut::new(&mut buf1)];
    check!(file.seek(std::io::SeekFrom::Start(4)));
    check!(file.read_vectored(&mut bufs));
    assert_eq!(check!(file.stream_position()), 20);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[test]
fn read_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    check!(file.read_at(&mut buf0, 4));
    check!(file.read_at(&mut buf1, 12));
    assert_eq!(check!(file.stream_position()), 26);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[test]
fn read() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let mut buf0 = vec![0; 8];
    let mut buf1 = vec![0; 8];
    check!(file.seek(std::io::SeekFrom::Start(4)));
    check!(file.read(&mut buf0));
    check!(file.read(&mut buf1));
    assert_eq!(check!(file.stream_position()), 20);
    assert_eq!(&buf0, b"efghijkl");
    assert_eq!(&buf1, b"mnopqrst");
}

#[cfg(any(not(windows), feature = "cap_std_impls"))]
#[test]
fn cap_write_all_vectored_at() {
    let tmpdir = tmpdir();
    let mut file = check!(tmpdir.open_with(
        "file",
        cap_std::fs::OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
    ));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    let mut bufs = vec![IoSlice::new(&buf0), IoSlice::new(&buf1)];
    check!(file.write_all_vectored_at(&mut bufs, 4));
    assert_eq!(check!(file.stream_position()), 26);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdEFGHIJKLMNOPQRSTuvwxyz");
}

#[test]
fn write_all_vectored_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    let mut bufs = vec![IoSlice::new(&buf0), IoSlice::new(&buf1)];
    check!(file.write_all_vectored_at(&mut bufs, 4));
    assert_eq!(check!(file.stream_position()), 26);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdEFGHIJKLMNOPQRSTuvwxyz");
}

#[test]
fn write_vectored_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    let mut bufs = vec![IoSlice::new(&buf0), IoSlice::new(&buf1)];
    check!(file.write_vectored_at(&mut bufs, 4));
    assert_eq!(check!(file.stream_position()), 26);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdEFGHIJKLMNOPQRSTuvwxyz");
}

#[test]
fn write_all_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    check!(file.write_all_at(&buf0, 4));
    check!(file.write_all_at(&buf1, 12));
    assert_eq!(check!(file.stream_position()), 26);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdEFGHIJKLMNOPQRSTuvwxyz");
}

#[test]
fn write_all_vectored() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    let mut bufs = vec![IoSlice::new(&buf0), IoSlice::new(&buf1)];
    check!(file.write_all_vectored(&mut bufs));
    assert_eq!(check!(file.stream_position()), 42);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdefghijklmnopqrstuvwxyzEFGHIJKLMNOPQRST");
}

#[test]
fn write_all() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    check!(file.write_all(&buf0));
    check!(file.write_all(&buf1));
    assert_eq!(check!(file.stream_position()), 42);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdefghijklmnopqrstuvwxyzEFGHIJKLMNOPQRST");
}

#[test]
fn write_vectored() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    let mut bufs = vec![IoSlice::new(&buf0), IoSlice::new(&buf1)];
    check!(file.write_all_vectored(&mut bufs));
    assert_eq!(check!(file.stream_position()), 42);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdefghijklmnopqrstuvwxyzEFGHIJKLMNOPQRST");
}

#[test]
fn write_at() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    check!(file.write_at(&buf0, 4));
    check!(file.write_at(&buf1, 12));
    assert_eq!(check!(file.stream_position()), 26);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdEFGHIJKLMNOPQRSTuvwxyz");
}

#[test]
fn write() {
    let dir = tempfile::tempdir().unwrap();
    let mut file = check!(OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(dir.path().join("file")));
    check!(write!(&mut file, "abcdefghijklmnopqrstuvwxyz"));
    let buf0 = b"EFGHIJKL".to_vec();
    let buf1 = b"MNOPQRST".to_vec();
    check!(file.write(&buf0));
    check!(file.write(&buf1));
    assert_eq!(check!(file.stream_position()), 42);
    let mut back = String::new();
    check!(file.seek(std::io::SeekFrom::Start(0)));
    check!(file.read_to_string(&mut back));
    assert_eq!(back, "abcdefghijklmnopqrstuvwxyzEFGHIJKLMNOPQRST");
}
