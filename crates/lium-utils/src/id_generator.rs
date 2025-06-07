use uuid::Uuid;

// Word lists for generating human-readable IDs
const ADJECTIVES: &[&str] = &[
    "brave", "calm", "clever", "cool", "eager", "fast", "gentle", "happy", "keen", "lively",
    "nice", "proud", "quick", "quiet", "smart", "swift", "warm", "wise", "young", "bold", "bright",
    "clean", "fresh", "grand", "great", "kind", "light", "lucky", "merry", "mild", "neat", "plain",
    "rich", "sharp", "shiny", "silly", "small", "super", "sweet", "thick",
];

const NOUNS: &[&str] = &[
    "ant", "bat", "bee", "cat", "cow", "dog", "elk", "fox", "gnu", "hen", "jay", "owl", "pig",
    "rat", "ram", "yak", "ape", "bug", "cub", "doe", "eel", "fly", "hog", "kid", "lab", "mom",
    "pup", "sun", "web", "zoo", "ace", "ash", "bay", "box", "day", "eye", "gem", "ink", "key",
    "oak",
];

/// Trait for generating IDs - allows for different implementations and testing
pub trait IdGenerator {
    fn generate_uuid(&self) -> String;
    fn generate_human_id(&self, uuid: &str) -> String;
    fn is_valid_uuid(&self, uuid_str: &str) -> bool;
}

/// Default ID generator implementation
pub struct DefaultIdGenerator;

impl IdGenerator for DefaultIdGenerator {
    /// Generate a new UUID v4
    fn generate_uuid(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Generate a human-readable ID from a UUID
    /// Format: adjective-noun-hexsuffix (e.g., "brave-cat-a1b2")
    fn generate_human_id(&self, uuid: &str) -> String {
        // Create a simple hash from the UUID for consistent selection
        let hash = uuid.chars().enumerate().fold(0u32, |acc, (i, c)| {
            acc.wrapping_add((c as u32) * (i as u32 + 1))
        });

        let adj_idx = (hash % ADJECTIVES.len() as u32) as usize;
        let noun_idx = ((hash / ADJECTIVES.len() as u32) % NOUNS.len() as u32) as usize;

        // Get last 4 characters of UUID for hex suffix
        let hex_suffix = uuid
            .chars()
            .rev()
            .take(4)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();

        format!("{}-{}-{}", ADJECTIVES[adj_idx], NOUNS[noun_idx], hex_suffix)
    }

    /// Validate UUID format
    fn is_valid_uuid(&self, uuid_str: &str) -> bool {
        Uuid::parse_str(uuid_str).is_ok()
    }
}

// Convenience functions for backward compatibility
pub fn generate_uuid() -> String {
    DefaultIdGenerator.generate_uuid()
}

pub fn generate_human_id(uuid: &str) -> String {
    DefaultIdGenerator.generate_human_id(uuid)
}

pub fn is_valid_uuid(uuid_str: &str) -> bool {
    DefaultIdGenerator.is_valid_uuid(uuid_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockIdGenerator {
        uuid: String,
    }

    impl IdGenerator for MockIdGenerator {
        fn generate_uuid(&self) -> String {
            self.uuid.clone()
        }

        fn generate_human_id(&self, _uuid: &str) -> String {
            "test-id-1234".to_string()
        }

        fn is_valid_uuid(&self, _uuid_str: &str) -> bool {
            true
        }
    }

    #[test]
    fn test_human_id_generation() {
        let generator = DefaultIdGenerator;
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let human_id = generator.generate_human_id(uuid);

        assert!(human_id.contains('-'));
        assert_eq!(human_id.matches('-').count(), 2);
    }

    #[test]
    fn test_uuid_validation() {
        let generator = DefaultIdGenerator;
        assert!(generator.is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!generator.is_valid_uuid("invalid-uuid"));
    }
}
