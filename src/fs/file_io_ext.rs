use super::{as_file, into_file};
#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "netbsd")))]
use posish::fs::fadvise;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use posish::fs::rdadvise;
#[cfg(not(windows))]
use posish::fs::tell;
#[cfg(not(any(windows, target_os = "macos", target_os = "ios")))]
use posish::fs::{preadv, pwritev};
#[cfg(unix)]
use std::os::unix::{fs::FileExt, io::AsRawFd};
use std::{
    convert::TryInto,
    fmt::Arguments,
    fs,
    io::{self, IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write},
    slice,
};
#[cfg(windows)]
use {
    std::{
        ffi::{c_void, OsString},
        mem::{size_of, MaybeUninit},
        os::windows::{
            io::RawHandle,
            {ffi::OsStringExt, fs::FileExt, io::AsRawHandle},
        },
    },
    winapi::{
        shared::winerror::{ERROR_BUFFER_OVERFLOW, ERROR_FILE_NOT_FOUND},
        um::{
            fileapi::{
                GetFileInformationByHandle, GetFinalPathNameByHandleW, BY_HANDLE_FILE_INFORMATION,
                FILE_ID_INFO,
            },
            minwinbase::FileIdInfo,
            winbase::GetFileInformationByHandleEx,
        },
    },
    winx::file::WIDE_MAX_PATH,
};

/// Advice to pass to `FileIoExt::advise`.
#[cfg(not(any(windows, target_os = "macos", target_os = "ios", target_os = "netbsd")))]
#[derive(Debug, Eq, PartialEq, Hash)]
#[repr(i32)]
pub enum Advice {
    /// No advice; default heuristics apply.
    Normal = posish::fs::Advice::Normal as i32,
    /// Data will be accessed sequentially at ascending offsets.
    Sequential = posish::fs::Advice::Sequential as i32,
    /// Data will be accessed with an irregular access pattern.
    Random = posish::fs::Advice::Random as i32,
    /// Data will be accessed soon.
    WillNeed = posish::fs::Advice::WillNeed as i32,
    /// Data will not be accessed soon.
    DontNeed = posish::fs::Advice::DontNeed as i32,
    /// Data will be accessed exactly once.
    NoReuse = posish::fs::Advice::NoReuse as i32,
}

/// Advice to pass to `FileIoExt::advise`.
#[cfg(any(windows, target_os = "macos", target_os = "ios", target_os = "netbsd"))]
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

/// extension trait for `std::fs::File` and `cap_std::fs::File`.
pub trait FileIoExt {
    /// Announce the expected access pattern of the data at the given offset.
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()>;

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
    /// This is similar to [`std::os::unix::fs::FileExt::read_exact_at`], except
    /// it takes `self` by immutable reference since the entire side effect is
    /// I/O, and it's supported on non-Unix platforms including Windows.
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

    /// Determines if this `Read`er has an efficient `read_vectored_at` implementation.
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

    /// Read all bytes until EOF in this source, appending them to `buf`.
    ///
    /// This is similar to [`std::io::Read::read_to_string`], except it takes
    /// `self` by immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Read::read_to_string`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.read_to_string
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize>;

    /// Transforms this `FileIoExt` instance to an [`Iterator`] over its bytes.
    ///
    /// This is similar to [`Read::bytes`], except it returns a
    /// `io::Bytes<fs::File>` instead of an `io::Bytes::<Self>`.
    ///
    /// [`Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
    /// [`Read::bytes`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.bytes
    fn bytes(self) -> io::Bytes<fs::File>;

    /// Creates an adaptor which will chain this stream with another.
    ///
    /// This is similar to [`Read::chain`], except it returns a
    /// `io::Chain<fs::File, R>` instead of an `io::Chain::<Self, R>`.
    ///
    /// [`Read::chain`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.chain
    fn chain<R: Read>(self, next: R) -> io::Chain<fs::File, R>;

    /// Creates an adaptor which will read at most `limit` bytes from it.
    ///
    /// This is similar to [`Read::take`], except it returns a
    /// `io::Take<fs::File>` instead of an `io::Take::<Self>`.
    ///
    /// [`Read::take`]: https://doc.rust-lang.org/std/io/trait.Read.html#method.take
    fn take(self, limit: u64) -> io::Take<fs::File>;

