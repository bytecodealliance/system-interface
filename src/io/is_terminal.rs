#[cfg(windows)]
use std::{
    fs,
    io::{Stderr, StderrLock, Stdin, StdinLock, Stdout, StdoutLock},
    net,
};
#[cfg(not(windows))]
use {io_lifetimes::AsFilelike, rustix::io::isatty};

/// Extension for I/O handles which may or may not be terminals.
pub trait IsTerminal {
    /// Test whether this output stream is attached to a terminal.
    ///
    /// This operation is also known as `isatty`.
    fn is_terminal(&self) -> bool;
}

/// Implement `IsTerminal` for types that implement `AsRawFd`.
#[cfg(not(windows))]
impl<T: AsFilelike> IsTerminal for T {
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
impl IsTerminal for fs::File {
    #[inline]
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Implement `IsTerminal` for `std::net::TcpStream`.
#[cfg(windows)]
impl IsTerminal for net::TcpStream {
    #[inline]
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Implement `IsTerminal` for `cap_std::fs::File`.
#[cfg(all(windows, feature = "cap_std_impls"))]
impl IsTerminal for cap_std::fs::File {
    #[inline]
    fn is_terminal(&self) -> bool {
        false
    }
}

/// Implement `IsTerminal` for `cap_std::net::TcpStream`.
#[cfg(all(windows, feature = "cap_std_impls"))]
impl IsTerminal for cap_std::net::TcpStream {
    #[inline]
    fn is_terminal(&self) -> bool {
        false
    }
}

#[cfg(all(windows, feature = "socket2"))]
impl IsTerminal for socket2::Socket {
    #[inline]
    fn is_terminal(&self) -> bool {
        use io_lifetimes::AsSocketlike;
        self.as_socketlike_view::<std::net::TcpStream>()
            .is_terminal()
    }
}
