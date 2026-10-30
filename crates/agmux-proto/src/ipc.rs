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
    ClientOptions::new().open(&pipe_name)
}
