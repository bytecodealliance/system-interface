#[cfg(not(windows))]
use posish::io::isatty;
#[cfg(windows)]
use std::{
    fs,
    io::{Stderr, StderrLock, Stdin, StdinLock, Stdout, StdoutLock},
    net,
};
#[cfg(not(windows))]
use unsafe_io::AsUnsafeHandle;

/// Extension for I/O handles which may or may not be terminals.
pub trait IsTerminal {
    /// Test whether this output stream is attached to a terminal.
    ///
    /// This operation is also known as `isatty`.
    fn is_terminal(&self) -> bool;
}

/// Implement `IsTerminal` for types that implement `AsRawFd`.
#[cfg(not(windows))]
impl<T> IsTerminal for T
where
    T: AsUnsafeHandle,
{
    #[inline]
    fn is_terminal(&self) -> bool {
        isatty(self)
    }
}

/// Implement `IsTerminal` for `Stdin`.
#[cfg(windows)]
impl IsTerminal for Stdin {
    #[inline]
    fn is_terminal(&self) -> bool {
        atty::is(atty::Stream::Stdin)
    }
}

/// Implement `IsTerminal` for `StdinLock`.
#[cfg(windows)]
impl<'a> IsTerminal for StdinLock<'a> {
    #[inline]
    fn is_terminal(&self) -> bool {
        atty::is(atty::Stream::Stdin)
    }
}

/// Implement `IsTerminal` for `Stdout`.
#[cfg(windows)]
impl IsTerminal for Stdout {
    #[inline]
    fn is_terminal(&self) -> bool {
        atty::is(atty::Stream::Stdout)
    }
}

/// Implement `IsTerminal` for `StdoutLock`.
#[cfg(windows)]
impl<'a> IsTerminal for StdoutLock<'a> {
    #[inline]
    fn is_terminal(&self) -> bool {
        atty::is(atty::Stream::Stdout)
    }
}

/// Implement `IsTerminal` for `Stderr`.
#[cfg(windows)]
impl IsTerminal for Stderr {
    #[inline]
    fn is_terminal(&self) -> bool {
        atty::is(atty::Stream::Stderr)
    }
}

/// Implement `IsTerminal` for `StderrLock`.
#[cfg(windows)]
impl<'a> IsTerminal for StderrLock<'a> {
    #[inline]
    fn is_terminal(&self) -> bool {
        atty::is(atty::Stream::Stderr)
    }
}

/// Implement `IsTerminal` for `std::fs::File`.
#[cfg(windows)]
impl<'a> IsTerminal for fs::File {
    #[inline]
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Implement `IsTerminal` for `std::net::TcpStream`.
#[cfg(windows)]
impl<'a> IsTerminal for net::TcpStream {
    #[inline]
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Implement `IsTerminal` for `cap_std::fs::File`.
#[cfg(all(windows, feature = "cap_std_impls"))]
impl<'a> IsTerminal for cap_std::fs::File {
    #[inline]
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Implement `IsTerminal` for `cap_std::net::TcpStream`.
#[cfg(all(windows, feature = "cap_std_impls"))]
impl<'a> IsTerminal for cap_std::net::TcpStream {
    #[inline]
    fn is_terminal(&self) -> bool {
        false
    }
}

#[cfg(all(windows, feature = "socket2"))]
impl IsTerminal for socket2::Socket {
    #[inline]
    fn is_terminal(&self) -> bool {
        use unsafe_io::AsUnsafeSocket;
        self.as_tcp_stream_view().is_terminal()
    }
}
