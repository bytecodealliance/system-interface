#[cfg(not(any(windows, target_os = "redox")))]
use posish::io::fionread;
use std::io::{self, Stdin, StdinLock};
#[cfg(not(target_os = "redox"))]
use std::net;
#[cfg(target_os = "wasi")]
use std::os::wasi::io::AsRawFd;
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
        Ok(1)
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
        Ok(1)
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
