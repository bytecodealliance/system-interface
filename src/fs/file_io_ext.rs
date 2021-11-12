use io_lifetimes::AsFilelike;
#[cfg(not(any(
    windows,
    target_os = "ios",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "redox",
)))]
use rustix::fs::fadvise;
#[cfg(any(target_os = "ios", target_os = "macos"))]
use rustix::fs::fcntl_rdadvise;
#[cfg(not(any(
    windows,
    target_os = "netbsd",
    target_os = "redox",
    target_os = "openbsd"
)))]
use rustix::fs::{fallocate, FallocateFlags};
#[cfg(not(any(windows, target_os = "ios", target_os = "macos", target_os = "redox")))]
use rustix::io::{preadv, pwritev};
use std::convert::TryInto;
use std::fmt::Arguments;
use std::io::{self, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write};
use std::slice;
#[cfg(windows)]
use {cap_fs_ext::Reopen, std::fs, std::os::windows::fs::FileExt};
#[cfg(not(windows))]
use {rustix::fs::tell, rustix::fs::FileExt};

/// Advice to pass to `FileIoExt::advise`.
#[cfg(not(any(
    windows,
    target_os = "ios",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "redox"
)))]
#[derive(Debug, Eq, PartialEq, Hash)]
#[repr(i32)]
pub enum Advice {
    /// No advice; default heuristics apply.
    Normal = rustix::fs::Advice::Normal as i32,
    /// Data will be accessed sequentially at ascending offsets.
    Sequential = rustix::fs::Advice::Sequential as i32,
    /// Data will be accessed with an irregular access pattern.
    Random = rustix::fs::Advice::Random as i32,
    /// Data will be accessed soon.
    WillNeed = rustix::fs::Advice::WillNeed as i32,
    /// Data will not be accessed soon.
    DontNeed = rustix::fs::Advice::DontNeed as i32,
    /// Data will be accessed exactly once.
    NoReuse = rustix::fs::Advice::NoReuse as i32,
}

/// Advice to pass to `FileIoExt::advise`.
#[cfg(any(
    windows,
    target_os = "ios",
    target_os = "macos",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "redox"
))]
#[derive(Debug, Eq, PartialEq, Hash)]
pub enum Advice {
    /// No advice; default heuristics apply.
    Normal,
    /// Data will be accessed sequentially at ascending offsets.
    Sequential,
    /// Data will be accessed with an irregular access pattern.
    Random,
    /// Data will be accessed soon.
    WillNeed,
    /// Data will not be accessed soon.
    DontNeed,
    /// Data will be accessed exactly once.
    NoReuse,
}

/// Extension trait for `std::fs::File` and `cap_std::fs::File`.
pub trait FileIoExt {
    /// Announce the expected access pattern of the data at the given offset.
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()>;

    /// Allocate space in the file, increasing the file size as needed, and
    /// ensuring that there are no holes under the given range.
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()>;

    /// Pull some bytes from this source into the specified buffer, returning
    /// how many bytes were read.
    ///
    /// This is similar to [`std::io::Read::read`], except it takes `self` by
    /// immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Read::read`]: https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read
    fn read(&self, buf: &mut [u8]) -> io::Result<usize>;

    /// Read the exact number of bytes required to fill `buf`.
    ///
    /// This is similar to [`std::io::Read::read_exact`], except it takes
    /// `self` by immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Read::read_exact`]: https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read_exact
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()>;

    /// Reads a number of bytes starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::read_at`], except it
    /// takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::read_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_at
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize>;

    /// Reads the exact number of byte required to fill buf from the given
    /// offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::read_exact_at`],
    /// except it takes `self` by immutable reference since the entire side
    /// effect is I/O, and it's supported on non-Unix platforms including
    /// Windows.
    ///
    /// [`std::os::unix::fs::FileExt::read_exact_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_exact_at
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()>;

    /// Like `read`, except that it reads into a slice of buffers.
    ///
    /// This is similar to [`std::io::Read::read_vectored`], except it takes
    /// `self` by immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Read::read_vectored`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_vectored
    fn read_vectored(&self, bufs: &mut [IoSliceMut]) -> io::Result<usize>;

