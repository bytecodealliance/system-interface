<div align="center">
  <h1><code>system-interface</code></h1>

  <p>
    <strong>Extensions to the Rust standard library</strong>
  </p>

  <strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <a href="https://github.com/bytecodealliance/system-interface/actions?query=workflow%3ACI"><img src="https://github.com/bytecodealliance/system-interface/workflows/CI/badge.svg" alt="Github Actions CI Status" /></a>
    <a href="https://crates.io/crates/system-interface"><img src="https://img.shields.io/crates/v/system-interface.svg" alt="crates.io page" /></a>
    <a href="https://docs.rs/system-interface"><img src="https://docs.rs/system-interface/badge.svg" alt="docs.rs docs" /></a>
  </p>
</div>

`system-interface` adds extensions to the Rust standard library, seeking to
stay within the style of [`std`], while exposing additional functionality:

  - [`fs::FileIoExt`] - Extra support for working with files, including
    all the features of [`std::fs::Read`], [`std::fs::Write`],
    [`std::io::Seek`], and [`std::os::unix::fs::FileExt`], but with both
    POSIX-ish and Windows support, and with additional features, including
    `read` and `write` with all combinations of `_vectored`, `_at`, and
    `_exact`/`_all`. If you've ever wanted something like
    [`read_exact_vectored_at`], [`write_all_vectored_at`], or any other
    combination, they're all here, *and* they work on Windows too!
  - [`io::IsTerminal`] - Test whether a given I/O handle refers to a terminal
    (aka a tty).
  - [`io::ReadReady`] - Query the number of bytes ready to be read immediately
    from an I/O handle.

Everything in this crate is portable across popular POSIX-ish platforms and
Windows.

Many of `system-interface`'s features correspond to features in [WASI], and are
designed to work with [`cap-std`], however it's not specific to WASI and can be
used with regular [`std`] too. To separate concerns, all sandboxing and
capability-oriented APIs are left to `cap-std`, so this crate's features are
usable independently.

[`std`]: https://doc.rust-lang.org/std/
[`cap-std`]: https://crates.io/crates/cap-std
[WASI]: https://github.com/WebAssembly/WASI/
[`fs::FileIoExt`]: https://docs.rs/system-interface/latest/system-interface/fs/trait.FileIoExt.html
[`io::IsTerminal`]: https://docs.rs/system-interface/latest/system-interface/io/trait.IsTerminal.html
[`io::ReadReady`]: https://docs.rs/system-interface/latest/system-interface/io/trait.ReadReady.html
[`std::fs::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
[`std::fs::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
[`std::io::Seek`]: https://doc.rust-lang.org/std/io/trait.Seek.html
[`std::os::unix::fs::FileExt`]: https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html
[`read_exact_vectored_at`]: https://docs.rs/system-interface/latest/system-interface/fs/trait.FileIoExt.html#tymethod.read_exact_vectored_at
[`write_all_vectored_at`]: https://docs.rs/system-interface/latest/system-interface/fs/trait.FileIoExt.html#tymethod.write_all_vectored_at
