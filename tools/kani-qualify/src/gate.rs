// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use wait_timeout::ChildExt;

#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn executable_sha256() -> Result<String, String> {
    let executable = std::env::current_exe()
        .map_err(|e| format!("cannot identify qualification executable: {e}"))?;
    sha256_file(&executable).map_err(|e| format!("cannot hash qualification executable: {e}"))
}

pub fn current_iso_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let dur = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();

    let days = (secs / 86400) as i64;
    let day_secs = (secs % 86400) as u32;
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;
    let second = day_secs % 60;

    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hour, minute, second)
}

pub struct CommandOutput {
    pub status: std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

/// Success requires normal process termination and EOF on both output streams.
/// The deadline includes pipe collection. Supported tools must keep descendants in
/// their process group; this runner is not an OS sandbox for daemonizing programs.
pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Result<CommandOutput, String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = cmd.spawn().map_err(|e| format!("failed to spawn command: {e}"))?;

    let stdout_pipe = child.stdout.take().ok_or("failed to capture child stdout")?;
    let stderr_pipe = child.stderr.take().ok_or("failed to capture child stderr")?;

    let stop_signal = Arc::new(AtomicBool::new(false));

    let stop_stdout = Arc::clone(&stop_signal);
    let stdout_handle =
        std::thread::spawn(move || read_pipe_deadline_bounded(stdout_pipe, stop_stdout));

    let stop_stderr = Arc::clone(&stop_signal);
    let stderr_handle =
        std::thread::spawn(move || read_pipe_deadline_bounded(stderr_pipe, stop_stderr));

    let started = std::time::Instant::now();
    while (!stdout_handle.is_finished() || !stderr_handle.is_finished())
        && started.elapsed() < timeout
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    let complete = stdout_handle.is_finished() && stderr_handle.is_finished();
    if !complete {
        stop_signal.store(true, Ordering::Relaxed);
    }
    let raw_stdout = stdout_handle
        .join()
        .map_err(|_| "stdout reader thread panicked")
        .and_then(|r| r.map_err(|_| "stdout collection failed or did not reach EOF"));
    let raw_stderr = stderr_handle
        .join()
        .map_err(|_| "stderr reader thread panicked")
        .and_then(|r| r.map_err(|_| "stderr collection failed or did not reach EOF"));

    // Do not reap the root before pipe collection. Its reserved PID prevents group
    // cleanup from targeting a reused process-group identifier after early exit.
    let collected = raw_stdout.is_ok() && raw_stderr.is_ok();
    let waited = if collected {
        child
            .wait_timeout(timeout.saturating_sub(started.elapsed()))
            .map_err(|e| format!("error waiting for child: {e}"))
    } else {
        Ok(None)
    };
    let status = if let Ok(Some(status)) = waited {
        status
    } else {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.kill();
        child.wait().map_err(|e| format!("failed to reap cancelled command: {e}"))?;
        return Err(if let Err(error) = waited {
            error
        } else if !complete || collected {
            format!(
                "command timed out after {}s (including pipe collection)",
                timeout.as_secs_f64()
            )
        } else {
            "command output collection failed before EOF".to_string()
        });
    };
    if status.code().is_none() {
        return Err(format!("command terminated by signal: {status}"));
    }
    let stdout = String::from_utf8(raw_stdout.map_err(str::to_string)?)
        .map_err(|e| format!("command stdout is not UTF-8: {e}"))?;
    let stderr = String::from_utf8(raw_stderr.map_err(str::to_string)?)
        .map_err(|e| format!("command stderr is not UTF-8: {e}"))?;
    Ok(CommandOutput { status, stdout, stderr })
}

#[cfg(unix)]
fn read_pipe_deadline_bounded<R: AsRawFd + Read>(
    mut pipe: R,
    stop: Arc<AtomicBool>,
) -> io::Result<Vec<u8>> {
    let fd = pipe.as_raw_fd();
    // Configure nonblocking read
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 || libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(io::Error::last_os_error());
        }
    }

    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];

    while !stop.load(Ordering::Relaxed) {
        let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        // Poll with short 20ms timeout to promptly check the stop signal
        let res = unsafe { libc::poll(&mut pfd, 1, 20) };
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if res > 0 {
            if pfd.revents & (libc::POLLIN | libc::POLLHUP) != 0 {
                match pipe.read(&mut chunk) {
                    Ok(0) => return Ok(buf),
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            } else if pfd.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
                return Err(io::Error::other("pipe polling failed"));
            }
        } else if res < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                return Err(err);
            }
        }
    }
    Err(io::Error::new(io::ErrorKind::TimedOut, "pipe collection cancelled before EOF"))
}

#[cfg(not(unix))]
fn read_pipe_deadline_bounded<R: Read>(_pipe: R, _stop: Arc<AtomicBool>) -> io::Result<Vec<u8>> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "bounded pipe collection requires Unix"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_process_tree_timeout_bounded() {
        let start = Instant::now();
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("sleep 5 & wait");
        let result = run_with_timeout(cmd, Duration::from_millis(100));
        let elapsed = start.elapsed();

        assert!(result.is_err());
        assert!(elapsed < Duration::from_millis(400));
    }

    #[test]
    fn test_inherited_pipes_fail_without_waiting_for_escaped_child() {
        // A Python child process calls setsid(), inherits stdout and stderr, and sleeps for 2s.
        // run_with_timeout with 100ms timeout must return well within 400ms.
        let start = Instant::now();
        let mut cmd = Command::new("python3");
        cmd.arg("-c").arg(
            r#"
import os, sys, time
pid = os.fork()
if pid == 0:
    os.setsid()
    time.sleep(2)
    sys.exit(0)
sys.exit(0)
"#,
        );
        let result = run_with_timeout(cmd, Duration::from_millis(100));
        let elapsed = start.elapsed();

        // Must complete within fixed cleanup allowance (100ms timeout + < 300ms cleanup)
        assert!(elapsed < Duration::from_millis(500), "elapsed was {:?}", elapsed);
        // A daemonized descendant is outside process-group containment, but its
        // incomplete output must never be admitted as successful execution.
        assert!(result.is_err());
    }
}
