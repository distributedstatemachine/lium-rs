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
        // Expected format: "ssh -p <port> <user>@<host>" or "ssh <user>@<host>"
        let parts: Vec<&str> = ssh_cmd.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(UtilsError::Parse(ParseError::InvalidFormat(
                "Invalid SSH command format".to_string(),
            )));
        }

        let mut port = 22u16;
        let mut user_host = "";

        // Parse command parts
        let mut i = 1; // Skip "ssh"
        while i < parts.len() {
            match parts[i] {
                "-p" => {
                    if i + 1 < parts.len() {
                        port = parts[i + 1].parse().map_err(|_| {
                            ParseError::InvalidFormat("Invalid port number".to_string())
                        })?;
                        i += 2;
                    } else {
                        return Err(UtilsError::Parse(ParseError::InvalidFormat(
                            "Missing port number after -p".to_string(),
                        )));
                    }
                }
                part if part.contains('@') => {
                    user_host = part;
                    break;
                }
                _ => i += 1,
            }
        }

        if user_host.is_empty() {
            return Err(UtilsError::Parse(ParseError::InvalidFormat(
                "No user@host found in SSH command".to_string(),
            )));
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

        // Format with port
        let result = parser.parse("ssh -p 2222 ubuntu@example.com").unwrap();
        assert_eq!(
            result,
            ("example.com".to_string(), 2222, "ubuntu".to_string()))
        ;

        // Invalid format
        assert!(parser.parse("ssh").is_err());
        assert!(parser.parse("ssh invalid").is_err());
    }
}
