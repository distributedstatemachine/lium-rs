// Docker utilities - stub for now
pub fn placeholder() {}

use crate::errors::{DockerError, LiumError, Result};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

/// Build and push a Docker image
/// Returns the image digest
pub async fn build_and_push_image(
    image_name_with_user: &str,
    dockerfile_path: &Path,
    docker_user: &str,
    docker_token: &str,
) -> Result<String> {
    // First, login to Docker Hub
    login_to_docker(docker_user, docker_token).await?;

    // Build the image
    let image_digest = build_docker_image(image_name_with_user, dockerfile_path).await?;

    // Push the image
    push_docker_image(image_name_with_user).await?;

    Ok(image_digest)
}

/// Login to Docker Hub
async fn login_to_docker(username: &str, token: &str) -> Result<()> {
    let mut login_command = Command::new("docker");
    login_command
        .arg("login")
        .arg("-u")
        .arg(username)
        .arg("--password-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = login_command.spawn().map_err(|e| {
        LiumError::Docker(DockerError::CommandFailed(format!(
            "Failed to start docker login: {}",
            e
        )))
    })?;

    // Write password to stdin
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(token.as_bytes()).map_err(|e| {
            LiumError::Docker(DockerError::CommandFailed(format!(
                "Failed to write password: {}",
                e
            )))
        })?;
    }

    let output = child.wait_with_output().map_err(|e| {
        LiumError::Docker(DockerError::CommandFailed(format!(
            "Failed to wait for docker login: {}",
            e
        )))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LiumError::Docker(DockerError::LoginFailed(
            stderr.to_string(),
        )));
    }

    println!("Successfully logged in to Docker Hub");
    Ok(())
}

/// Build Docker image
async fn build_docker_image(image_name: &str, dockerfile_path: &Path) -> Result<String> {
    let dockerfile_dir = dockerfile_path.parent().ok_or_else(|| {
        LiumError::Docker(DockerError::InvalidPath(
            "Invalid Dockerfile path".to_string(),
        ))
    })?;

    let mut build_command = Command::new("docker");
    build_command
        .arg("build")
        .arg("-t")
        .arg(image_name)
        .arg("--progress=plain")
        .arg("-f")
        .arg(dockerfile_path)
        .arg(dockerfile_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    println!("Building Docker image: {}", image_name);

    let mut child = build_command.spawn().map_err(|e| {
        LiumError::Docker(DockerError::CommandFailed(format!(
            "Failed to start docker build: {}",
            e
        )))
    })?;

    // Stream build output
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => println!("{}", line),
                Err(e) => eprintln!("Error reading build output: {}", e),
            }
        }
    }

    let output = child.wait_with_output().map_err(|e| {
        LiumError::Docker(DockerError::CommandFailed(format!(
            "Failed to wait for docker build: {}",
            e
        )))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LiumError::Docker(DockerError::BuildFailed(
            stderr.to_string(),
        )));
    }

    // Get image digest
    let digest = get_image_digest(image_name).await?;
    println!("Successfully built image with digest: {}", digest);

    Ok(digest)
}

/// Push Docker image
async fn push_docker_image(image_name: &str) -> Result<()> {
    let mut push_command = Command::new("docker");
    push_command
        .arg("push")
        .arg(image_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    println!("Pushing Docker image: {}", image_name);

    let mut child = push_command.spawn().map_err(|e| {
        LiumError::Docker(DockerError::CommandFailed(format!(
            "Failed to start docker push: {}",
            e
        )))
    })?;

    // Stream push output
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => println!("{}", line),
                Err(e) => eprintln!("Error reading push output: {}", e),
            }
        }
    }

    let output = child.wait_with_output().map_err(|e| {
        LiumError::Docker(DockerError::CommandFailed(format!(
            "Failed to wait for docker push: {}",
            e
        )))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LiumError::Docker(DockerError::PushFailed(
            stderr.to_string(),
        )));
    }

    println!("Successfully pushed image: {}", image_name);
    Ok(())
}

/// Get image digest
async fn get_image_digest(image_name: &str) -> Result<String> {
    let mut inspect_command = Command::new("docker");
    inspect_command
        .arg("inspect")
        .arg("--format")
        .arg("{{index .RepoDigests 0}}")
        .arg(image_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = inspect_command.output().map_err(|e| {
        LiumError::Docker(DockerError::CommandFailed(format!(
            "Failed to inspect image: {}",
            e
        )))
    })?;

    if !output.status.success() {
        // If RepoDigests is empty, try getting the image ID instead
        let mut id_command = Command::new("docker");
        id_command
            .arg("inspect")
            .arg("--format")
            .arg("{{.Id}}")
            .arg(image_name);

        let id_output = id_command.output().map_err(|e| {
            LiumError::Docker(DockerError::CommandFailed(format!(
                "Failed to get image ID: {}",
                e
            )))
        })?;

        if id_output.status.success() {
            let image_id = String::from_utf8_lossy(&id_output.stdout)
                .trim()
                .to_string();
            return Ok(image_id);
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LiumError::Docker(DockerError::CommandFailed(format!(
            "Failed to get image digest: {}",
            stderr
        ))));
    }

    let digest = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Extract just the digest part if it's in format "image@sha256:digest"
    if let Some(at_pos) = digest.find('@') {
        Ok(digest[at_pos + 1..].to_string())
    } else {
        Ok(digest)
    }
}

/// Check if Docker is installed and running
pub fn check_docker_available() -> Result<()> {
    let output = Command::new("docker")
        .arg("version")
        .arg("--format")
        .arg("{{.Server.Version}}")
        .output()
        .map_err(|e| {
            LiumError::Docker(DockerError::NotAvailable(format!(
                "Docker not available: {}",
                e
            )))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(LiumError::Docker(DockerError::NotAvailable(format!(
            "Docker not running: {}",
            stderr
        ))));
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    let version = version_output.trim();
    println!("Docker version: {}", version);
    Ok(())
}

/// Validate Docker image name format
pub fn validate_image_name(image_name: &str) -> Result<()> {
    if image_name.is_empty() {
        return Err(LiumError::Docker(DockerError::InvalidImageName(
            "Image name cannot be empty".to_string(),
        )));
    }

    // Basic validation - Docker image names should not contain uppercase letters
    if image_name.chars().any(|c| c.is_uppercase()) {
        return Err(LiumError::Docker(DockerError::InvalidImageName(
            "Image name cannot contain uppercase letters".to_string(),
        )));
    }

    // Should contain a username/organization for Docker Hub
    if !image_name.contains('/') {
        return Err(LiumError::Docker(DockerError::InvalidImageName(
            "Image name should include username (e.g., username/image-name)".to_string(),
        )));
    }

    Ok(())
}

/// Remove local Docker image after push (cleanup)
pub async fn cleanup_local_image(image_name: &str) -> Result<()> {
    let mut remove_command = Command::new("docker");
    remove_command
        .arg("rmi")
        .arg(image_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let _output = remove_command.output(); // Ignore errors for cleanup
    Ok(())
}

// TODO: Add support for building multi-platform images
// TODO: Add support for build arguments
// TODO: Add support for build secrets
// TODO: Add progress reporting for long builds
// TODO: Add support for private registries beyond Docker Hub
