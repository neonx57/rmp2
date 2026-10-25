use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Warn => "warn",
            Status::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthEntry {
    pub name: String,
    pub status: Status,
    pub detail: String,
}

pub fn report(mpv_binary: &str) -> Vec<HealthEntry> {
    vec![
        probe(mpv_binary, "--version", true),
        probe("yt-dlp", "--version", false),
    ]
}

fn probe(binary: &str, version_arg: &str, required: bool) -> HealthEntry {
    let name = Path::new(binary)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(binary);
    let result = match Command::new(binary).arg(version_arg).output() {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let summary = version_summary(&text);
            Ok(if summary.is_empty() {
                "installed".to_string()
            } else {
                summary
            })
        }
        Ok(out) => {
            let code = out
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into());
            Err(format!("exited with status {code}"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err("not found on PATH".into()),
        Err(e) => Err(e.to_string()),
    };
    entry(name, required, result)
}

fn entry(name: &str, required: bool, result: Result<String, String>) -> HealthEntry {
    match result {
        Ok(detail) => HealthEntry {
            name: name.to_string(),
            status: Status::Ok,
            detail,
        },
        Err(detail) => HealthEntry {
            name: name.to_string(),
            status: if required { Status::Fail } else { Status::Warn },
            detail: if required {
                format!("{detail} (required)")
            } else {
                format!("{detail} (optional: online streams)")
            },
        },
    }
}

fn version_summary(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");
    line.split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_summary_keeps_the_first_two_words() {
        assert_eq!(
            version_summary("mpv 0.38.0 Copyright (C) 2000-2025"),
            "mpv 0.38.0"
        );
        assert_eq!(version_summary("\n\n2025.09.05\n"), "2025.09.05");
        assert_eq!(version_summary(""), "");
    }

    #[test]
    fn missing_required_binary_fails() {
        let e = entry("mpv", true, Err("not found on PATH".into()));
        assert_eq!(e.status, Status::Fail);
        assert_eq!(e.detail, "not found on PATH (required)");
    }

    #[test]
    fn missing_optional_binary_warns() {
        let e = entry("yt-dlp", false, Err("not found on PATH".into()));
        assert_eq!(e.status, Status::Warn);
        assert_eq!(e.detail, "not found on PATH (optional: online streams)");
    }

    #[test]
    fn found_binary_is_ok() {
        let e = entry("mpv", true, Ok("mpv 0.38.0".into()));
        assert_eq!(
            e,
            HealthEntry {
                name: "mpv".into(),
                status: Status::Ok,
                detail: "mpv 0.38.0".into(),
            }
        );
        assert_eq!(Status::Ok.label(), "ok");
    }
}
