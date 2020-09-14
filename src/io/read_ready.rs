#[cfg(not(windows))]
use posish::io::fionread;
#[cfg(target_os = "wasi")]
use std::os::wasi::io::AsRawFd;
use std::{
    io::{self, Stdin, StdinLock},
    net,
};
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
#[cfg(not(windows))]
impl ReadReady for Stdin {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        fionread(self)
    }
}

/// Implement `ReadReady` for `Stdin`.
#[cfg(windows)]
impl ReadReady for Stdin {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        // Return the conservatively correct result.
        Ok(1)
    }
}

/// Implement `ReadReady` for `StdinLock`.
#[cfg(not(windows))]
impl<'a> ReadReady for StdinLock<'a> {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        fionread(self)
    }
}

/// Implement `ReadReady` for `StdinLock`.
#[cfg(windows)]
impl<'a> ReadReady for StdinLock<'a> {
    #[inline]
    fn num_ready_bytes(&self) -> io::Result<u64> {
        // Return the conservatively correct result.
        Ok(1)
    }
}

/// Implement `ReadReady` for `std::net::TcpStream`.
#[cfg(not(windows))]
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
