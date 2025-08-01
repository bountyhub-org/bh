pub fn valid_scan_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn valid_workflow_var_key(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
