//! One Ion process per profile.
//!
//! The first process binds a Unix socket in the user's runtime directory. A
//! later launch connects to it, sends the URLs it was given (plus the Wayland
//! activation token its launcher handed it, so the running window may take
//! focus), and exits. On platforms without Unix sockets every launch is a
//! primary instance.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Protocol version line; bump when the format changes.
const HEADER: &str = "ion-open/1";
/// Requests are a few URLs; anything bigger is not from Ion.
const MAX_REQUEST: u64 = 64 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);

/// What a second launch asks the running instance to do: open `urls` as tabs
/// (or just come to the front when there are none).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub urls: Vec<String>,
    /// `XDG_ACTIVATION_TOKEN` from the launcher, if any (Wayland focus).
    pub activation_token: Option<String>,
}

impl Request {
    /// Build a request from this process's command line and environment.
    pub fn from_launch(args: impl IntoIterator<Item = String>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        Self {
            urls: urls_from_args(args, &cwd),
            activation_token: std::env::var("XDG_ACTIVATION_TOKEN")
                .ok()
                .filter(|t| !t.is_empty()),
        }
    }

    pub fn encode(&self) -> String {
        let mut out = format!("{HEADER}\n");
        if let Some(token) = self.activation_token.as_deref().filter(|t| is_line(t)) {
            out.push_str(&format!("token {token}\n"));
        }
        for url in self.urls.iter().filter(|u| is_line(u)) {
            out.push_str(&format!("url {url}\n"));
        }
        out
    }

    pub fn decode(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()? != HEADER {
            return None;
        }
        let mut request = Self::default();
        for line in lines {
            match line.split_once(' ') {
                Some(("url", url)) if !url.is_empty() => request.urls.push(url.to_owned()),
                Some(("token", token)) if !token.is_empty() => {
                    request.activation_token = Some(token.to_owned())
                }
                // Unknown lines are skipped so newer launchers can add fields.
                _ => {}
            }
        }
        Some(request)
    }
}

fn is_line(s: &str) -> bool {
    !s.is_empty() && !s.contains(['\n', '\r'])
}

/// The URLs in a command line: options are skipped and arguments that name an
/// existing file are made absolute, because the running instance has a
/// different working directory.
pub fn urls_from_args(args: impl IntoIterator<Item = String>, cwd: &Path) -> Vec<String> {
    args.into_iter()
        .filter(|arg| !arg.is_empty() && !arg.starts_with('-'))
        .map(|arg| {
            let path = Path::new(&arg);
            if !path.is_absolute() && !arg.contains("://") && cwd.join(path).exists() {
                cwd.join(path).to_string_lossy().into_owned()
            } else {
                arg
            }
        })
        .collect()
}

/// Where the socket for `profile` lives: `$XDG_RUNTIME_DIR/<app_id>/` on Linux
/// sessions that have one, otherwise a per-user directory under the temp dir
/// (`$TMPDIR` is already per-user on macOS).
pub fn socket_path(app_id: &str, profile: &str) -> PathBuf {
    let dir = match std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) {
        Some(runtime) => PathBuf::from(runtime).join(app_id),
        None => {
            let user = std::env::var("USER").unwrap_or_default();
            std::env::temp_dir().join(format!("{app_id}-{user}"))
        }
    };
    dir.join(format!("{profile}.sock"))
}

/// Outcome of [`claim`].
pub enum Claim {
    /// Another Ion took the request; this process should exit.
    Forwarded,
    /// This process is the primary instance. `None` when the socket could not
    /// be set up; Ion then runs without single-instance handoff.
    Primary(Option<Listener>),
}

#[cfg(unix)]
pub use unix::{Listener, claim};

#[cfg(not(unix))]
pub use fallback::{Listener, claim};

#[cfg(unix)]
mod unix {
    use super::*;
    use std::fs::{DirBuilder, File, OpenOptions, Permissions};
    use std::io::ErrorKind;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
    use std::os::unix::net::{UnixListener, UnixStream};

    /// Hand `request` to a running instance at `path`, or become the primary.
    ///
    /// Runs under an exclusive lock on `<path>.lock`, so two launches at the
    /// same moment cannot both find no listener and both bind: the second one
    /// waits, then finds the first one's socket and hands over to it.
    pub fn claim(path: &Path, request: &Request) -> Claim {
        let Ok(_lock) = lock(path) else {
            return Claim::Primary(None);
        };
        match forward(path, request) {
            Ok(()) => return Claim::Forwarded,
            // Nobody listening: a stale socket from a crash, or no socket.
            Err(e) if matches!(e.kind(), ErrorKind::ConnectionRefused | ErrorKind::NotFound) => {}
            Err(_) => return Claim::Primary(None),
        }
        Claim::Primary(bind(path).ok())
    }

