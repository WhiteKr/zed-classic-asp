//! The server must answer `shutdown` and terminate even while a workspace scan
//! is still pending; it previously indexed everything up front and then hung in
//! `io_threads.join()`.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Duration, Instant};

fn workspace(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("asp-ls-life-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Enough files that the scan is still pending when the first request lands.
    for i in 0..200 {
        std::fs::write(dir.join(format!("f{i}.asp")), "<% Sub Noop\nEnd Sub %>").unwrap();
    }
    dir
}

fn send(stdin: &mut ChildStdin, payload: &str) {
    write!(stdin, "Content-Length: {}\r\n\r\n{payload}", payload.len()).unwrap();
    stdin.flush().unwrap();
}

/// Reads one LSP message and returns its body.
fn recv(stdout: &mut BufReader<ChildStdout>) -> String {
    let mut len = 0;
    loop {
        let mut header = String::new();
        stdout.read_line(&mut header).unwrap();
        if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
            len = value.trim().parse().unwrap();
        }
        if header.trim().is_empty() {
            break;
        }
    }
    let mut body = vec![0u8; len];
    stdout.read_exact(&mut body).unwrap();
    String::from_utf8(body).unwrap()
}

fn start(dir: &PathBuf) -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_asp-ls"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let uri = format!("file://{}", dir.display());
    send(
        &mut stdin,
        &format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"capabilities":{{}},"rootUri":"{uri}"}}}}"#
        ),
    );
    assert!(recv(&mut stdout).contains("\"id\":1"));
    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    );
    (child, stdin, stdout)
}

fn assert_exits(child: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    child.kill().unwrap();
    panic!("server did not exit");
}

#[test]
fn responds_to_shutdown_and_exits() {
    let dir = workspace("shutdown");
    let (mut child, mut stdin, mut stdout) = start(&dir);

    send(
        &mut stdin,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown","params":null}"#,
    );
    assert!(recv(&mut stdout).contains("\"id\":2"));
    send(&mut stdin, r#"{"jsonrpc":"2.0","method":"exit","params":null}"#);

    assert_exits(&mut child);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn exits_on_stdin_eof() {
    let dir = workspace("eof");
    let (mut child, stdin, _stdout) = start(&dir);

    drop(stdin);

    assert_exits(&mut child);
    std::fs::remove_dir_all(&dir).unwrap();
}
