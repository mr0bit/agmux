//! Cross-platform IPC transport abstraction (Unix Domain Sockets on Unix, Named Pipes on Windows).

use std::path::Path;

#[cfg(unix)]
pub type IpcClientStream = tokio::net::UnixStream;
#[cfg(unix)]
pub type IpcServerStream = tokio::net::UnixStream;

#[cfg(unix)]
pub struct IpcListener {
    inner: tokio::net::UnixListener,
}

#[cfg(unix)]
impl IpcListener {
    pub fn bind(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let inner = tokio::net::UnixListener::bind(path)?;
        Ok(Self { inner })
    }

    pub async fn accept_stream(&self) -> std::io::Result<IpcServerStream> {
        let (stream, _) = self.inner.accept().await?;
        Ok(stream)
    }
}

#[cfg(unix)]
pub async fn connect_ipc(path: impl AsRef<Path>) -> std::io::Result<IpcClientStream> {
    tokio::net::UnixStream::connect(path).await
}

#[cfg(windows)]
pub type IpcClientStream = tokio::net::windows::named_pipe::NamedPipeClient;
#[cfg(windows)]
pub type IpcServerStream = tokio::net::windows::named_pipe::NamedPipeServer;

#[cfg(windows)]
pub fn to_pipe_name(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.starts_with(r"\\.\pipe\") {
        s.to_string()
    } else {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("agmuxd");
        format!(r"\\.\pipe\{}", name)
    }
}

#[cfg(windows)]
pub struct IpcListener {
    pipe_name: String,
    server: std::sync::Mutex<Option<tokio::net::windows::named_pipe::NamedPipeServer>>,
}

#[cfg(windows)]
impl IpcListener {
    pub fn bind(path: impl AsRef<Path>) -> std::io::Result<Self> {
        use tokio::net::windows::named_pipe::ServerOptions;
        let pipe_name = to_pipe_name(path.as_ref());
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&pipe_name)?;
        Ok(Self {
            pipe_name,
            server: std::sync::Mutex::new(Some(server)),
        })
    }

    pub async fn accept_stream(&self) -> std::io::Result<IpcServerStream> {
        use tokio::net::windows::named_pipe::ServerOptions;
        let current_server = {
            let mut guard = self.server.lock().unwrap();
            guard.take().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::Other, "Named pipe listener closed")
            })?
        };

        current_server.connect().await?;

        // Prepare next server pipe instance for incoming connections
        let next_server = ServerOptions::new().create(&self.pipe_name)?;
        {
            let mut guard = self.server.lock().unwrap();
            *guard = Some(next_server);
        }

        Ok(current_server)
    }
}

#[cfg(windows)]
pub async fn connect_ipc(path: impl AsRef<Path>) -> std::io::Result<IpcClientStream> {
    use tokio::net::windows::named_pipe::ClientOptions;
    let pipe_name = to_pipe_name(path.as_ref());
    // A server instance can be occupied until accept prepares its successor.
    // Retry only before connecting: no handshake or command is replayed.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match ClientOptions::new().open(&pipe_name) {
            Ok(stream) => return Ok(stream),
            Err(error) if error.raw_os_error() == Some(231) => {
                if tokio::time::Instant::now() >= deadline {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Agmux background service is busy. Try again shortly.",
                    ));
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

#[cfg(all(test, windows))]
mod connection_tests {
    use super::*;
    use std::{
        path::PathBuf,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn endpoint() -> PathBuf {
        PathBuf::from(format!(
            "agmux-contention-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    // TEST-VER-IPC-125: native public-seam recovery, not a mocked error code.
    #[tokio::test]
    async fn busy_pipe_recovers_without_replaying_a_request() {
        let path = endpoint();
        let listener = IpcListener::bind(&path).unwrap();
        let first = connect_ipc(&path).await.unwrap();
        let next_path = path.clone();
        let next = tokio::spawn(async move { connect_ipc(next_path).await });
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !next.is_finished(),
            "a busy endpoint must wait for its next instance"
        );
        let _first_server = listener.accept_stream().await.unwrap();
        let mut client = tokio::time::timeout(Duration::from_secs(5), next)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let mut server = listener.accept_stream().await.unwrap();
        client.write_all(b"x").await.unwrap();
        let mut byte = [0];
        tokio::time::timeout(Duration::from_secs(5), server.read_exact(&mut byte))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&byte, b"x");
        drop(first);
    }

    // TEST-VER-IPC-125: permanent contention is bounded and understandable.
    #[tokio::test]
    async fn busy_pipe_deadline_is_bounded_and_readable() {
        let path = endpoint();
        let _listener = IpcListener::bind(&path).unwrap();
        let _occupied = connect_ipc(&path).await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), connect_ipc(&path))
            .await
            .unwrap();
        let error = result.err().expect("occupied pipe must time out");
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(error.to_string().contains("background service is busy"));
    }
}
