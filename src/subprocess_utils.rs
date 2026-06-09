use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct CommandResult {
    pub returncode: i32,
    pub stdout: String,
    pub stderr: String,
    pub command: Vec<String>,
}

pub fn run_capture(cmd: &[String], cwd: Option<&Path>) -> io::Result<CommandResult> {
    let mut command = Command::new(&cmd[0]);
    command.args(&cmd[1..]);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let output = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let rc = output.status.code().unwrap_or(-1);
    Ok(CommandResult {
        returncode: rc,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        command: cmd.iter().map(|s| s.to_string()).collect(),
    })
}

pub fn run_stream(cmd: &[String], cwd: Option<&Path>) -> io::Result<CommandResult> {
    let mut command = Command::new(&cmd[0]);
    command.args(&cmd[1..]);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child = command.spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| io::Error::other("stdout not captured"))?;
    let stderr = child.stderr.take().ok_or_else(|| io::Error::other("stderr not captured"))?;

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();

    let mut stdout_reader = BufReader::new(stdout);
    let mut stderr_reader = BufReader::new(stderr);

    let mut line = String::new();
    loop {
        line.clear();
        let n = stdout_reader.read_line(&mut line)?;
        if n > 0 {
            print!("{}", line);
            io::stdout().flush().ok();
            stdout_buf.push_str(&line);
        }

        line.clear();
        let n_err = stderr_reader.read_line(&mut line)?;
        if n_err > 0 {
            eprint!("{}", line);
            io::stderr().flush().ok();
            stderr_buf.push_str(&line);
        }

        if n == 0 && n_err == 0 {
            break;
        }
    }

    let status = child.wait()?;

    Ok(CommandResult {
        returncode: status.code().unwrap_or(-1),
        stdout: stdout_buf,
        stderr: stderr_buf,
        command: cmd.iter().map(|s| s.to_string()).collect(),
    })
}
