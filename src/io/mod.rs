//! I/O extension traits.

mod io_ext;
mod is_read_write;
mod is_terminal;
mod peek;
mod read_ready;

pub use io_ext::IoExt;
pub use is_read_write::IsReadWrite;
pub use is_terminal::IsTerminal;
pub use peek::{peek_from_bufread, Peek};
pub use read_ready::ReadReady;
