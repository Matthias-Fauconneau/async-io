//! Bindings to epoll/kqueue/wepoll.
//!
//! On all platforms the I/O reactor is used in oneshot mode.
//!
//! Since this module is just syscalls to the operating system, it is the only place where unsafe
//! code is required.

use std::io;
use std::mem::ManuallyDrop;
use std::net::{Shutdown, TcpStream};
#[cfg(unix)]
use std::os::unix::io::{FromRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{FromRawSocket, RawSocket};

use cfg_if::cfg_if;

/// Calls a libc function and results in `io::Result`.
#[cfg(unix)]
macro_rules! syscall {
    ($fn:ident $args:tt) => {{
        let res = unsafe { libc::$fn $args };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

cfg_if! {
    if #[cfg(any(target_os = "linux", target_os = "android", target_os = "illumos"))] {
        mod epoll;
        pub use self::epoll::*;
    } else if #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "dragonfly",
    ))] {
        mod kqueue;
        pub use self::kqueue::*;
    } else if #[cfg(target_os = "windows")] {
        mod wepoll;
        pub use self::wepoll::*;
    } else {
        compile_error!("async-io does not support this target OS");
    }
}

/// An event reported by epoll/kqueue/wepoll.
pub struct Event {
    /// Key passed when registering interest in the I/O handle.
    pub key: usize,
    /// Is the I/O handle readable?
    pub readable: bool,
    /// Is the I/O handle writable?
    pub writable: bool,
}

/// Shuts down the write side of a socket.
///
/// If this source is not a socket, the `shutdown()` syscall error is ignored.
pub fn shutdown_write(#[cfg(unix)] raw: RawFd, #[cfg(windows)] raw: RawSocket) -> io::Result<()> {
    // This may not be a TCP stream, but that's okay. All we do is call `shutdown()` on the raw
    // descriptor and ignore errors if it's not a socket.
    let res = unsafe {
        #[cfg(unix)]
        let stream = ManuallyDrop::new(TcpStream::from_raw_fd(raw));
        #[cfg(windows)]
        let stream = ManuallyDrop::new(TcpStream::from_raw_socket(raw));
        stream.shutdown(Shutdown::Write)
    };

    // The only actual error may be ENOTCONN, ignore everything else.
    match res {
        Err(err) if err.kind() == io::ErrorKind::NotConnected => Err(err),
        _ => Ok(()),
    }
}