    /// Write a buffer into this writer, returning how many bytes were written.
    ///
    /// This is similar to [`std::io::Write::write`], except it takes `self` by
    /// immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Write::write`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write
    fn write(&self, buf: &[u8]) -> io::Result<usize>;

    /// Attempts to write an entire buffer into this writer.
    ///
    /// This is similar to [`std::io::Write::write_all`], except it takes `self`
    /// by immutable reference since the entire side effect is I/O.
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
        // TODO: Use [rust-lang/rust#70436]once it stabilizes.
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

    /// Determines if this `Write`r has an efficient `write_vectored_at` implementation.
    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        false
    }

    /// Writes a formatted string into this writer, returning any error encountered.
    ///
    /// This is similar to [`std::io::Write::write_fmt`], except it takes `self` by
    /// immutable reference since the entire side effect is I/O.
    ///
    /// [`std::io::Write::write_fmt`]: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write_fmt
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()>;

    /// Flush this output stream, ensuring that all intermediately buffered contents reach their destination.
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

#[cfg(windows)]
unsafe fn file_path(handle: RawHandle) -> io::Result<OsString> {
    let mut raw_path: Vec<u16> = vec![0; WIDE_MAX_PATH as usize];

    let read_len = GetFinalPathNameByHandleW(handle, raw_path.as_mut_ptr(), WIDE_MAX_PATH, 0);
    if read_len == 0 {
        return Err(io::Error::last_os_error());
    }

    // obtain a slice containing the written bytes, and check for it being too long
    // (practically probably impossible)
    let written_bytes = raw_path
        .get(..read_len as usize)
        .ok_or_else(|| io::Error::from_raw_os_error(ERROR_BUFFER_OVERFLOW as i32))?;

    Ok(OsString::from_wide(written_bytes))
}

