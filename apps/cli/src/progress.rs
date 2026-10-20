//! Terminal-only activity feedback. The engine remains independent of the UI.
use std::{
    io::{self, IsTerminal, Write},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

pub struct Progress {
    worker: Option<(mpsc::Sender<()>, thread::JoinHandle<()>)>,
}

impl Progress {
    pub fn start(label: &'static str, json: bool) -> Self {
        if json || !io::stderr().is_terminal() {
            return Self { worker: None };
        }
        let (stop, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let started = Instant::now();
            let frames = ['|', '/', '-', '\\'];
            let mut frame = 0;
            loop {
                // Lock only while drawing, so other diagnostics can use stderr.
                {
                    let mut stderr = io::stderr().lock();
                    let _ = write!(
                        stderr,
                        "\r\x1b[2K{} {label}… ({}s elapsed)",
                        frames[frame % frames.len()],
                        started.elapsed().as_secs()
                    );
                    let _ = stderr.flush();
                }
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Err(mpsc::RecvTimeoutError::Timeout) => frame += 1,
                    _ => break,
                }
            }
            let mut stderr = io::stderr().lock();
            let _ = write!(stderr, "\r\x1b[2K");
            let _ = stderr.flush();
        });
        Self {
            worker: Some((stop, worker)),
        }
    }
}

impl Drop for Progress {
    fn drop(&mut self) {
        if let Some((stop, worker)) = self.worker.take() {
            let _ = stop.send(());
            let _ = worker.join();
        }
    }
}
