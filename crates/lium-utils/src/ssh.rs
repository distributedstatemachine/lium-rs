use crate::errors::{Result, SshError, UtilsError};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

/// Execute a remote command via SSH
pub async fn execute_remote_command(
    host: &str,
    port: u16,
    user: &str,
    private_key_path: &Path,
    command: &str,
    env_vars: Option<HashMap<String, String>>,
) -> Result<(String, String, i32)> {
    let mut ssh_command = Command::new("ssh");

    // SSH options
    ssh_command
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-i")
        .arg(private_key_path)
        .arg("-p")
        .arg(port.to_string());

    // Add environment variables to the command if provided
    let final_command = if let Some(env) = env_vars {
        let env_exports: Vec<String> = env
            .iter()
            .map(|(k, v)| format!("export {}='{}'", k, v.replace('\'', "'\"'\"'")))
            .collect();

        if env_exports.is_empty() {
            command.to_string()
        } else {
            format!("{}; {}", env_exports.join("; "), command)
        }
    } else {
        command.to_string()
    };

    ssh_command
        .arg(format!("{}@{}", user, host))
        .arg(final_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = ssh_command.spawn().map_err(|e| {
        UtilsError::Ssh(SshError::CommandFailed(format!(
            "Failed to execute SSH command: {}",
            e
        )))
    })?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    // Read stdout
    for line in stdout_reader.lines() {
        match line {
            Ok(line) => {
                println!("{}", line); // Stream to console
                stdout_lines.push(line);
            }
            Err(e) => eprintln!("Error reading stdout: {}", e),
        }
    }

    // Read stderr
    for line in stderr_reader.lines() {
        match line {
            Ok(line) => {
                eprintln!("{}", line); // Stream to console
                stderr_lines.push(line);
            }
            Err(e) => eprintln!("Error reading stderr: {}", e),
        }
    }

    let exit_status = child.wait().map_err(|e| {
        UtilsError::Ssh(SshError::CommandFailed(format!(
            "Failed to wait for SSH command: {}",
            e
        )))
    })?;

    let exit_code = exit_status.code().unwrap_or(-1);
    let stdout_str = stdout_lines.join("\n");
    let stderr_str = stderr_lines.join("\n");

    Ok((stdout_str, stderr_str, exit_code))
}

/// Upload a file via SFTP/SCP
pub async fn upload_file_sftp(
    host: &str,
    port: u16,
    user: &str,
    private_key_path: &Path,
    local_path: &Path,
    remote_path: &str,
) -> Result<()> {
    // Use scp command for file upload
    let mut scp_command = Command::new("scp");

    scp_command
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-i")
        .arg(private_key_path)
        .arg("-P")
        .arg(port.to_string())
        .arg(local_path)
        .arg(format!("{}@{}:{}", user, host, remote_path));

    let output = scp_command.output().map_err(|e| {
        UtilsError::Ssh(SshError::TransferFailed(format!(
            "Failed to execute SCP command: {}",
            e
        )))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(UtilsError::Ssh(SshError::TransferFailed(format!(
            "SCP upload failed: {}",
            stderr
        ))));
    }

    Ok(())
}

/// Download a file via SFTP/SCP
pub async fn download_file_sftp(
    host: &str,
    port: u16,
    user: &str,
    private_key_path: &Path,
    remote_path: &str,
    local_path: &Path,
) -> Result<()> {
    // Use scp command for file download
    let mut scp_command = Command::new("scp");

    scp_command
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-i")
        .arg(private_key_path)
        .arg("-P")
        .arg(port.to_string())
        .arg(format!("{}@{}:{}", user, host, remote_path))
        .arg(local_path);

    let output = scp_command.output().map_err(|e| {
        UtilsError::Ssh(SshError::TransferFailed(format!(
            "Failed to execute SCP command: {}",
            e
        )))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(UtilsError::Ssh(SshError::TransferFailed(format!(
            "SCP download failed: {}",
            stderr
        ))));
    }

    Ok(())
}

/// Execute interactive SSH session (for CLI ssh command)
pub fn execute_ssh_interactive(
    host: &str,
    port: u16,
    user: &str,
    private_key_path: &Path,
) -> Result<()> {
    let mut ssh_command = Command::new("ssh");

    ssh_command
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-i")
        .arg(private_key_path)
        .arg("-p")
        .arg(port.to_string())
        .arg(format!("{}@{}", user, host));

    let status = ssh_command.status().map_err(|e| {
        UtilsError::Ssh(SshError::ConnectionFailed(format!(
            "Failed to execute SSH command: {}",
            e
        )))
    })?;

    if !status.success() {
        return Err(UtilsError::Ssh(SshError::ConnectionFailed(
            "SSH session failed".to_string(),
        )));
    }

    Ok(())
}

/// Execute SCP command (for CLI scp command)
pub fn execute_scp_command(
    host: &str,
    port: u16,
    user: &str,
    private_key_path: &Path,
    source: &str,
    destination: &str,
    is_upload: bool,
) -> Result<()> {
    let mut scp_command = Command::new("scp");

    scp_command
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-i")
        .arg(private_key_path)
        .arg("-P")
        .arg(port.to_string());

    if is_upload {
        scp_command
            .arg(source)
            .arg(format!("{}@{}:{}", user, host, destination));
    } else {
        scp_command
            .arg(format!("{}@{}:{}", user, host, source))
            .arg(destination);
    }

    let status = scp_command.status().map_err(|e| {
        UtilsError::Ssh(SshError::TransferFailed(format!(
            "Failed to execute SCP command: {}",
            e
        )))
    })?;

    if !status.success() {
        return Err(UtilsError::Ssh(SshError::TransferFailed(
            "SCP command failed".to_string(),
        )));
    }

    Ok(())
}

/// Execute rsync command (for CLI rsync command)
pub fn execute_rsync_command(
    host: &str,
    port: u16,
    user: &str,
    private_key_path: &Path,
    source: &str,
    destination: &str,
    options: Option<&str>,
    is_upload: bool,
) -> Result<()> {
    let mut rsync_command = Command::new("rsync");

    // Default rsync options
    rsync_command.arg("-avz").arg("--progress");

    // Add custom options if provided
    if let Some(opts) = options {
        for opt in opts.split_whitespace() {
            rsync_command.arg(opt);
        }
    }

    // SSH options for rsync
    let ssh_opts = format!(
        "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -i {} -p {}",
        private_key_path.display(),
        port
    );
    rsync_command.arg("-e").arg(ssh_opts);

    if is_upload {
        rsync_command
            .arg(source)
            .arg(format!("{}@{}:{}", user, host, destination));
    } else {
        rsync_command
            .arg(format!("{}@{}:{}", user, host, source))
            .arg(destination);
    }

    let status = rsync_command.status().map_err(|e| {
        UtilsError::Ssh(SshError::TransferFailed(format!(
            "Failed to execute rsync command: {}",
            e
        )))
    })?;

    if !status.success() {
        return Err(UtilsError::Ssh(SshError::TransferFailed(
            "Rsync command failed".to_string(),
        )));
    }

    Ok(())
}

/// Ensure remote directory exists
pub async fn ensure_remote_directory(
    host: &str,
    port: u16,
    user: &str,
    private_key_path: &Path,
    remote_path: &str,
) -> Result<()> {
    let command = format!("mkdir -p '{}'", remote_path.replace('\'', "'\"'\"'"));
    let (_, _, exit_code) =
        execute_remote_command(host, port, user, private_key_path, &command, None).await?;

    if exit_code != 0 {
        return Err(UtilsError::Ssh(SshError::CommandFailed(format!(
            "Failed to create remote directory: {}",
            remote_path
        ))));
    }

    Ok(())
}

// TODO: Add support for SSH agent authentication
// TODO: Add support for password authentication (if needed)
// TODO: Add progress callbacks for file transfers
// TODO: Add connection pooling/reuse for multiple operations