    /// Is to `read_vectored` what `read_exact` is to `read`.
    fn read_exact_vectored(&self, mut bufs: &mut [IoSliceMut]) -> io::Result<()> {
        while !bufs.is_empty() {
            match self.read_vectored(bufs) {
                Ok(0) => break,
                Ok(nread) => bufs = advance_mut(bufs, nread),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Is to `read_vectored` what `read_at` is to `read`.
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        let buf = bufs
            .iter_mut()
            .find(|b| !b.is_empty())
            .map_or(&mut [][..], |b| &mut **b);
        self.read_at(buf, offset)
    }

    /// Is to `read_exact_vectored` what `read_exact_at` is to `read_exact`.
    fn read_exact_vectored_at(
        &self,
        mut bufs: &mut [IoSliceMut],
        mut offset: u64,
    ) -> io::Result<()> {
        while !bufs.is_empty() {
            match self.read_vectored_at(bufs, offset) {
                Ok(0) => break,
                Ok(nread) => {
                    offset = offset
                        .checked_add(nread.try_into().unwrap())
                        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "offset overflow"))?;
                    bufs = advance_mut(bufs, nread);
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Determines if this `Read`er has an efficient `read_vectored_at`
    /// implementation.
    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        false
    }

    /// Read all bytes until EOF in this source, placing them into `buf`.
    ///
    /// This is similar to [`std::io::Read::read_to_end`], except it takes
    /// `self` by immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Read::read_to_end`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_end
    fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize>;

    /// Read all bytes, starting at `offset`, until EOF in this source, placing
    /// them into `buf`.
    fn read_to_end_at(&self, buf: &mut Vec<u8>, offset: u64) -> io::Result<usize>;

    /// Read all bytes until EOF in this source, appending them to `buf`.
    ///
    /// This is similar to [`std::io::Read::read_to_string`], except it takes
    /// `self` by immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Read::read_to_string`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_string
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize>;

    /// Read all bytes, starting at `offset`, until EOF in this source,
    /// appending them to `buf`.
    fn read_to_string_at(&self, buf: &mut String, offset: u64) -> io::Result<usize>;

    /// Read bytes from the current position without advancing the current
    /// position.
    fn peek(&self, buf: &mut [u8]) -> io::Result<usize>;

    /// Write a buffer into this writer, returning how many bytes were written.
    ///
    /// This is similar to [`std::io::Write::write`], except it takes `self` by
    /// immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Write::write`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write
    fn write(&self, buf: &[u8]) -> io::Result<usize>;

    /// Attempts to write an entire buffer into this writer.
    ///
    /// This is similar to [`std::io::Write::write_all`], except it takes
    /// `self` by immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Write::write_all`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write_all
    fn write_all(&self, buf: &[u8]) -> io::Result<()>;

    /// Writes a number of bytes starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::write_at`], except it
    /// takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::write_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.write_at
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize>;

    /// Attempts to write an entire buffer starting from a given offset.
    ///
    /// This is similar to [`std::os::unix::fs::FileExt::write_all_at`], except
    /// it takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
    ///
    /// [`std::os::unix::fs::FileExt::write_all_at`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.write_all_at
    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()>;

    /// Like `write`, except that it writes from a slice of buffers.
    ///
    /// This is similar to [`std::io::Write::write_vectored`], except it takes
    /// `self` by immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Write::write_vectored`]: https://doc.rust-lang.org/std/io/trait.Write.html#method.write_vectored
    fn write_vectored(&self, bufs: &[IoSlice]) -> io::Result<usize>;

    /// Is to `write_vectored` what `write_all` is to `write`.
    fn write_all_vectored(&self, mut bufs: &mut [IoSlice]) -> io::Result<()> {
        // TODO: Use [rust-lang/rust#70436] once it stabilizes.
        // [rust-lang/rust#70436]: https://github.com/rust-lang/rust/issues/70436
        while !bufs.is_empty() {
            match self.write_vectored(bufs) {
                Ok(nwritten) => bufs = advance(bufs, nwritten),
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Is to `write_vectored` what `write_at` is to `write`.
    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        let buf = bufs
            .iter()
            .find(|b| !b.is_empty())
            .map_or(&[][..], |b| &**b);
        self.write_at(buf, offset)
    }

    /// Is to `write_all_vectored` what `write_all_at` is to `write_all`.
    fn write_all_vectored_at(&self, mut bufs: &mut [IoSlice], mut offset: u64) -> io::Result<()> {
        while !bufs.is_empty() {
            match self.write_vectored_at(bufs, offset) {
                Ok(nwritten) => {
                    offset = offset
                        .checked_add(nwritten.try_into().unwrap())
                        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "offset overflow"))?;
                    bufs = advance(bufs, nwritten);
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Determines if this `Write`r has an efficient `write_vectored_at`
    /// implementation.
    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        false
    }

    /// Writes a formatted string into this writer, returning any error
    /// encountered.
    ///
    /// This is similar to [`std::io::Write::write_fmt`], except it takes
    /// `self` by immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Write::write_fmt`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write_fmt
    fn write_fmt(&self, fmt: Arguments) -> io::Result<()>;

    /// Flush this output stream, ensuring that all intermediately buffered
    /// contents reach their destination.
    ///
    /// This is similar to [`std::io::Write::flush`], except it takes `self` by
    /// immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Write::flush`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.flush
    fn flush(&self) -> io::Result<()>;

    /// Seek to an offset, in bytes, in a stream.
    ///
    /// This is similar to [`std::io::Seek::seek`], except it takes `self` by
    /// immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Seek::seek`]: https://doc.rust-lang.org/std/io/trait.Seek.html#tymethod.seek
    fn seek(&self, pos: SeekFrom) -> io::Result<u64>;

    /// Returns the current seek position from the start of the stream.
    ///
    /// This is similar to [`std::io::Seek::stream_position`], except it's
    /// available on Rust stable.
    ///
    /// This may eventually be implemented by [rust-lang/rust#62726].
    ///
    /// [`std::io::Seek::stream_position`]: https://doc.rust-lang.org/std/io/trait.Seek.html#method.stream_position
    /// [rust-lang/rust#62726]: https://github.com/rust-lang/rust/issues/59359.
    fn stream_position(&self) -> io::Result<u64>;
}

/// This will be obviated by [rust-lang/rust#62726].
///
/// [rust-lang/rust#62726]: https://github.com/rust-lang/rust/issues/62726.
fn advance<'a, 'b>(bufs: &'b mut [IoSlice<'a>], n: usize) -> &'b mut [IoSlice<'a>] {
    // Number of buffers to remove.
    let mut remove = 0;
    // Total length of all the to be removed buffers.
    let mut accumulated_len = 0;
    for buf in bufs.iter() {
        if accumulated_len + buf.len() > n {
            break;
        } else {
            accumulated_len += buf.len();
            remove += 1;
        }
    }

    #[allow(clippy::indexing_slicing)]
    let bufs = &mut bufs[remove..];
    if let Some(first) = bufs.first_mut() {
        let advance_by = n - accumulated_len;
        let mut ptr = first.as_ptr();
        let mut len = first.len();
        unsafe {
            ptr = ptr.add(advance_by);
            len -= advance_by;
            *first = IoSlice::<'a>::new(slice::from_raw_parts::<'a>(ptr, len));
        }
    }
    bufs
}

/// This will be obviated by [rust-lang/rust#62726].
///
/// [rust-lang/rust#62726]: https://github.com/rust-lang/rust/issues/62726.
fn advance_mut<'a, 'b>(bufs: &'b mut [IoSliceMut<'a>], n: usize) -> &'b mut [IoSliceMut<'a>] {
    // Number of buffers to remove.
    let mut remove = 0;
    // Total length of all the to be removed buffers.
    let mut accumulated_len = 0;
    for buf in bufs.iter() {
        if accumulated_len + buf.len() > n {
            break;
        } else {
            accumulated_len += buf.len();
            remove += 1;
        }
    }

    #[allow(clippy::indexing_slicing)]
    let bufs = &mut bufs[remove..];
    if let Some(first) = bufs.first_mut() {
        let advance_by = n - accumulated_len;
        let mut ptr = first.as_mut_ptr();
        let mut len = first.len();
        unsafe {
            ptr = ptr.add(advance_by);
            len -= advance_by;
            *first = IoSliceMut::<'a>::new(slice::from_raw_parts_mut::<'a>(ptr, len));
        }
    }
    bufs
}

