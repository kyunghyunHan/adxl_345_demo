//! Optional UNIX serial integration test. Requires python3 and access to local PTYs.
use super::*;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
};

struct Simulator {
    child: Child,
    output: BufReader<std::process::ChildStdout>,
    path: std::path::PathBuf,
}
impl Simulator {
    fn command(&mut self, command: &str) {
        let input = self.child.stdin.as_mut().unwrap();
        writeln!(input, "{command}").unwrap();
        input.flush().unwrap();
        let mut reply = String::new();
        self.output.read_line(&mut reply).unwrap();
        assert_eq!(reply.trim(), "OK");
    }
}
impl Drop for Simulator {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.path);
    }
}
fn wait_for(feed: &SerialFeed, condition: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let start = Instant::now();
    loop {
        let snapshot = feed.shared.lock().unwrap().clone();
        if condition(&snapshot) {
            return snapshot;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "Serial condition timed out: open={} seq={} error={}",
            snapshot.open,
            snapshot.sequence,
            snapshot.error
        );
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
#[ignore = "requires python3 and UNIX pseudo-terminal access; run explicitly"]
fn uart_partial_stale_disconnect_reconnect_and_latest_mailbox() {
    let path = std::env::temp_dir().join(format!("attitude-lab-{}-uart", std::process::id()));
    let script = r#"
import os,sys,tty
path=sys.argv[1]
def create():
    master,slave=os.openpty()
    tty.setraw(slave)
    if os.path.lexists(path): os.unlink(path)
    os.symlink(os.ttyname(slave),path)
    return master,slave
master,slave=create()
print('OK',flush=True)
for line in sys.stdin:
    cmd=line.strip()
    if cmd=='fragment': os.write(master,b'ROLL:12.3,PIT')
    elif cmd=='rest': os.write(master,b'CH:5.4\n')
    elif cmd=='invalid': os.write(master,b'ROLL:NaN,PITCH:0\n'+b'x'*1000+b'\n')
    elif cmd=='burst':
        for i in range(200): os.write(master,('ROLL:%s,PITCH:0\n'%(i%90)).encode())
    elif cmd=='close': os.close(master); os.close(slave)
    elif cmd=='reopen': master,slave=create()
    elif cmd=='sample': os.write(master,b'ROLL:-30,PITCH:20\n')
    print('OK',flush=True)
"#;
    let mut child = Command::new("python3")
        .args(["-u", "-c", script, path.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("python3 must be installed");
    let output = BufReader::new(child.stdout.take().unwrap());
    let mut sim = Simulator {
        child,
        output,
        path: path.clone(),
    };
    let mut ready = String::new();
    sim.output.read_line(&mut ready).unwrap();
    assert_eq!(ready.trim(), "OK");
    let feed = SerialFeed::start_with_baud(Some(path.to_str().unwrap().into()), 0);
    wait_for(&feed, |s| s.open);
    sim.command("fragment");
    thread::sleep(Duration::from_millis(250));
    assert_eq!(feed.shared.lock().unwrap().sequence, 0);
    sim.command("rest");
    let s = wait_for(&feed, |s| s.sequence == 1);
    assert_eq!(
        s.raw,
        Attitude {
            roll: 12.3,
            pitch: 5.4
        }
    );
    assert_eq!(s.status(Instant::now()), "CONNECTED");
    sim.command("invalid");
    wait_for(&feed, |s| s.invalid == 2);
    sim.command("burst");
    let s = wait_for(&feed, |s| s.sequence == 201);
    assert_eq!(s.raw.roll, 19.);
    // No renderer drained the notification channel during the burst; the mailbox still has the newest value.
    assert!(feed.rx.lock().unwrap().try_recv().is_ok());
    thread::sleep(Duration::from_millis(2100));
    assert_eq!(
        feed.shared.lock().unwrap().status(Instant::now()),
        "WAITING FOR UART"
    );
    sim.command("close");
    wait_for(&feed, |s| !s.open);
    sim.command("reopen");
    wait_for(&feed, |s| s.open);
    assert_eq!(
        feed.shared.lock().unwrap().status(Instant::now()),
        "WAITING FOR UART"
    );
    sim.command("sample");
    let s = wait_for(&feed, |s| s.sequence == 202);
    assert_eq!(
        s.raw,
        Attitude {
            roll: -30.,
            pitch: 20.
        }
    );
    assert_eq!(s.status(Instant::now()), "CONNECTED");
}
