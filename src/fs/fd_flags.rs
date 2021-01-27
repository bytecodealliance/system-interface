use bitflags::bitflags;
#[cfg(not(any(windows, target_os = "redox")))]
use posish::fs::{getfl, setfl, OFlags};
use std::io;
use unsafe_io::AsUnsafeFile;
#[cfg(windows)]
use {
    std::os::windows::io::AsRawHandle,
    unsafe_io::{FromUnsafeFile, UnsafeFile},
    winapi::um::winbase::FILE_FLAG_WRITE_THROUGH,
    winx::file::{AccessMode, FileModeInformation},
};

/// Extension trait that can indicate various I/O flags.
pub trait GetSetFdFlags {
    /// Query the "status" flags for the `self` file descriptor.
    fn get_fd_flags(&self) -> io::Result<FdFlags>;

    /// Set the "status" flags for the `self` file descriptor. On some
    /// platforms, this may close the file descriptor and produce a new
    /// one.
    fn set_fd_flags(&mut self, flags: FdFlags) -> io::Result<()>;
}

bitflags! {
    /// Flag definitions for use with `SetFlags::set_flags`.
    pub struct FdFlags: u32 {
        /// Writes always write to the end of the file.
        const APPEND = 0x01;

        /// Write I/O operations on the file descriptor shall complete as
        /// defined by synchronized I/O *data* integrity completion.
        const DSYNC = 0x02;

        /// I/O operations return `io::ErrorKind::WouldBlock`.
        const NONBLOCK = 0x04;

        /// Read I/O operations on the file descriptor shall complete at the
        /// same level of integrity as specified by the `DSYNC` and `SYNC` flags.
        const RSYNC = 0x08;

        /// Write I/O operations on the file descriptor shall complete as
        /// defined by synchronized I/O *file* integrity completion.
        const SYNC = 0x10;
    }
}

#[cfg(not(windows))]
impl<T: AsUnsafeFile> GetSetFdFlags for T {
    fn get_fd_flags(&self) -> io::Result<FdFlags> {
        let mut fd_flags = FdFlags::empty();
        let flags = getfl(&self.as_unsafe_file())?;

        fd_flags.set(FdFlags::APPEND, flags.contains(OFlags::APPEND));
        fd_flags.set(FdFlags::DSYNC, flags.contains(OFlags::DSYNC));
        fd_flags.set(FdFlags::NONBLOCK, flags.contains(OFlags::NONBLOCK));
        #[cfg(any(
            target_os = "ios",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "fuchsia"
        ))]
        {
            fd_flags.set(FdFlags::SYNC, flags.contains(OFlags::SYNC));
        }
        #[cfg(not(any(
            target_os = "ios",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "fuchsia"
        )))]
        {
            fd_flags.set(FdFlags::RSYNC, flags.contains(OFlags::RSYNC));
            fd_flags.set(FdFlags::SYNC, flags.contains(OFlags::SYNC));
        }

        Ok(fd_flags)
    }

    fn set_fd_flags(&mut self, fd_flags: FdFlags) -> io::Result<()> {
        let mut flags = OFlags::empty();
        flags.set(OFlags::APPEND, fd_flags.contains(FdFlags::APPEND));
        flags.set(OFlags::NONBLOCK, fd_flags.contains(FdFlags::NONBLOCK));

        // Linux, FreeBSD, and others silently ignore these flags in `F_SETFL`.
        if fd_flags.intersects(FdFlags::DSYNC | FdFlags::SYNC | FdFlags::RSYNC) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "setting fd_flags SYNC, DSYNC, and RSYNC is not supported",
            ));
        }

        setfl(&self.as_unsafe_file(), flags)
    }
}

#[cfg(windows)]
impl<T: AsUnsafeFile + FromUnsafeFile> GetSetFdFlags for T {
    fn get_fd_flags(&self) -> io::Result<FdFlags> {
        let mut fd_flags = FdFlags::empty();
        let handle = self.as_unsafe_file().as_raw_handle();
        let access_mode = winx::file::query_access_information(handle)?;
        let mode = winx::file::query_mode_information(handle)?;

        // `FILE_APPEND_DATA` with `FILE_WRITE_DATA` means append-mode.
        if access_mode.contains(AccessMode::FILE_APPEND_DATA)
            && !access_mode.contains(AccessMode::FILE_WRITE_DATA)
        {
            fd_flags |= FdFlags::APPEND;
        }

        if mode.contains(FileModeInformation::FILE_WRITE_THROUGH) {
            // Only report `SYNC`. This is technically the only one of the
            // `O_?SYNC` flags Windows supports.
            fd_flags |= FdFlags::SYNC;
        }

        Ok(fd_flags)
    }

    fn set_fd_flags(&mut self, fd_flags: FdFlags) -> io::Result<()> {
        let mut flags = 0;

        if fd_flags.contains(FdFlags::SYNC)
            || fd_flags.contains(FdFlags::DSYNC)
            || fd_flags.contains(FdFlags::RSYNC)
        {
            flags |= FILE_FLAG_WRITE_THROUGH;
        }

        if fd_flags.contains(FdFlags::NONBLOCK) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Non-blocking I/O is not yet supported on Windows",
            ));
        }

        let handle = self.as_unsafe_file().as_raw_handle();
        let access_mode = winx::file::query_access_information(handle)?;
        let new_access_mode = file_access_mode_from_fd_flags(
            fd_flags,
            access_mode.contains(AccessMode::FILE_READ_DATA),
            access_mode.contains(AccessMode::FILE_WRITE_DATA)
                | access_mode.contains(AccessMode::FILE_APPEND_DATA),
        );
        let new_handle = winx::file::reopen_file(
            handle,
            new_access_mode,
            winx::file::Flags::from_bits(flags).unwrap(),
        )?;
        let unsafe_file = UnsafeFile::from_raw_handle(new_handle);
        *self = unsafe { T::from_unsafe_file(unsafe_file) };
        Ok(())
    }
}

#[cfg(windows)]
fn file_access_mode_from_fd_flags(fd_flags: FdFlags, read: bool, write: bool) -> AccessMode {
    let mut access_mode = AccessMode::READ_CONTROL;

    // We always need `FILE_WRITE_ATTRIBUTES` so that we can set attributes
    // such as filetimes, etc.
    access_mode.insert(AccessMode::FILE_WRITE_ATTRIBUTES);

    // Note that `GENERIC_READ` and `GENERIC_WRITE` cannot be used to properly
    // support append-only mode. The file-specific flags `FILE_GENERIC_READ`
    // and `FILE_GENERIC_WRITE` are used here instead. These flags have the
    // same semantic meaning for file objects, but allow removal of specific
    // permissions (see below).
    if read {
        access_mode.insert(AccessMode::FILE_GENERIC_READ);
    }
    if write {
        access_mode.insert(AccessMode::FILE_GENERIC_WRITE);
    }

    // For append, grant the handle FILE_APPEND_DATA access but *not*
    // `FILE_WRITE_DATA`. This makes the handle "append only". Changes to the
    // file pointer will be ignored (like POSIX's `O_APPEND` behavior).
    if fd_flags.contains(FdFlags::APPEND) {
        access_mode.insert(AccessMode::FILE_APPEND_DATA);
        access_mode.remove(AccessMode::FILE_WRITE_DATA);
    }

    access_mode
}