/// Implement `FileIoExt` for any type which implements `AsRawFd`.
#[cfg(not(windows))]
impl<T: AsFilelike> FileIoExt for T {
    #[cfg(not(any(
        target_os = "ios",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "redox"
    )))]
    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        let advice = match advice {
            Advice::WillNeed => rustix::fs::Advice::WillNeed,
            Advice::Normal => rustix::fs::Advice::Normal,
            Advice::Sequential => rustix::fs::Advice::Sequential,
            Advice::NoReuse => rustix::fs::Advice::NoReuse,
            Advice::Random => rustix::fs::Advice::Random,
            Advice::DontNeed => rustix::fs::Advice::DontNeed,
        };
        Ok(fadvise(self, offset, len, advice)?)
    }

    #[cfg(any(target_os = "ios", target_os = "macos"))]
    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        // Darwin lacks `posix_fadvise`, but does have an `fcntl_rdadvise`
        // feature which roughly corresponds to `WillNeed`. This is not yet
        // tuned.
        match advice {
            Advice::WillNeed => (),
            Advice::Normal
            | Advice::Sequential
            | Advice::NoReuse
            | Advice::Random
            | Advice::DontNeed => return Ok(()),
        }

        Ok(fcntl_rdadvise(self, offset, len)?)
    }

    #[cfg(any(target_os = "netbsd", target_os = "redox", target_os = "openbsd"))]
    #[inline]
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> io::Result<()> {
        // Netbsd lacks `posix_fadvise` and doesn't have an obvious replacement,
        // so just ignore the advice.
        Ok(())
    }

    #[cfg(not(any(target_os = "netbsd", target_os = "redox", target_os = "openbsd")))]
    #[inline]
    fn allocate(&self, offset: u64, len: u64) -> io::Result<()> {
        Ok(fallocate(self, FallocateFlags::empty(), offset, len)?)
    }

    #[cfg(target_os = "netbsd")]
    fn allocate(&self, _offset: u64, _len: u64) -> io::Result<()> {
        todo!("NetBSD 7.0 supports posix_fallocate; add bindings for it")
    }

    #[cfg(target_os = "openbsd")]
    fn allocate(&self, _offset: u64, _len: u64) -> io::Result<()> {
        todo!("OpenBSD does not support posix_fallocate; figure out what to do")
    }

    #[cfg(target_os = "redox")]
    fn allocate(&self, _offset: u64, _len: u64) -> io::Result<()> {
        todo!("figure out what to do on redox for posix_fallocate")
    }

    #[inline]
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        Read::read_exact(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        FileExt::read_at(&*self.as_filelike_view::<std::fs::File>(), buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        FileExt::read_exact_at(&*self.as_filelike_view::<std::fs::File>(), buf, offset)
    }

    #[inline]
    fn read_vectored(&self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        Read::read_vectored(&mut *self.as_filelike_view::<std::fs::File>(), bufs)
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos", target_os = "redox")))]
    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        Ok(preadv(self, bufs, offset)?)
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos", target_os = "redox")))]
    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        true
    }

    #[inline]
    fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        Read::read_to_end(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn read_to_end_at(&self, buf: &mut Vec<u8>, offset: u64) -> io::Result<usize> {
        read_to_end_at(&self.as_filelike_view::<std::fs::File>(), buf, offset)
    }

    #[inline]
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize> {
        Read::read_to_string(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn read_to_string_at(&self, buf: &mut String, offset: u64) -> io::Result<usize> {
        read_to_string_at(&self.as_filelike_view::<std::fs::File>(), buf, offset)
    }

    #[inline]
    fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        let pos = self.seek(SeekFrom::Current(0))?;
        self.read_at(buf, pos)
    }

    #[inline]
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Write::write(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        Write::write_all(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        FileExt::write_at(&*self.as_filelike_view::<std::fs::File>(), buf, offset)
    }

    #[inline]
    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        FileExt::write_all_at(&*self.as_filelike_view::<std::fs::File>(), buf, offset)
    }

    #[inline]
    fn write_vectored(&self, bufs: &[IoSlice]) -> io::Result<usize> {
        Write::write_vectored(&mut *self.as_filelike_view::<std::fs::File>(), bufs)
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos", target_os = "redox")))]
    #[inline]
    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        Ok(pwritev(self, bufs, offset)?)
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos", target_os = "redox")))]
    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        true
    }

    #[inline]
    fn flush(&self) -> io::Result<()> {
        Write::flush(&mut *self.as_filelike_view::<std::fs::File>())
    }

    #[inline]
    fn write_fmt(&self, fmt: Arguments) -> io::Result<()> {
        Write::write_fmt(&mut *self.as_filelike_view::<std::fs::File>(), fmt)
    }

    #[inline]
    fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        Seek::seek(&mut *self.as_filelike_view::<std::fs::File>(), pos)
    }

    #[inline]
    fn stream_position(&self) -> io::Result<u64> {
        // This may eventually be obsoleted by [rust-lang/rust#59359].
        // [rust-lang/rust#59359]: https://github.com/rust-lang/rust/issues/59359.
        Ok(tell(self)?)
    }
}

#[cfg(windows)]
impl<T: AsFilelike> FileIoExt for T {
    #[inline]
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> io::Result<()> {
        // TODO: Do something with the advice.
        Ok(())
    }

    #[inline]
    fn allocate(&self, _offset: u64, _len: u64) -> io::Result<()> {
        // We can't faithfully support allocate on Windows without exposing race
        // conditions. Instead, refuse:
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "file allocate is not supported on Windows",
        ))
    }

    #[inline]
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        Read::read_exact(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        // Windows' `seek_read` modifies the current position in the file, so
        // re-open the file to leave the original open file unmodified.
        let reopened = reopen(self)?;
        reopened.seek_read(buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        // Similar to `read_at`, re-open the file so that we can do a seek and
        // leave the original file unmodified.
        let reopened = loop {
            match reopen(self) {
                Ok(file) => break file,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        };
        loop {
            match reopened.seek(SeekFrom::Start(offset)) {
                Ok(_) => break,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }
        reopened.read_exact(buf)
    }

    #[inline]
    fn read_vectored(&self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        Read::read_vectored(&mut *self.as_filelike_view::<std::fs::File>(), bufs)
    }

    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        // Similar to `read_at`, re-open the file so that we can do a seek and
        // leave the original file unmodified.
        let reopened = reopen(self)?;
        reopened.seek(SeekFrom::Start(offset))?;
        reopened.read_vectored(bufs)
    }

    #[inline]
    fn read_exact_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<()> {
        // Similar to `read_vectored_at`, re-open the file so that we can do a seek and
        // leave the original file unmodified.
        let reopened = loop {
            match reopen(self) {
                Ok(file) => break file,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        };
        loop {
            match reopened.seek(SeekFrom::Start(offset)) {
                Ok(_) => break,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }
        reopened.read_exact_vectored(bufs)
    }

    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        true
    }

    #[inline]
    fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        Read::read_to_end(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn read_to_end_at(&self, buf: &mut Vec<u8>, offset: u64) -> io::Result<usize> {
        read_to_end_at(&self.as_filelike_view::<std::fs::File>(), buf, offset)
    }

    #[inline]
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize> {
        Read::read_to_string(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn read_to_string_at(&self, buf: &mut String, offset: u64) -> io::Result<usize> {
        read_to_string_at(&self.as_filelike_view::<std::fs::File>(), buf, offset)
    }

    #[inline]
    fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        let reopened = reopen_write(self)?;
        reopened.read(buf)
    }

    #[inline]
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Write::write(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        Write::write_all(&mut *self.as_filelike_view::<std::fs::File>(), buf)
    }

    #[inline]
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        // Windows' `seek_write` modifies the current position in the file, so
        // re-open the file to leave the original open file unmodified.
        let reopened = reopen_write(self)?;
        reopened.seek_write(buf, offset)
    }

    #[inline]
    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        // Similar to `read_exact_at`, re-open the file so that we can do a seek and
        // leave the original file unmodified.
        let reopened = loop {
            match reopen_write(self) {
                Ok(file) => break file,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        };
        loop {
            match reopened.seek(SeekFrom::Start(offset)) {
                Ok(_) => break,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }
        reopened.write_all(buf)
    }

    #[inline]
    fn write_vectored(&self, bufs: &[IoSlice]) -> io::Result<usize> {
        Write::write_vectored(&mut *self.as_filelike_view::<std::fs::File>(), bufs)
    }

    #[inline]
    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        // Similar to `read_vectored_at`, re-open the file to avoid adjusting
        // the current position of the already-open file.
        let reopened = reopen_write(self)?;
        reopened.seek(SeekFrom::Start(offset))?;
        reopened.write_vectored(bufs)
    }

    #[inline]
    fn write_all_vectored_at(&self, bufs: &mut [IoSlice], offset: u64) -> io::Result<()> {
        // Similar to `read_vectored_at`, re-open the file to avoid adjusting
        // the current position of the already-open file.
        let reopened = loop {
            match reopen_write(self) {
                Ok(file) => break file,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        };
        loop {
            match reopened.seek(SeekFrom::Start(offset)) {
                Ok(_) => break,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            }
        }
        reopened.write_all_vectored(bufs)
    }

    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        true
    }

    #[inline]
    fn flush(&self) -> io::Result<()> {
        Write::flush(&mut *self.as_filelike_view::<std::fs::File>())
    }

    #[inline]
    fn write_fmt(&self, fmt: Arguments) -> io::Result<()> {
        Write::write_fmt(&mut *self.as_filelike_view::<std::fs::File>(), fmt)
    }

    #[inline]
    fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        Seek::seek(&mut *self.as_filelike_view::<std::fs::File>(), pos)
    }

    #[inline]
    fn stream_position(&self) -> io::Result<u64> {
        // This may eventually be obsoleted by [rust-lang/rust#59359].
        // [rust-lang/rust#59359]: https://github.com/rust-lang/rust/issues/59359.
        Seek::seek(
            &mut *self.as_filelike_view::<std::fs::File>(),
            SeekFrom::Current(0),
        )
    }
}

#[cfg(windows)]
#[inline]
fn reopen<Filelike: AsFilelike>(filelike: &Filelike) -> io::Result<fs::File> {
    let file = filelike.as_filelike_view::<std::fs::File>();
    unsafe { _reopen(&file) }
}

#[cfg(windows)]
unsafe fn _reopen(file: &fs::File) -> io::Result<fs::File> {
    file.reopen(cap_fs_ext::OpenOptions::new().read(true))
}

#[cfg(windows)]
#[inline]
fn reopen_write<Filelike: AsFilelike>(filelike: &Filelike) -> io::Result<fs::File> {
    let file = filelike.as_filelike_view::<std::fs::File>();
    unsafe { _reopen_write(&file) }
}

#[cfg(windows)]
unsafe fn _reopen_write(file: &fs::File) -> io::Result<fs::File> {
    file.reopen(cap_fs_ext::OpenOptions::new().write(true))
}

fn read_to_end_at(file: &std::fs::File, buf: &mut Vec<u8>, offset: u64) -> io::Result<usize> {
    let len = match file.metadata()?.len().checked_sub(offset) {
        None => return Ok(0),
        Some(len) => len,
    };

    // This initializes the buffer with zeros which is theoretically
    // unnecesary, but current alternatives involve tricky `unsafe` code.
    buf.resize(
        (buf.len() as u64)
            .saturating_add(len)
            .try_into()
            .unwrap_or(usize::MAX),
        0_u8,
    );
    FileIoExt::read_exact_at(file, buf, offset)?;
    Ok(len as usize)
}

fn read_to_string_at(file: &std::fs::File, buf: &mut String, offset: u64) -> io::Result<usize> {
    let len = match file.metadata()?.len().checked_sub(offset) {
        None => return Ok(0),
        Some(len) => len,
    };

    // This temporary buffer is theoretically unnecessary, but eliminating it
    // curently involves a bunch of `unsafe`.
    let mut tmp = vec![0_u8; len.try_into().unwrap_or(usize::MAX)];
    FileIoExt::read_exact_at(file, &mut tmp, offset)?;
    let s = String::from_utf8(tmp).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })?;
    buf.push_str(&s);
    Ok(len as usize)
}

fn _file_io_ext_can_be_trait_object(_: &dyn FileIoExt) {}
