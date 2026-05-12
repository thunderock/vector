use std::io::{self, Read, Write};
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tokio::sync::mpsc;

use crate::error::PtyError;

#[derive(Debug, Clone)]
pub struct SpawnCommand {
    pub argv: Option<Vec<String>>,
    pub cwd: Option<PathBuf>,
    pub rows: u16,
    pub cols: u16,
    pub env: Vec<(String, String)>,
}

pub struct LocalPty {
    master: Box<dyn MasterPty + Send>,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    writer_tx: mpsc::Sender<Vec<u8>>,
    reader_rx: Option<mpsc::Receiver<Vec<u8>>>,
}

impl LocalPty {
    // SpawnCommand is consumed by ownership because Plan 02-04 wraps this in
    // `Domain::spawn(SpawnCommand)` which hands the value off — no point cloning.
    #[allow(clippy::needless_pass_by_value)]
    pub fn spawn(shell: &Path, cmd: SpawnCommand) -> Result<Self, PtyError> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: cmd.rows,
                cols: cmd.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::OpenPty(e.to_string()))?;

        let mut builder = match &cmd.argv {
            Some(argv) if !argv.is_empty() => {
                let mut b = CommandBuilder::new(&argv[0]);
                for a in &argv[1..] {
                    b.arg(a);
                }
                b
            }
            _ => CommandBuilder::new(shell),
        };
        if let Some(ref cwd) = cmd.cwd {
            builder.cwd(cwd);
        }
        // CORE-05: TERM=xterm-256color before user env so user can override.
        builder.env("TERM", "xterm-256color");
        for (k, v) in &cmd.env {
            builder.env(k, v);
        }

        let child = pair
            .slave
            .spawn_command(builder)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        // Pitfall 3: keep slave open in parent -> child can't get EOF -> zombie.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Io(io::Error::other(e.to_string())))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Io(io::Error::other(e.to_string())))?;

        let (reader_tx, reader_rx) = mpsc::channel::<Vec<u8>>(64);
        let (writer_tx, mut writer_rx) = mpsc::channel::<Vec<u8>>(64);

        // Reader: blocking read in spawn_blocking, push to mpsc.
        tokio::task::spawn_blocking(move || {
            let mut reader = reader;
            let mut buf = vec![0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let chunk = buf[..n].to_vec();
                        // blocking_send -> natural backpressure (Pitfall 7 / ANTI 6).
                        if reader_tx.blocking_send(chunk).is_err() {
                            break;
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(_) => break,
                }
            }
        });

        // Writer: drain mpsc into the blocking master writer.
        tokio::task::spawn_blocking(move || {
            let mut writer = writer;
            while let Some(bytes) = writer_rx.blocking_recv() {
                if writer.write_all(&bytes).is_err() {
                    break;
                }
                let _ = writer.flush();
            }
        });

        Ok(Self {
            master: pair.master,
            child: Some(child),
            writer_tx,
            reader_rx: Some(reader_rx),
        })
    }

    pub fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<(), PtyError> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: px_w,
                pixel_height: px_h,
            })
            .map_err(|e| PtyError::Resize(e.to_string()))
    }

    // Takes `&mut self` so the trait-object wrapper's `async fn write(&mut self)`
    // future is Send (LocalPty itself is !Sync because `Box<dyn MasterPty + Send>`
    // is not Sync — but &mut LocalPty is Send via LocalPty: Send).
    pub async fn write(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.writer_tx
            .send(bytes.to_vec())
            .await
            .map_err(|_| PtyError::WriteClosed)
    }

    pub fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
        self.reader_rx.take()
    }

    /// Child shell PID. Returns None after `wait()` consumes the child.
    #[must_use]
    pub fn child_pid(&self) -> Option<i32> {
        self.child
            .as_ref()
            .and_then(|c| c.process_id())
            .and_then(|u| i32::try_from(u).ok())
    }

    /// Raw fd of the master PTY for `tcgetpgrp` / SIGWINCH ioctls.
    /// Fd is owned by LocalPty (closed on Drop); callers must NOT close it.
    /// Returns None on platforms where portable-pty cannot expose the fd.
    #[must_use]
    pub fn master_raw_fd(&self) -> Option<RawFd> {
        self.master.as_raw_fd()
    }

    pub async fn wait(&mut self) -> Result<Option<i32>, PtyError> {
        let mut child = self.child.take().ok_or(PtyError::AlreadyWaited)?;
        let status = tokio::task::spawn_blocking(move || child.wait())
            .await
            .map_err(|e| PtyError::Io(io::Error::other(e.to_string())))?
            .map_err(|e| PtyError::Io(io::Error::other(e.to_string())))?;
        Ok(status.exit_code().try_into().ok())
    }
}

impl Drop for LocalPty {
    fn drop(&mut self) {
        // Closing the master fd makes the kernel deliver SIGHUP to the child pgrp;
        // we also kill+wait explicitly so we don't rely on Drop order between
        // master fd and the spawn_blocking reader.
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
