#[cfg(not(any(windows, target_os = "redox")))]
use posish::io::fionread;
use std::io::{self, Stdin, StdinLock};
#[cfg(not(target_os = "redox"))]
use std::net;
#[cfg(windows)]
use {
    std::{mem::MaybeUninit, os::windows::io::AsRawSocket},
    winapi::um::winsock2::{ioctlsocket, FIONREAD, SOCKET},
};

/// Extension for readable streams that can indicate the number of bytes
/// ready to be read immediately.
pub trait ReadReady {
    /// Return the number of bytes which are ready to be read immediately.
    fn num_ready_bytes(&self) -> io::Result<u64>;
}

/// Implement `ReadReady` for `Stdin`.
#[cfg(not(any(windows, target_os = "redox")))]
impl ReadReady for Stdin {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        fionread(self)
    }
}

/// Implement `ReadReady` for `Stdin`.
#[cfg(any(windows, target_os = "redox"))]
impl ReadReady for Stdin {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        // Return the conservatively correct result.
        Ok(0)
    }
}

/// Implement `ReadReady` for `StdinLock`.
#[cfg(not(any(windows, target_os = "redox")))]
impl<'a> ReadReady for StdinLock<'a> {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        fionread(self)
    }
}

/// Implement `ReadReady` for `StdinLock`.
#[cfg(any(windows, target_os = "redox"))]
impl<'a> ReadReady for StdinLock<'a> {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        // Return the conservatively correct result.
        Ok(0)
    }
}

/// Implement `ReadReady` for `std::net::TcpStream`.
#[cfg(not(any(windows, target_os = "redox")))]
impl ReadReady for net::TcpStream {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        fionread(self)
    }
}

/// Implement `ReadReady` for `std::net::TcpStream`.
#[cfg(target_os = "redox")]
impl ReadReady for net::TcpStream {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        // Return the conservatively correct result.
        Ok(0)
    }
}

/// Implement `ReadReady` for `std::os::unix::net::UnixStream`.
#[cfg(unix)]
impl ReadReady for std::os::unix::net::UnixStream {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        fionread(self)
    }
}

/// Implement `ReadReady` for `std::net::TcpStream`.
#[cfg(windows)]
impl ReadReady for net::TcpStream {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        let mut arg = MaybeUninit::<winapi::ctypes::c_ulong>::uninit();
        if unsafe { ioctlsocket(self.as_raw_socket() as SOCKET, FIONREAD, arg.as_mut_ptr()) } == 0 {
            Ok(unsafe { arg.assume_init() }.into())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(all(not(windows), feature = "os_pipe"))]
impl ReadReady for os_pipe::PipeReader {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        fionread(self)
    }
}

#[cfg(all(windows, feature = "os_pipe"))]
impl ReadReady for os_pipe::PipeReader {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        // Return the conservatively correct result.
        Ok(0)
    }
}

#[cfg(feature = "socketpair")]
impl ReadReady for socketpair::SocketpairStream {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        socketpair::SocketpairStream::num_ready_bytes(self)
    }
}

#[cfg(feature = "char-device")]
impl ReadReady for char_device::CharDevice {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        char_device::CharDevice::num_ready_bytes(self)
    }
}

/// Implement `ReadReady` for `cap_std::fs::File`.
#[cfg(feature = "cap_std_impls")]
impl ReadReady for cap_std::fs::File {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        use unsafe_io::AsUnsafeFile;
        self.as_file_view().num_ready_bytes()
    }
}

/// Implement `ReadReady` for `cap_std::fs_utf8::File`.
#[cfg(feature = "cap_std_impls_fs_utf8")]
impl ReadReady for cap_std::fs_utf8::File {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        use unsafe_io::AsUnsafeFile;
        self.as_file_view().num_ready_bytes()
    }
}
