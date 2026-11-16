//! Real UART worker integration using a Rust-created UNIX pseudo-terminal.
use super::*;
use std::{ffi::CStr, fs::File, io::Write, os::fd::FromRawFd, path::PathBuf};
struct Pty {
    master: File,
    _slave: File,
    path: String,
}
impl Pty {
    fn new() -> Self {
        let (mut master, mut slave) = (-1, -1);
        // SAFETY: valid writable fd pointers, optional termios/window/name are null.
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            },
            0,
            "openpty: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: openpty returned unique owned descriptors on success.
        let master = unsafe { File::from_raw_fd(master) };
        let slave_file = unsafe { File::from_raw_fd(slave) };
        let mut name = [0i8; 256];
        // SAFETY: the name buffer is writable and its length is supplied.
        assert_eq!(
            unsafe { libc::ttyname_r(slave, name.as_mut_ptr(), name.len()) },
            0
        );
        // SAFETY: ttyname_r has successfully produced a NUL-terminated name.
        let path = unsafe { CStr::from_ptr(name.as_ptr()) }
            .to_str()
            .unwrap()
            .to_owned();
        let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: tcgetattr initializes the termios object before it is read.
        assert_eq!(unsafe { libc::tcgetattr(slave, settings.as_mut_ptr()) }, 0);
        let mut settings = unsafe { settings.assume_init() };
        // SAFETY: settings is initialized; slave is an open terminal descriptor.
        unsafe {
            libc::cfmakeraw(&mut settings);
            assert_eq!(libc::tcsetattr(slave, libc::TCSANOW, &settings), 0);
        }
        Self {
            master,
            _slave: slave_file,
            path,
        }
    }
    fn write(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).unwrap();
        self.master.flush().unwrap();
    }
}
struct TestDir(PathBuf);
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn wait(feed: &SerialFeed, condition: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let start = Instant::now();
    loop {
        let s = feed.shared.lock().unwrap().clone();
        if condition(&s) {
            return s;
        }
        assert!(
            start.elapsed() < Duration::from_secs(6),
            "UART timeout: open={} seq={} error={}",
            s.open,
            s.sequence,
            s.error
        );
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
#[ignore = "requires local UNIX pseudo-terminal access; run explicitly"]
fn uart_fragment_stale_disconnect_reconnect() {
    let dir = TestDir(std::env::temp_dir().join(format!("flight-pty-{}", std::process::id())));
    std::fs::create_dir(&dir.0).unwrap();
    let link = dir.0.join("uart");
    let mut pty = Pty::new();
    std::os::unix::fs::symlink(&pty.path, &link).unwrap();
    // macOS PTYs reject physical baud configuration. Only this private test uses
    // baud zero; the production entry point is always 115200 / 8N1.
    let feed = SerialFeed::start_with_baud(Some(link.to_str().unwrap().into()), true, 0);
    wait(&feed, |s| s.open);
    pty.write(b"ROLL:12.3,PIT");
    thread::sleep(Duration::from_millis(250));
    assert_eq!(feed.shared.lock().unwrap().sequence, 0);
    pty.write(b"CH:-5.4\r\n");
    let s = wait(&feed, |s| s.sequence == 1);
    assert_eq!(
        s.raw,
        Attitude {
            roll: 12.3,
            pitch: -5.4
        }
    );
    assert!(s.fresh());
    pty.write(b"ROLL:NaN,PITCH:0\n");
    pty.write(&[b'x'; 1000]);
    pty.write(b"\n");
    wait(&feed, |s| s.invalid == 2);
    for i in 0..200 {
        pty.write(format!("ROLL:{},PITCH:0\n", i % 90).as_bytes());
    }
    let s = wait(&feed, |s| s.sequence == 201);
    assert_eq!(s.raw.roll, 19.);
    thread::sleep(Duration::from_millis(800));
    assert!(!feed.shared.lock().unwrap().fresh());
    drop(pty);
    wait(&feed, |s| !s.open);
    let mut pty = Pty::new();
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&pty.path, &link).unwrap();
    let s = wait(&feed, |s| s.open);
    assert!(!s.fresh());
    pty.write(b"ROLL:-30,PITCH:20\n");
    let s = wait(&feed, |s| s.sequence == 202);
    assert!(s.fresh());
    assert_eq!(
        s.raw,
        Attitude {
            roll: -30.,
            pitch: 20.
        }
    );
}