/// Implement `FileIoExt` for any type which implements `AsRawFd`.
#[cfg(not(windows))]
impl<T> FileIoExt for T
where
    T: AsRawFd,
{
    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "netbsd")))]
    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        let advice = match advice {
            Advice::WillNeed => posish::fs::Advice::WillNeed,
            Advice::Normal => posish::fs::Advice::Normal,
            Advice::Sequential => posish::fs::Advice::Sequential,
            Advice::NoReuse => posish::fs::Advice::NoReuse,
            Advice::Random => posish::fs::Advice::Random,
            Advice::DontNeed => posish::fs::Advice::DontNeed,
        };
        fadvise(self, offset, len, advice)
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    #[inline]
    fn advise(&self, offset: u64, len: u64, advice: Advice) -> io::Result<()> {
        // Darwin lacks `posix_fadvise`, but does have an `rdadvise` feature
        // which roughly corresponds to `WillNeed`. This is not yet tuned.
        match advice {
            Advice::WillNeed => (),
            Advice::Normal
            | Advice::Sequential
            | Advice::NoReuse
            | Advice::Random
            | Advice::DontNeed => return Ok(()),
        }

        rdadvise(self, offset, len)
    }

    #[cfg(target_os = "netbsd")]
    #[inline]
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> io::Result<()> {
        // Netbsd lacks `posix_fadvise` and doesn't have an obvious replacement,
        // so just ignore the advice.
        Ok(())
    }

    #[inline]
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        Read::read_exact(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        FileExt::read_at(&*unsafe { as_file(self) }, buf, offset)
    }

    #[inline]
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> io::Result<()> {
        FileExt::read_exact_at(&*unsafe { as_file(self) }, buf, offset)
    }

    #[inline]
    fn read_vectored(&self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        Read::read_vectored(&mut *unsafe { as_file(self) }, bufs)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    #[inline]
    fn read_vectored_at(&self, bufs: &mut [IoSliceMut], offset: u64) -> io::Result<usize> {
        preadv(self, bufs, offset)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    #[inline]
    fn is_read_vectored_at(&self) -> bool {
        true
    }

    #[inline]
    fn read_to_end(&self, buf: &mut Vec<u8>) -> io::Result<usize> {
        Read::read_to_end(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize> {
        Read::read_to_string(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn bytes(self) -> io::Bytes<fs::File> where {
        Read::bytes(into_file(self))
    }

    #[inline]
    fn chain<R: Read>(self, next: R) -> io::Chain<fs::File, R> where {
        Read::chain(into_file(self), next)
    }

    #[inline]
    fn take(self, limit: u64) -> io::Take<fs::File> where {
        Read::take(into_file(self), limit)
    }

    #[inline]
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Write::write(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        Write::write_all(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn write_at(&self, buf: &[u8], offset: u64) -> io::Result<usize> {
        FileExt::write_at(&*unsafe { as_file(self) }, buf, offset)
    }

    #[inline]
    fn write_all_at(&self, buf: &[u8], offset: u64) -> io::Result<()> {
        FileExt::write_all_at(&*unsafe { as_file(self) }, buf, offset)
    }

    #[inline]
    fn write_vectored(&self, bufs: &[IoSlice]) -> io::Result<usize> {
        Write::write_vectored(&mut *unsafe { as_file(self) }, bufs)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    #[inline]
    fn write_vectored_at(&self, bufs: &[IoSlice], offset: u64) -> io::Result<usize> {
        pwritev(self, bufs, offset)
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    #[inline]
    fn is_write_vectored_at(&self) -> bool {
        true
    }

    #[inline]
    fn flush(&self) -> io::Result<()> {
        Write::flush(&mut *unsafe { as_file(self) })
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> {
        Write::write_fmt(&mut *unsafe { as_file(self) }, fmt)
    }

    #[inline]
    fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        Seek::seek(&mut *unsafe { as_file(self) }, pos)
    }

    #[inline]
    fn stream_position(&self) -> io::Result<u64> {
        // This may eventually be obsoleted by [rust-lang/rust#59359].
        // [rust-lang/rust#59359]: https://github.com/rust-lang/rust/issues/59359.
        tell(self)
    }
}

#[cfg(windows)]
impl FileIoExt for fs::File {
    #[inline]
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> io::Result<()> {
        // TODO: Do something with the advice.
        Ok(())
    }

    #[inline]
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        Read::read_exact(&mut *unsafe { as_file(self) }, buf)
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
        Read::read_vectored(&mut *unsafe { as_file(self) }, bufs)
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
        Read::read_to_end(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize> {
        Read::read_to_string(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn bytes(self) -> io::Bytes<fs::File> where {
        Read::bytes(into_file(self))
    }

    #[inline]
    fn chain<R: Read>(self, next: R) -> io::Chain<fs::File, R> where {
        Read::chain(into_file(self), next)
    }

    #[inline]
    fn take(self, limit: u64) -> io::Take<fs::File> where {
        Read::take(into_file(self), limit)
    }

    #[inline]
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Write::write(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        Write::write_all(&mut *unsafe { as_file(self) }, buf)
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
        Write::write_vectored(&mut *unsafe { as_file(self) }, bufs)
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
        Write::flush(&mut *unsafe { as_file(self) })
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> {
        Write::write_fmt(&mut *unsafe { as_file(self) }, fmt)
    }

    #[inline]
    fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        Seek::seek(&mut *unsafe { as_file(self) }, pos)
    }

    #[inline]
    fn stream_position(&self) -> io::Result<u64> {
        // This may eventually be obsoleted by [rust-lang/rust#59359].
        // [rust-lang/rust#59359]: https://github.com/rust-lang/rust/issues/59359.
        Seek::seek(&mut *unsafe { as_file(self) }, SeekFrom::Current(0))
    }
}

#[cfg(all(windows, feature = "cap_std_impls"))]
impl FileIoExt for cap_std::fs::File {
    #[inline]
    fn advise(&self, _offset: u64, _len: u64, _advice: Advice) -> io::Result<()> {
        // TODO: Do something with the advice.
        Ok(())
    }

    #[inline]
    fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        Read::read(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn read_exact(&self, buf: &mut [u8]) -> io::Result<()> {
        Read::read_exact(&mut *unsafe { as_file(self) }, buf)
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
        Read::read_vectored(&mut *unsafe { as_file(self) }, bufs)
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
        Read::read_to_end(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn read_to_string(&self, buf: &mut String) -> io::Result<usize> {
        Read::read_to_string(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn bytes(self) -> io::Bytes<fs::File> where {
        Read::bytes(into_file(self))
    }

    #[inline]
    fn chain<R: Read>(self, next: R) -> io::Chain<fs::File, R> where {
        Read::chain(into_file(self), next)
    }

    #[inline]
    fn take(self, limit: u64) -> io::Take<fs::File> where {
        Read::take(into_file(self), limit)
    }

    #[inline]
    fn write(&self, buf: &[u8]) -> io::Result<usize> {
        Write::write(&mut *unsafe { as_file(self) }, buf)
    }

    #[inline]
    fn write_all(&self, buf: &[u8]) -> io::Result<()> {
        Write::write_all(&mut *unsafe { as_file(self) }, buf)
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
        Write::write_vectored(&mut *unsafe { as_file(self) }, bufs)
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
        Write::flush(&mut *unsafe { as_file(self) })
    }

    #[inline]
    fn write_fmt(&mut self, fmt: Arguments) -> io::Result<()> {
        Write::write_fmt(&mut *unsafe { as_file(self) }, fmt)
    }

    #[inline]
    fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        Seek::seek(&mut *unsafe { as_file(self) }, pos)
    }

    #[inline]
    fn stream_position(&self) -> io::Result<u64> {
        // This may eventually be obsoleted by [rust-lang/rust#59359].
        // [rust-lang/rust#59359]: https://github.com/rust-lang/rust/issues/59359.
        Seek::seek(&mut *unsafe { as_file(self) }, SeekFrom::Current(0))
    }
}

#[cfg(windows)]
fn reopen<Handle: AsRawHandle>(handle: &Handle) -> io::Result<fs::File> {
    let handle = handle.as_raw_handle();
    unsafe { _reopen(handle) }
}

#[cfg(windows)]
unsafe fn _reopen(handle: RawHandle) -> io::Result<fs::File> {
    let path = file_path(handle).map_err(|err| {
        if let Some(code) = err.raw_os_error() {
            if code == ERROR_FILE_NOT_FOUND as i32 {
                return concurrent_rename();
            }
        }
        err
    })?;
    let reopened = fs::File::open(path).map_err(|err| {
        if let Some(code) = err.raw_os_error() {
            if code == ERROR_FILE_NOT_FOUND as i32 {
                return concurrent_rename();
            }
        }
        err
    })?;

    if !is_same_file(reopened.as_raw_handle(), handle)? {
        return Err(concurrent_rename());
    }

    Ok(reopened)
}

#[cfg(windows)]
fn reopen_write<Handle: AsRawHandle>(handle: &Handle) -> io::Result<fs::File> {
    let handle = handle.as_raw_handle();
    unsafe { _reopen_write(handle) }
}

#[cfg(windows)]
unsafe fn _reopen_write(handle: RawHandle) -> io::Result<fs::File> {
    let path = file_path(handle).map_err(|err| {
        if let Some(code) = err.raw_os_error() {
            if code == ERROR_FILE_NOT_FOUND as i32 {
                return concurrent_rename();
            }
        }
        err
    })?;
    let reopened = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|err| {
            if let Some(code) = err.raw_os_error() {
                if code == ERROR_FILE_NOT_FOUND as i32 {
                    return concurrent_rename();
                }
            }
            err
        })?;

    if !is_same_file(reopened.as_raw_handle(), handle)? {
        return Err(concurrent_rename());
    }

    Ok(reopened)
}

#[cfg(windows)]
unsafe fn is_same_file(a: RawHandle, b: RawHandle) -> io::Result<bool> {
    Ok(get_id(a)? == get_id(b)?)
}

/// Return file information fields which uniquely identify an open file.
#[cfg(windows)]
unsafe fn get_id(handle: RawHandle) -> io::Result<(u32, u32, u32, u64, u128)> {
    // It may not be necessary to get the `BY_HANDLE_FILE_INFORMATION` here,
    // as the `FILE_ID_INFO` may be sufficient, but for now, be conservative.
    let mut file_info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    let mut id_info = MaybeUninit::<FILE_ID_INFO>::uninit();

    if GetFileInformationByHandle(handle, file_info.as_mut_ptr()) == 0
        || GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            id_info.as_mut_ptr() as *mut c_void,
            size_of::<FILE_ID_INFO>() as u32,
        ) == 0
    {
        return Err(io::Error::last_os_error());
    }

    let file_info = file_info.assume_init();
    let id_info = id_info.assume_init();

    Ok((
        file_info.dwVolumeSerialNumber,
        file_info.nFileIndexLow,
        file_info.nFileIndexHigh,
        id_info.VolumeSerialNumber,
        u128::from_ne_bytes(id_info.FileId.Identifier),
    ))
}

#[cfg(windows)]
#[cold]
fn concurrent_rename() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "file was concurrently renamed")
}
