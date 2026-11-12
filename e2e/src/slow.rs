//! A request sent by hand, its body slowly, as a phone on a weak network
//! sends it: what the server's time limit (`timeout_request`, the configs)
//! does to a body still on its way, which a browser cannot show: Chrome's
//! throttling hands the server the body whole. As an islands app on this
//! stack tests it.

use std::{
    io::{Read, Write},
    net::TcpStream,
    thread,
    time::{Duration, Instant},
};

/// What the server answered and when, counted from the request's start.
#[derive(Debug)]
pub struct Answered {
    /// The answer's first line, `HTTP/1.1 408 Request Timeout`; empty for
    /// none.
    pub status: String,
    pub seconds: f64,
}

/// `body` posted to `path` on the site at `base` (`http://localhost:5163`)
/// at `per_second` bytes a second, a tenth of it every 100 ms, with
/// `headers` besides the length and the host; the answer read as it
/// comes, while the body is still being sent. Blocking: a test runs it in
/// `spawn_blocking`.
///
/// # Errors
/// No connection to the site.
pub fn slow_post(
    base: &str,
    path: &str,
    headers: &[(&str, String)],
    body: &[u8],
    per_second: usize,
) -> std::io::Result<Answered> {
    let host = base.trim_start_matches("http://");
    let mut stream = TcpStream::connect(host)?;
    let mut head = format!("POST {path} HTTP/1.1\r\nHost: {host}\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let started = Instant::now();
    let mut reader = stream.try_clone()?;
    let answered = thread::spawn(move || {
        let _ = reader.set_read_timeout(Some(Duration::from_secs(150)));
        let mut first = [0u8; 128];
        let status = match reader.read(&mut first) {
            Ok(n) if n > 0 => String::from_utf8_lossy(&first[..n])
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned(),
            _ => String::new(),
        };
        Answered {
            status,
            seconds: started.elapsed().as_secs_f64(),
        }
    });
    if stream.write_all(head.as_bytes()).is_ok() {
        for chunk in body.chunks((per_second / 10).max(1)) {
            // An answer before the body's end closes the connection: the
            // rest has nowhere to go.
            if stream.write_all(chunk).is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    answered
        .join()
        .map_err(|_| std::io::Error::other("the reader's thread panicked"))
}
