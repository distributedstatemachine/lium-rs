use regex::Regex;
use std::sync::OnceLock;

/// GPU model extraction patterns
static GPU_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_gpu_patterns() -> &'static Vec<Regex> {
    GPU_PATTERNS.get_or_init(|| {
        let patterns = vec![
            r"(?i)(RTX\s*\d+(?:\s*Ti)?(?:\s*Super)?)",
            r"(?i)(GTX\s*\d+(?:\s*Ti)?(?:\s*Super)?)",
            r"(?i)(Tesla\s*[A-Z]\d+)",
            r"(?i)(A\d+(?:\s*SXM)?)",
            r"(?i)(V\d+(?:\s*SXM)?)",
            r"(?i)(H\d+(?:\s*SXM)?)",
            r"(?i)(Quadro\s*\w+)",
            r"(?i)(T4)",
            r"(?i)(P100)",
            r"(?i)(K80)",
        ];

        patterns
            .into_iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect()
    })
}

/// Trait for GPU model extraction - allows for different implementations
pub trait GpuModelExtractor {
    fn extract_gpu_model(&self, machine_name: &str) -> String;
}

/// Default GPU model extractor implementation
pub struct DefaultGpuModelExtractor;

impl GpuModelExtractor for DefaultGpuModelExtractor {
    /// Extract GPU model from machine name using regex patterns
    fn extract_gpu_model(&self, machine_name: &str) -> String {
        // Try specific GPU patterns first
        for pattern in get_gpu_patterns() {
            if let Some(captures) = pattern.captures(machine_name) {
                if let Some(matched) = captures.get(1) {
                    return matched.as_str().to_string();
                }
            }
        }

        // If no pattern matches, try to extract any sequence that looks GPU-like
        if let Ok(re) = Regex::new(r"(?i)([A-Z]+\d+[A-Z]*\d*)") {
            if let Some(captures) = re.captures(machine_name) {
                if let Some(matched) = captures.get(1) {
                    return matched.as_str().to_string();
                }
            }
        }

        // Fallback: return "Unknown"
        "Unknown".to_string()
    }
}

// Convenience function for backward compatibility
pub fn extract_gpu_model(machine_name: &str) -> String {
    DefaultGpuModelExtractor.extract_gpu_model(machine_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_model_extraction() {
        let extractor = DefaultGpuModelExtractor;

        assert_eq!(
            extractor.extract_gpu_model("nvidia-rtx-4090-machine"),
            "RTX 4090"
        );
        assert_eq!(
            extractor.extract_gpu_model("tesla-v100-server"),
            "Tesla V100"
        );
        assert_eq!(
            extractor.extract_gpu_model("gtx-1080-ti-workstation"),
            "GTX 1080 Ti"
        );
        assert_eq!(extractor.extract_gpu_model("unknown-machine"), "Unknown");
    }
}
