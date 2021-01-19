//! I/O extension traits.

mod is_terminal;
mod peek;
mod read_ready;

pub use is_terminal::IsTerminal;
pub use peek::{peek_from_bufread, Peek};
pub use read_ready::ReadReady;
