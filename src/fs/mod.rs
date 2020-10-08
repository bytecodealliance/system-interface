//! Filesystem extension traits.

mod file_io_ext;

pub use file_io_ext::{Advice, FileIoExt};

#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(target_os = "wasi")]
use std::os::wasi::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, FromRawHandle};

// Windows quirks:
//  - Open dir can't be renamed or deleted
//  - symlinks are different
//  - seek past end of file files unspec rather than zero

// TODO: test that remove_dir can remove symlink_dirs, and remove_file files.

// TODO: poll

#[cfg(not(windows))]
pub(crate) unsafe fn as_file(file: &impl AsRawFd) -> std::mem::ManuallyDrop<std::fs::File> {
    std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(file.as_raw_fd()))
}

#[cfg(windows)]
pub(crate) unsafe fn as_file(file: &impl AsRawHandle) -> std::mem::ManuallyDrop<std::fs::File> {
    std::mem::ManuallyDrop::new(std::fs::File::from_raw_handle(file.as_raw_handle()))
}

#[cfg(not(windows))]
pub(crate) fn into_file<Fd: AsRawFd>(file: Fd) -> std::fs::File {
    unsafe { std::fs::File::from_raw_fd(file.as_raw_fd()) }
}

#[cfg(windows)]
pub(crate) fn into_file<Handle: AsRawHandle>(file: Handle) -> std::fs::File {
    unsafe { std::fs::File::from_raw_handle(file.as_raw_handle()) }
}

impl crate::io::ReadReady for std::fs::File {
    #[inline]
    fn num_ready_bytes(&self) -> std::io::Result<u64> {
        let file = unsafe { as_file(self) };
        let (read, _write) = is_read_write(&*file)?;
        if read {
            let metadata = file.metadata()?;
            if metadata.is_file() {
                return Ok(metadata.len());
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "stream is not readable",
        ))
    }
}

#[cfg(not(windows))]
use posish::io::is_read_write;

// TODO: This code is duplicated from cap-std.
#[cfg(windows)]
fn is_read_write(file: &std::fs::File) -> std::io::Result<(bool, bool)> {
    let handle = std::os::windows::io::AsRawHandle::as_raw_handle(file);
    let access_mode = winx::file::query_access_information(handle)?;
    let read = access_mode.contains(winx::file::AccessMode::FILE_READ_DATA);
    let write = access_mode.contains(winx::file::AccessMode::FILE_WRITE_DATA);
    Ok((read, write))
}
