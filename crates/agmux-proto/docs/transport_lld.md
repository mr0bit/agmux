# IPC transport connection design

## LLD-IPC-CONNECT-CONTENTION-125
The registered transport owner includes ipc.rs connection/listener helpers.
Windows exposes only a short interval between accepting a named-pipe connection
and preparing the next server instance. Concurrent snapshot, chat and diff
clients can receive ERROR_PIPE_BUSY (231) during this normal transition.

connect_ipc retries only that error asynchronously every 10ms, with a two-second
absolute deadline. Other errors return immediately. A persistent busy endpoint
returns a readable TimedOut error. The retry occurs before framing, handshake
or command transmission; requests and provider prompts are never replayed.
Unix socket behavior is unchanged and healthy Windows opens have no delay.

TEST-VER-IPC-125 uses real native Windows named pipes through public listener
and client APIs: one occupied instance followed by accept must recover and carry
a byte; an instance that remains occupied must time out. The Windows release
job executes these tests before packaging, and the full packaged browser suite
checks that non-Git/shared-view workflows do not surface a busy-pipe banner.
