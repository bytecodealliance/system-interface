//! Filesystem extension traits.

mod file_io_ext;

pub use file_io_ext::{Advice, FileIoExt};

use unsafe_io::AsUnsafeFile;

// Windows quirks:
//  - Open dir can't be renamed or deleted
//  - symlinks are different
//  - seek past end of file files unspec rather than zero

// TODO: test that remove_dir can remove symlink_dirs, and remove_file files.

// TODO: poll

impl crate::io::ReadReady for std::fs::File {
    #[inline]
    fn num_ready_bytes(&self) -> std::io::Result<u64> {
        let file = self.as_file_view();
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
