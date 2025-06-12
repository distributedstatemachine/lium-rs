use crate::errors::{ParseError, Result, UtilsError};

/// Trait for parsing different types of commands/inputs
pub trait Parser<T> {
    type Output;

    fn parse(&self, input: &str) -> Result<Self::Output>;
}

/// SSH command parser
pub struct SshCommandParser;

impl Parser<String> for SshCommandParser {
    type Output = (String, u16, String); // (host, port, user)

    /// Parse SSH connection command to extract host, port, user
    fn parse(&self, ssh_cmd: &str) -> Result<Self::Output> {
        // Expected formats:
        // "ssh -p <port> <user>@<host>"
        // "ssh <user>@<host> -p <port>"
        // "ssh <user>@<host>"
        let parts: Vec<&str> = ssh_cmd.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(UtilsError::Parse(ParseError::InvalidFormat(
                "Invalid SSH command format".to_string(),
            )));
        }

        let mut port = 22u16;
        let mut user_host = "";

        // First pass: find user@host
        for part in &parts[1..] {
            if part.contains('@') {
                user_host = part;
                break;
            }
        }

        if user_host.is_empty() {
            return Err(UtilsError::Parse(ParseError::InvalidFormat(
                "No user@host found in SSH command".to_string(),
            )));
        }

        // Second pass: find -p flag (can be before or after user@host)
        let mut i = 1;
        while i < parts.len() {
            if parts[i] == "-p" && i + 1 < parts.len() {
                port = parts[i + 1]
                    .parse()
                    .map_err(|_| ParseError::InvalidFormat("Invalid port number".to_string()))?;
                break;
            }
            i += 1;
        }

        // Split user@host
        let user_host_parts: Vec<&str> = user_host.split('@').collect();
        if user_host_parts.len() != 2 {
            return Err(UtilsError::Parse(ParseError::InvalidFormat(
                "Invalid user@host format".to_string(),
            )));
        }

        let user = user_host_parts[0].to_string();
        let host = user_host_parts[1].to_string();

        Ok((host, port, user))
    }
}

// Convenience function for backward compatibility
pub fn parse_ssh_command(ssh_cmd: &str) -> Result<(String, u16, String)> {
    let parser = SshCommandParser;
    parser.parse(ssh_cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_command_parsing() {
        let parser = SshCommandParser;

        // Basic format without port
        let result = parser.parse("ssh root@192.168.1.10").unwrap();
        assert_eq!(result, ("192.168.1.10".to_string(), 22, "root".to_string()));

        // Format with port before user@host
        let result = parser.parse("ssh -p 2222 ubuntu@example.com").unwrap();
        assert_eq!(
            result,
            ("example.com".to_string(), 2222, "ubuntu".to_string())
        );

        // Format with port after user@host (the case that was failing)
        let result = parser.parse("ssh root@198.145.127.160 -p 45480").unwrap();
        assert_eq!(
            result,
            ("198.145.127.160".to_string(), 45480, "root".to_string())
        );

        // Invalid format
        assert!(parser.parse("ssh").is_err());
        assert!(parser.parse("ssh invalid").is_err());
    }
}
