use std::io::Write;
use std::path::Path;

use chrono::Utc;

const SECRET_KEYS: [&str; 9] = [
    "api_key",
    "apikey",
    "authorization",
    "bearer",
    "client_secret",
    "openai_api_key",
    "password",
    "secret",
    "token",
];

pub fn redact_secrets(input: &str) -> String {
    let mut output = input.to_string();
    for key in SECRET_KEYS {
        output = redact_key_values(&output, key);
    }
    output
}

pub fn append_app_log(path: impl AsRef<Path>, message: &str) -> std::io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let redacted = redact_secrets(message);
    writeln!(file, "{} {}", Utc::now().to_rfc3339(), redacted)
}

fn redact_key_values(input: &str, key: &str) -> String {
    let lower = input.to_lowercase();
    let mut result = String::with_capacity(input.len());
    let mut cursor = 0;

    while let Some(relative) = lower[cursor..].find(key) {
        let key_start = cursor + relative;
        let key_end = key_start + key.len();
        let delimiter_start = skip_spaces(input, key_end);
        let Some(delimiter) = input[delimiter_start..].chars().next() else {
            break;
        };

        if delimiter != '=' && delimiter != ':' {
            result.push_str(&input[cursor..key_end]);
            cursor = key_end;
            continue;
        }

        let value_start = skip_spaces(input, delimiter_start + delimiter.len_utf8());
        let (value_end, replacement) = redacted_value(input, value_start);
        result.push_str(&input[cursor..value_start]);
        result.push_str(&replacement);
        cursor = value_end;
    }

    result.push_str(&input[cursor..]);
    result
}

fn skip_spaces(input: &str, mut index: usize) -> usize {
    while let Some(ch) = input[index..].chars().next() {
        if !ch.is_whitespace() {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn redacted_value(input: &str, start: usize) -> (usize, String) {
    let Some(first) = input[start..].chars().next() else {
        return (start, "[REDACTED]".to_string());
    };

    if first == '"' || first == '\'' {
        let quote = first;
        let content_start = start + quote.len_utf8();
        let mut end = content_start;
        for ch in input[content_start..].chars() {
            if ch == quote {
                let close = end + ch.len_utf8();
                return (close, format!("{quote}[REDACTED]{quote}"));
            }
            end += ch.len_utf8();
        }
        return (end, format!("{quote}[REDACTED]"));
    }

    let mut end = start;
    for ch in input[start..].chars() {
        if ch.is_whitespace() || ch == ',' || ch == ';' {
            break;
        }
        end += ch.len_utf8();
    }
    (end, "[REDACTED]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_common_secret_shapes() {
        let message = "OPENAI_API_KEY=sk-live password: hunter2 token=\"abc123\"";

        let redacted = redact_secrets(message);

        assert!(!redacted.contains("sk-live"));
        assert!(!redacted.contains("hunter2"));
        assert!(!redacted.contains("abc123"));
        assert!(redacted.contains("OPENAI_API_KEY=[REDACTED]"));
        assert!(redacted.contains("password: [REDACTED]"));
        assert!(redacted.contains("token=\"[REDACTED]\""));
    }

    #[test]
    fn append_log_writes_redacted_line_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("agenthub.log");

        append_app_log(&log_path, "scan failed with api_key=secret-value").unwrap();

        let contents = std::fs::read_to_string(log_path).unwrap();
        assert!(contents.contains("scan failed"));
        assert!(contents.contains("api_key=[REDACTED]"));
        assert!(!contents.contains("secret-value"));
    }
}
