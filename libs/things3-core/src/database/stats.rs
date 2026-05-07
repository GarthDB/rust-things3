//! Aggregate database statistics.

use serde::{Deserialize, Serialize};

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub task_count: u64,
    pub project_count: u64,
    pub area_count: u64,
}

impl DatabaseStats {
    #[must_use]
    pub fn total_items(&self) -> u64 {
        self.task_count + self.project_count + self.area_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_stats_total_items() {
        let stats = DatabaseStats {
            task_count: 10,
            project_count: 5,
            area_count: 3,
        };
        assert_eq!(stats.total_items(), 18);

        let empty_stats = DatabaseStats {
            task_count: 0,
            project_count: 0,
            area_count: 0,
        };
        assert_eq!(empty_stats.total_items(), 0);
    }
}
