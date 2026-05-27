use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

/// Severity level of a classified hashcat output line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Category of a parsed hashcat message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageCategory {
    HashParseError,
    DeviceError,
    OutOfMemory,
    SessionConflict,
    TemperatureWarning,
    BackendError,
    Info,
    Unknown,
}

/// Classified output from hashcat's stdout or stderr.
#[derive(Debug, Clone)]
pub struct ClassifiedMessage {
    pub category: MessageCategory,
    pub severity: Severity,
    pub context: HashMap<String, String>,
}

/// Pattern definition for hashcat output classification.
struct Pattern {
    regex: &'static LazyLock<Regex>,
    category: MessageCategory,
    severity: Severity,
    extractor: fn(&regex::Captures, &mut HashMap<String, String>),
}

const fn noop_extractor(_: &regex::Captures, _: &mut HashMap<String, String>) {}

// ---------------------------------------------------------------------------
// Compiled regex patterns (package-level, compiled once)
// ---------------------------------------------------------------------------

// v6.x: Hashfile '<file>' on line N (<hash>): <error>
// Compile-time constant regex pattern
#[expect(clippy::expect_used)]
static HASH_PARSE_V6: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Hashfile '(.+?)' on line (\d+) \((.+?)\): (.+)").expect("invalid regex")
});

// v7.x: Hash parsing error in hashfile: '<file>' on line N (<hash>): <error>
// Compile-time constant regex pattern
#[expect(clippy::expect_used)]
static HASH_PARSE_V7: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Hash parsing error in hashfile: '(.+?)' on line (\d+) \((.+?)\): (.+)")
        .expect("invalid regex")
});

// Machine-readable: <file>:<line>:<hash>:<error>
// Non-greedy file capture to handle colons in hash values
// Compile-time constant regex pattern
#[expect(clippy::expect_used)]
static HASH_PARSE_MACHINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.+?):(\d+):(.+?):(.+)$").expect("invalid regex"));

// Compile-time constant regex pattern
#[expect(clippy::expect_used)]
static DEVICE_ERROR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Device #(\d+).*(?:error|failed|fault)").expect("invalid regex"));

// Compile-time constant regex pattern
#[expect(clippy::expect_used)]
static OOM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:out of memory|cannot allocate|CUDA_ERROR_OUT_OF_MEMORY)")
        .expect("invalid regex")
});

// Compile-time constant regex pattern
#[expect(clippy::expect_used)]
static SESSION_CONFLICT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"session.*(?:already running|locked|in use)").expect("invalid regex")
});

// Compile-time constant regex pattern
#[expect(clippy::expect_used)]
static TEMPERATURE_WARN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[Tt]emperature.*(?:limit|threshold|abort|warning)").expect("invalid regex")
});

// Compile-time constant regex pattern
#[expect(clippy::expect_used)]
static BACKEND_ERROR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:OpenCL|CUDA|HIP|Metal).*(?:[Ee]rror|[Ff]ailed)").expect("invalid regex")
});

fn extract_hash_parse(caps: &regex::Captures, ctx: &mut HashMap<String, String>) {
    if let Some(file) = caps.get(1) {
        ctx.insert("hashfile".to_owned(), file.as_str().to_owned());
    }
    if let Some(line) = caps.get(2) {
        ctx.insert("line_number".to_owned(), line.as_str().to_owned());
    }
    if let Some(err) = caps.get(4) {
        ctx.insert("error_type".to_owned(), err.as_str().to_owned());
    }
}

fn extract_device(caps: &regex::Captures, ctx: &mut HashMap<String, String>) {
    if let Some(id) = caps.get(1) {
        ctx.insert("device_id".to_owned(), id.as_str().to_owned());
    }
}

static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    vec![
        Pattern {
            regex: &HASH_PARSE_V7,
            category: MessageCategory::HashParseError,
            severity: Severity::Warning,
            extractor: extract_hash_parse,
        },
        Pattern {
            regex: &HASH_PARSE_V6,
            category: MessageCategory::HashParseError,
            severity: Severity::Warning,
            extractor: extract_hash_parse,
        },
        Pattern {
            regex: &HASH_PARSE_MACHINE,
            category: MessageCategory::HashParseError,
            severity: Severity::Warning,
            extractor: extract_hash_parse,
        },
        Pattern {
            regex: &OOM,
            category: MessageCategory::OutOfMemory,
            severity: Severity::Error,
            extractor: noop_extractor,
        },
        Pattern {
            regex: &SESSION_CONFLICT,
            category: MessageCategory::SessionConflict,
            severity: Severity::Error,
            extractor: noop_extractor,
        },
        Pattern {
            regex: &DEVICE_ERROR,
            category: MessageCategory::DeviceError,
            severity: Severity::Error,
            extractor: extract_device,
        },
        Pattern {
            regex: &TEMPERATURE_WARN,
            category: MessageCategory::TemperatureWarning,
            severity: Severity::Warning,
            extractor: noop_extractor,
        },
        Pattern {
            regex: &BACKEND_ERROR,
            category: MessageCategory::BackendError,
            severity: Severity::Error,
            extractor: noop_extractor,
        },
    ]
});

/// Classify a single line of hashcat output (stdout or stderr).
///
/// Returns `None` if the line doesn't match any known pattern.
pub fn classify_line(line: &str) -> Option<ClassifiedMessage> {
    for pattern in PATTERNS.iter() {
        if let Some(caps) = pattern.regex.captures(line) {
            let mut context = HashMap::new();
            (pattern.extractor)(&caps, &mut context);
            return Some(ClassifiedMessage {
                category: pattern.category,
                severity: pattern.severity,
                context,
            });
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn parse_v6_hash_error() {
        let line = "Hashfile 'hashes.txt' on line 3 (abc123): Token length exception";
        let msg = classify_line(line).expect("should match");
        assert_eq!(msg.category, MessageCategory::HashParseError);
        assert_eq!(msg.context["hashfile"], "hashes.txt");
        assert_eq!(msg.context["line_number"], "3");
    }

    #[test]
    fn parse_v7_hash_error() {
        let line =
            "Hash parsing error in hashfile: 'hashes.txt' on line 5 (def456): Separator unmatched";
        let msg = classify_line(line).expect("should match");
        assert_eq!(msg.category, MessageCategory::HashParseError);
        assert_eq!(msg.context["hashfile"], "hashes.txt");
    }

    #[test]
    fn classify_oom() {
        let line = "CUDA_ERROR_OUT_OF_MEMORY in line 42";
        let msg = classify_line(line).expect("should match");
        assert_eq!(msg.category, MessageCategory::OutOfMemory);
        assert_eq!(msg.severity, Severity::Error);
    }

    #[test]
    fn unrecognized_returns_none() {
        assert!(classify_line("some random log output").is_none());
    }
}
