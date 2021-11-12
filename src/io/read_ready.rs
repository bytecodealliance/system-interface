#[cfg(not(any(windows, target_os = "redox")))]
use rustix::io::ioctl_fionread;
use std::io::{self, Stdin, StdinLock};
#[cfg(not(target_os = "redox"))]
use std::net;
use std::process::{ChildStderr, ChildStdout};
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
        Ok(ioctl_fionread(self)?)
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
        Ok(ioctl_fionread(self)?)
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
        Ok(ioctl_fionread(self)?)
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
        Ok(ioctl_fionread(self)?)
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

/// Implement `ReadReady` for `std::net::TcpStream`.
#[cfg(feature = "socket2")]
impl ReadReady for socket2::Socket {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        use io_lifetimes::AsSocketlike;
        self.as_socketlike_view::<std::net::TcpStream>()
            .num_ready_bytes()
    }
}

#[cfg(all(not(windows), feature = "os_pipe"))]
impl ReadReady for os_pipe::PipeReader {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        Ok(ioctl_fionread(self)?)
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

#[cfg(not(windows))]
impl ReadReady for ChildStdout {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        Ok(ioctl_fionread(self)?)
    }
}

#[cfg(windows)]
impl ReadReady for ChildStdout {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        // Return the conservatively correct result.
        Ok(0)
    }
}

#[cfg(not(windows))]
impl ReadReady for ChildStderr {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        Ok(ioctl_fionread(self)?)
    }
}

#[cfg(windows)]
impl ReadReady for ChildStderr {
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

#[cfg(feature = "ssh2")]
impl ReadReady for ssh2::Channel {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        Ok(u64::from(self.read_window().available))
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
        use io_lifetimes::AsFilelike;
        self.as_filelike_view::<std::fs::File>().num_ready_bytes()
    }
}

/// Implement `ReadReady` for `cap_std::fs_utf8::File`.
#[cfg(feature = "cap_std_impls_fs_utf8")]
impl ReadReady for cap_std::fs_utf8::File {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        use io_lifetimes::AsFilelike;
        self.as_filelike_view::<std::fs::File>().num_ready_bytes()
    }
}
