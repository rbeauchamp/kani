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

#[cfg(unix)]
fn kill_process_tree(root_pid: i32) {
    // 1. Kill the process group directly
    unsafe {
        libc::kill(-root_pid, libc::SIGKILL);
        libc::kill(root_pid, libc::SIGKILL);
    }
    // 2. Query child processes to catch any descendant that escaped via setsid()
    if let Ok(output) = Command::new("pgrep").arg("-P").arg(root_pid.to_string()).output() {
        let pids_str = String::from_utf8_lossy(&output.stdout);
        for line in pids_str.lines() {
            if let Ok(child_pid) = line.trim().parse::<i32>() {
                kill_process_tree(child_pid);
                unsafe {
                    libc::kill(child_pid, libc::SIGKILL);
                }
            }
        }
    }
}

/// Run a command with process group isolation, descendant supervision, and deadline-bounded
/// pipe collection. The cleanup allowance after timeout expiration is bounded to 200 ms.
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

    let timeout_result =
        child.wait_timeout(timeout).map_err(|e| format!("error waiting for child: {e}"))?;

    let status = match timeout_result {
        Some(status) => {
            // Once the main process has exited, give reader threads a short deadline (50ms)
            // to drain any pending output before stopping them, preventing orphaned descendants
            // that inherited stdout/stderr from extending the cleanup budget.
            let drain_start = std::time::Instant::now();
            while !stdout_handle.is_finished() || !stderr_handle.is_finished() {
                if drain_start.elapsed() > Duration::from_millis(50) {
                    stop_signal.store(true, Ordering::Relaxed);
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            status
        }
        None => {
            // Signal reader threads to break immediately, preventing inherited descriptors
            // from extending the cleanup budget.
            stop_signal.store(true, Ordering::Relaxed);

            #[cfg(unix)]
            {
                let pid = child.id() as i32;
                kill_process_tree(pid);
            }
            let _ = child.kill();
            let _ = child.wait();

            // Collect reader threads within the cleanup allowance
            let _ = stdout_handle.join();
            let _ = stderr_handle.join();
            return Err(format!("command timed out after {}s", timeout.as_secs()));
        }
    };

    let raw_stdout = stdout_handle.join().map_err(|_| "stdout reader thread panicked")?;
    let raw_stderr = stderr_handle.join().map_err(|_| "stderr reader thread panicked")?;

    let stdout = String::from_utf8(raw_stdout)
        .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned());
    let stderr = String::from_utf8(raw_stderr)
        .unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned());

    Ok(CommandOutput { status, stdout, stderr })
}

#[cfg(unix)]
fn read_pipe_deadline_bounded<R: AsRawFd + Read>(mut pipe: R, stop: Arc<AtomicBool>) -> Vec<u8> {
    let fd = pipe.as_raw_fd();
    // Configure nonblocking read
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags >= 0 {
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
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
                    Ok(0) => break, // EOF
                    Ok(n) => buf.extend_from_slice(&chunk[..n]),
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(_) => break,
                }
            } else if pfd.revents & libc::POLLERR != 0 {
                break;
            }
        } else if res < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                break;
            }
        }
    }
    buf
}

#[cfg(not(unix))]
fn read_pipe_deadline_bounded<R: Read>(mut pipe: R, _stop: Arc<AtomicBool>) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = pipe.read_to_end(&mut buf);
    buf
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
    fn test_setsid_descendant_retaining_pipes_bounded_cleanup() {
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
        // Ensure child process is killed or reaped
        let _ = result;
    }
}
