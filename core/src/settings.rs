use crate::Database;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobSettings {
    pub worker_timeout: u64,
    pub max_flags: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutorSettings {
    pub concurrent_limit: usize,
    pub worker_timeout: u64,
    pub max_flags: usize,
    pub skip_on_flag: bool,
    pub sequential_per_target: bool,
}

pub fn parse_setting_u64(value: Option<String>, default: u64) -> u64 {
    value.and_then(|v| v.parse().ok()).unwrap_or(default)
}

pub fn parse_setting_usize(value: Option<String>, default: usize) -> usize {
    value.and_then(|v| v.parse().ok()).unwrap_or(default)
}

pub fn parse_setting_bool(value: Option<String>, default: bool) -> bool {
    value.map(|v| v == "true").unwrap_or(default)
}

pub async fn load_job_settings(db: &Database) -> JobSettings {
    let worker_timeout = parse_setting_u64(db.get_setting("worker_timeout").await.ok(), 60);
    let max_flags = parse_setting_usize(db.get_setting("max_flags_per_job").await.ok(), 50);
    JobSettings { worker_timeout, max_flags }
}

pub fn compute_timeout(exploit_timeout_secs: i32, worker_timeout: u64) -> u64 {
    if exploit_timeout_secs > 0 {
        exploit_timeout_secs as u64
    } else {
        worker_timeout
    }
}

pub async fn load_executor_settings(db: &Database) -> ExecutorSettings {
    let concurrent_limit = parse_setting_usize(db.get_setting("concurrent_limit").await.ok(), 10);
    let worker_timeout = parse_setting_u64(db.get_setting("worker_timeout").await.ok(), 60);
    let max_flags = parse_setting_usize(db.get_setting("max_flags_per_job").await.ok(), 50);
    let skip_on_flag = parse_setting_bool(db.get_setting("skip_on_flag").await.ok(), false);
    let sequential_per_target = parse_setting_bool(db.get_setting("sequential_per_target").await.ok(), false);
    ExecutorSettings { concurrent_limit, worker_timeout, max_flags, skip_on_flag, sequential_per_target }
}

#[cfg(test)]
mod tests {
    use super::{compute_timeout, parse_setting_bool, parse_setting_u64, parse_setting_usize};

    #[test]
    fn parse_setting_u64_falls_back() {
        assert_eq!(parse_setting_u64(None, 60), 60);
        assert_eq!(parse_setting_u64(Some("bad".to_string()), 60), 60);
        assert_eq!(parse_setting_u64(Some("30".to_string()), 60), 30);
    }

    #[test]
    fn parse_setting_usize_falls_back() {
        assert_eq!(parse_setting_usize(None, 50), 50);
        assert_eq!(parse_setting_usize(Some("bad".to_string()), 50), 50);
        assert_eq!(parse_setting_usize(Some("25".to_string()), 50), 25);
    }

    #[test]
    fn parse_setting_bool_falls_back() {
        assert!(!parse_setting_bool(None, false));
        assert!(!parse_setting_bool(Some("bad".to_string()), false));
        assert!(parse_setting_bool(Some("true".to_string()), false));
    }

    #[test]
    fn compute_timeout_prefers_exploit() {
        assert_eq!(compute_timeout(10, 60), 10);
    }

    #[test]
    fn compute_timeout_falls_back_to_worker() {
        assert_eq!(compute_timeout(0, 60), 60);
    }
}
