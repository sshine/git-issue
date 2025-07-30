use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Issue priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    /// No priority set (default)
    None = 0,
    /// Urgent priority
    Urgent = 1,
    /// High priority
    High = 2,
    /// Medium priority
    Medium = 3,
    /// Low priority
    Low = 4,
}

impl Priority {
    /// Get the numeric value of the priority
    #[allow(unused)]
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Create priority from numeric value
    #[allow(unused)]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Priority::None),
            1 => Some(Priority::Urgent),
            2 => Some(Priority::High),
            3 => Some(Priority::Medium),
            4 => Some(Priority::Low),
            _ => None,
        }
    }

    /// Get all valid priority values
    #[allow(unused)]
    pub fn all() -> &'static [Priority] {
        &[
            Priority::None,
            Priority::Urgent,
            Priority::High,
            Priority::Medium,
            Priority::Low,
        ]
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::None
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::None => write!(f, "none"),
            Priority::Urgent => write!(f, "urgent"),
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" | "0" => Ok(Priority::None),
            "urgent" | "1" => Ok(Priority::Urgent),
            "high" | "2" => Ok(Priority::High),
            "medium" | "3" => Ok(Priority::Medium),
            "low" | "4" => Ok(Priority::Low),
            _ => Err(format!(
                "Invalid priority '{}'. Valid options: none, urgent, high, medium, low (or 0-4)",
                s
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::None.to_string(), "none");
        assert_eq!(Priority::Urgent.to_string(), "urgent");
        assert_eq!(Priority::High.to_string(), "high");
        assert_eq!(Priority::Medium.to_string(), "medium");
        assert_eq!(Priority::Low.to_string(), "low");
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!("none".parse::<Priority>().unwrap(), Priority::None);
        assert_eq!("urgent".parse::<Priority>().unwrap(), Priority::Urgent);
        assert_eq!("high".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("medium".parse::<Priority>().unwrap(), Priority::Medium);
        assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);

        // Test numeric values
        assert_eq!("0".parse::<Priority>().unwrap(), Priority::None);
        assert_eq!("1".parse::<Priority>().unwrap(), Priority::Urgent);
        assert_eq!("2".parse::<Priority>().unwrap(), Priority::High);
        assert_eq!("3".parse::<Priority>().unwrap(), Priority::Medium);
        assert_eq!("4".parse::<Priority>().unwrap(), Priority::Low);

        // Test case insensitivity
        assert_eq!("URGENT".parse::<Priority>().unwrap(), Priority::Urgent);
        assert_eq!("High".parse::<Priority>().unwrap(), Priority::High);
    }

    #[test]
    fn test_priority_from_str_invalid() {
        assert!("invalid".parse::<Priority>().is_err());
        assert!("5".parse::<Priority>().is_err());
        assert!("".parse::<Priority>().is_err());
    }

    #[test]
    fn test_priority_numeric_conversion() {
        assert_eq!(Priority::None.as_u8(), 0);
        assert_eq!(Priority::Urgent.as_u8(), 1);
        assert_eq!(Priority::High.as_u8(), 2);
        assert_eq!(Priority::Medium.as_u8(), 3);
        assert_eq!(Priority::Low.as_u8(), 4);

        assert_eq!(Priority::from_u8(0), Some(Priority::None));
        assert_eq!(Priority::from_u8(1), Some(Priority::Urgent));
        assert_eq!(Priority::from_u8(2), Some(Priority::High));
        assert_eq!(Priority::from_u8(3), Some(Priority::Medium));
        assert_eq!(Priority::from_u8(4), Some(Priority::Low));
        assert_eq!(Priority::from_u8(5), None);
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(Priority::default(), Priority::None);
    }

    #[test]
    fn test_priority_all() {
        let all = Priority::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&Priority::None));
        assert!(all.contains(&Priority::Urgent));
        assert!(all.contains(&Priority::High));
        assert!(all.contains(&Priority::Medium));
        assert!(all.contains(&Priority::Low));
    }
}