    /// Create the private socket directory and take the claim lock, which is
    /// released when the returned file is dropped (or the process exits).
    fn lock(path: &Path) -> io::Result<File> {
        let dir = path.parent().ok_or(ErrorKind::InvalidInput)?;
        DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
        // Refuse a directory others can reach (for example one pre-created in
        // /tmp by another user).
        if std::fs::metadata(dir)?.permissions().mode() & 0o077 != 0 {
            return Err(ErrorKind::PermissionDenied.into());
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .open(path.with_extension("lock"))?;
        // SAFETY: `file` owns a valid descriptor for the duration of the call.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(file)
    }

    fn forward(path: &Path, request: &Request) -> io::Result<()> {
        let mut stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        stream.write_all(request.encode().as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;
        // Wait for the acknowledgement so the launcher does not exit before
        // the request was read; a timeout here still counts as delivered.
        let mut ack = [0u8; 3];
        let _ = stream.read(&mut ack);
        Ok(())
    }

    /// Only called with the claim lock held, after nobody answered at `path`,
    /// so a socket file still there is stale.
    fn bind(path: &Path) -> io::Result<Listener> {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() != ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        let listener = UnixListener::bind(path)?;
        std::fs::set_permissions(path, Permissions::from_mode(0o600))?;
        Ok(Listener {
            listener,
            path: path.to_owned(),
        })
    }

    /// The primary instance's end of the socket.
    pub struct Listener {
        listener: UnixListener,
        path: PathBuf,
    }

    impl Listener {
        /// Accept requests forever, calling `on_request` for each. Run this on
        /// its own thread.
        pub fn serve(self, mut on_request: impl FnMut(Request)) {
            for stream in self.listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                if let Some(request) = read_request(&mut stream) {
                    let _ = stream.write_all(b"ok\n");
                    on_request(request);
                }
            }
        }
    }

    fn read_request(stream: &mut UnixStream) -> Option<Request> {
        stream.set_read_timeout(Some(TIMEOUT)).ok()?;
        stream.set_write_timeout(Some(TIMEOUT)).ok()?;
        let mut text = String::new();
        stream.take(MAX_REQUEST).read_to_string(&mut text).ok()?;
        Request::decode(&text)
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(not(unix))]
mod fallback {
    use super::*;

    pub fn claim(_path: &Path, _request: &Request) -> Claim {
        Claim::Primary(None)
    }

    pub struct Listener;

    impl Listener {
        pub fn serve(self, _on_request: impl FnMut(Request)) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip() {
        let request = Request {
            urls: vec!["https://example.com/a b".into(), "/tmp/page.html".into()],
            activation_token: Some("kwin-123".into()),
        };
        assert_eq!(Request::decode(&request.encode()), Some(request));
        assert_eq!(
            Request::decode(&Request::default().encode()),
            Some(Request::default())
        );
    }

    #[test]
    fn decode_rejects_foreign_data_and_skips_unknown_lines() {
        assert_eq!(Request::decode("GET / HTTP/1.1\n"), None);
        assert_eq!(Request::decode(""), None);
        let request = Request::decode("ion-open/1\nwindow new\nurl https://a.b\n").unwrap();
        assert_eq!(request.urls, ["https://a.b"]);
    }

    #[test]
    fn encode_drops_values_that_would_break_lines() {
        let request = Request {
            urls: vec!["https://a.b\nurl evil".into(), "https://ok".into()],
            activation_token: Some("x\ny".into()),
        };
        assert_eq!(request.encode(), "ion-open/1\nurl https://ok\n");
    }

    #[test]
    fn args_skip_options_and_absolutize_files() {
        let dir = scratch_dir("args");
        std::fs::write(dir.join("page.html"), "").unwrap();
        let args = [
            "--flag",
            "page.html",
            "example.com",
            "https://x.y",
            "-psn_0_1",
        ];
        let urls = urls_from_args(args.map(String::from), &dir);
        assert_eq!(
            urls,
            [
                dir.join("page.html").to_string_lossy().as_ref(),
                "example.com",
                "https://x.y"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn second_launch_hands_over_to_the_first() {
        let path = scratch_dir("socket").join("Default.sock");

        let Claim::Primary(Some(listener)) = claim(&path, &Request::default()) else {
            panic!("first launch should become primary");
        };
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || listener.serve(move |r| tx.send(r).unwrap()));

        let request = Request {
            urls: vec!["https://example.com".into()],
            activation_token: None,
        };
        assert!(matches!(claim(&path, &request), Claim::Forwarded));
        assert_eq!(rx.recv_timeout(TIMEOUT).unwrap(), request);
    }

    #[cfg(unix)]
    #[test]
    fn simultaneous_launches_elect_one_primary() {
        let path = scratch_dir("race").join("Default.sock");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let launches: Vec<_> = (0..8)
            .map(|_| {
                let (path, barrier) = (path.clone(), barrier.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    match claim(&path, &Request::default()) {
                        Claim::Primary(Some(listener)) => {
                            std::thread::spawn(move || listener.serve(|_| {}));
                            true
                        }
                        Claim::Primary(None) => panic!("socket setup failed"),
                        Claim::Forwarded => false,
                    }
                })
            })
            .collect();
        let primaries = launches
            .into_iter()
            .map(|launch| launch.join().unwrap())
            .filter(|&primary| primary)
            .count();
        assert_eq!(primaries, 1);
    }

    #[cfg(unix)]
    #[test]
    fn stale_socket_is_replaced() {
        let path = scratch_dir("stale").join("Default.sock");
        // Simulate a crash: the socket file stays but nobody listens.
        drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
        assert!(path.exists());
        assert!(matches!(
            claim(&path, &Request::default()),
            Claim::Primary(Some(_))
        ));
    }

    fn scratch_dir(name: &str) -> PathBuf {
        // Unix socket paths are limited to ~100 bytes, and macOS build
        // sandboxes can have long temp dirs.
        let mut base = std::env::temp_dir();
        if base.as_os_str().len() > 60 {
            base = PathBuf::from("/tmp");
        }
        let dir = base.join(format!("ion-t{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        dir
    }
}
