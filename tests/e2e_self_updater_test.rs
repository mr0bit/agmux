//! TEST-VER-GUI-003: exercise the updater against a loopback release server.
#![cfg(all(unix, target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use agmux_cli::remote_bootstrapper::RemoteBootstrapper;

struct Fixture {
    dir: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    url: String,
}

impl Fixture {
    fn new(corrupt: bool) -> Self {
        static BUILD: OnceLock<()> = OnceLock::new();
        BUILD.get_or_init(|| {
            assert!(
                Command::new("cargo")
                    .args(["build", "-p", "agmux-cli", "--bin", "agmux"])
                    .status()
                    .unwrap()
                    .success()
            )
        });
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "agmux-update-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("payload")).unwrap();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or(root.join("target"));
        std::fs::copy(target.join("debug/agmux"), dir.join("agmux")).unwrap();
        std::fs::write(dir.join("agmuxd"), b"old daemon").unwrap();
        std::fs::write(dir.join("payload/agmux"), b"new cli").unwrap();
        std::fs::write(dir.join("payload/agmuxd"), b"new daemon").unwrap();
        let archive = dir.join("release.tar.gz");
        assert!(
            Command::new("tar")
                .args(["-czf"])
                .arg(&archive)
                .arg("-C")
                .arg(dir.join("payload"))
                .args(["agmux", "agmuxd"])
                .status()
                .unwrap()
                .success()
        );
        let bytes = std::fs::read(archive).unwrap();
        let digest = if corrupt {
            "0".repeat(64)
        } else {
            RemoteBootstrapper::sha256_hex(&bytes)
        };
        let name = "agmux-v99.0.0-x86_64-unknown-linux-gnu.tar.gz";
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let metadata = format!(r#"{{"tag_name":"v99.0.0","assets":[{{"name":"{name}","browser_download_url":"{base}/archive"}},{{"name":"SHA256SUMS","browser_download_url":"{base}/sums"}}]}}"#).into_bytes();
        let sums = format!("{digest}  {name}\n").into_bytes();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request = [0; 4096];
                        let n = stream.read(&mut request).unwrap();
                        let request = String::from_utf8_lossy(&request[..n]);
                        let body = if request.starts_with("GET /sums ") {
                            &sums
                        } else if request.starts_with("GET /archive ") {
                            &bytes
                        } else {
                            &metadata
                        };
                        write!(
                            stream,
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        )
                        .unwrap();
                        stream.write_all(body).unwrap();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("server: {e}"),
                }
            }
        });
        Self {
            dir,
            stop,
            thread: Some(thread),
            url: format!("{base}/latest"),
        }
    }
    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(self.dir.join("agmux"))
            .args(args)
            .env("AGMUX_UPDATE_URL", &self.url)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

#[test]
fn test_self_updater_check_only_reports_available_update() {
    let f = Fixture::new(false);
    let before = std::fs::read(f.dir.join("agmux")).unwrap();
    let result = f.run(&["update", "--check"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("Update available"));
    assert_eq!(std::fs::read(f.dir.join("agmux")).unwrap(), before);
    assert_eq!(std::fs::read(f.dir.join("agmuxd")).unwrap(), b"old daemon");
}

#[test]
fn test_self_updater_checksum_mismatch_aborts() {
    let f = Fixture::new(true);
    let before = std::fs::read(f.dir.join("agmux")).unwrap();
    let result = f.run(&["update"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("checksum mismatch"));
    assert_eq!(std::fs::read(f.dir.join("agmux")).unwrap(), before);
    assert_eq!(std::fs::read(f.dir.join("agmuxd")).unwrap(), b"old daemon");
}

#[test]
fn test_self_updater_atomic_replacement_success() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new(false);
    let result = f.run(&["--update"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(f.dir.join("agmux")).unwrap(), b"new cli");
    assert_eq!(std::fs::read(f.dir.join("agmuxd")).unwrap(), b"new daemon");
    assert_eq!(
        std::fs::metadata(f.dir.join("agmux"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        std::fs::metadata(f.dir.join("agmuxd"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    assert_eq!(
        std::fs::read(f.dir.join("agmuxd.old")).unwrap(),
        b"old daemon"
    );
}
