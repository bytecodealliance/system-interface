//! Filesystem extension traits.

mod fd_flags;
mod file_io_ext;

use crate::io::IsReadWrite;

pub use fd_flags::{FdFlags, GetSetFdFlags, SetFdFlags};
pub use file_io_ext::{Advice, FileIoExt};

// Windows quirks:
//  - Open dir can't be renamed or deleted
//  - symlinks are different
//  - seek past end of file files unspec rather than zero

// TODO: test that remove_dir can remove symlink_dirs, and remove_file files.

// TODO: poll

impl crate::io::ReadReady for std::fs::File {
    #[inline]
    fn num_ready_bytes(&self) -> std::io::Result<u64> {
        let (read, _write) = self.is_read_write()?;
        if read {
            let metadata = self.metadata()?;
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
