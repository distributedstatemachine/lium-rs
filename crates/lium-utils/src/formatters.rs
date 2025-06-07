/// Trait for formatting different types of data
pub trait Formatter<T> {
    fn format(&self, input: T) -> String;
}

/// Time formatter for converting seconds to human-readable format
pub struct UptimeFormatter;

impl Formatter<u64> for UptimeFormatter {
    /// Format uptime from seconds to human-readable string
    fn format(&self, seconds: u64) -> String {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let mins = (seconds % 3600) / 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, mins)
        } else if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }
}

/// Cost formatter for calculating cost spent
pub struct CostFormatter;

impl CostFormatter {
    /// Calculate cost spent based on uptime and hourly rate
    pub fn calculate_cost_spent(&self, uptime_seconds: u64, price_per_hour: f64) -> f64 {
        let hours = uptime_seconds as f64 / 3600.0;
        hours * price_per_hour
    }
}

impl Formatter<(u64, f64)> for CostFormatter {
    /// Format cost based on uptime and hourly rate
    fn format(&self, input: (u64, f64)) -> String {
        let (uptime_seconds, price_per_hour) = input;
        let cost = self.calculate_cost_spent(uptime_seconds, price_per_hour);
        format!("${:.2}", cost)
    }
}

// Convenience functions for backward compatibility
pub fn format_uptime(seconds: u64) -> String {
    UptimeFormatter.format(seconds)
}

pub fn calculate_cost_spent(uptime_seconds: u64, price_per_hour: f64) -> f64 {
    let formatter = CostFormatter;
    formatter.calculate_cost_spent(uptime_seconds, price_per_hour)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uptime_formatting() {
        let formatter = UptimeFormatter;

        assert_eq!(formatter.format(3600), "1h 0m");
        assert_eq!(formatter.format(3661), "1h 1m");
        assert_eq!(formatter.format(86400), "1d 0h 0m");
        assert_eq!(formatter.format(90061), "1d 1h 1m");
        assert_eq!(formatter.format(30), "30m");
    }

    #[test]
    fn test_cost_calculation() {
        let formatter = CostFormatter;

        // 1 hour at $2/hour = $2
        assert_eq!(formatter.calculate_cost_spent(3600, 2.0), 2.0);

        // 30 minutes at $2/hour = $1
        assert_eq!(formatter.calculate_cost_spent(1800, 2.0), 1.0);
    }

    #[test]
    fn test_cost_formatting() {
        let formatter = CostFormatter;

        assert_eq!(formatter.format((3600, 1.5)), "$1.50");
        assert_eq!(formatter.format((7200, 0.75)), "$1.50");
    }
}
