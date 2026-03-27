/// Hashcat process exit codes from `types.h`.
///
/// Negative codes are converted to unsigned 8-bit on Unix (e.g. -1 → 255).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExitCategory {
    Success,
    Exhausted,
    Aborted,
    RuntimeError,
    GpuError,
    InternalError,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ExitCodeInfo {
    pub raw_code: i32,
    pub category: ExitCategory,
    pub retryable: bool,
    pub description: &'static str,
}

/// Normalize a Unix exit code: values 245-255 map to -11 through -1.
#[allow(clippy::arithmetic_side_effects)]
pub fn normalize_exit_code(code: i32) -> i32 {
    if (245..=255).contains(&code) {
        code - 256
    } else {
        code
    }
}

/// Classify a (normalized) hashcat exit code.
pub fn classify_exit_code(raw: i32) -> ExitCodeInfo {
    let code = normalize_exit_code(raw);
    match code {
        0 => ExitCodeInfo {
            raw_code: code,
            category: ExitCategory::Success,
            retryable: false,
            description: "cracked",
        },
        1 => ExitCodeInfo {
            raw_code: code,
            category: ExitCategory::Exhausted,
            retryable: false,
            description: "exhausted",
        },
        2 => ExitCodeInfo {
            raw_code: code,
            category: ExitCategory::Aborted,
            retryable: true,
            description: "aborted by user",
        },
        3 => ExitCodeInfo {
            raw_code: code,
            category: ExitCategory::RuntimeError,
            retryable: true,
            description: "aborted by checkpoint",
        },
        4 => ExitCodeInfo {
            raw_code: code,
            category: ExitCategory::RuntimeError,
            retryable: false,
            description: "aborted by runtime limit",
        },
        -1 => ExitCodeInfo {
            raw_code: code,
            category: ExitCategory::InternalError,
            retryable: false,
            description: "internal error",
        },
        -2 => ExitCodeInfo {
            raw_code: code,
            category: ExitCategory::GpuError,
            retryable: true,
            description: "GPU watchdog alarm / driver error",
        },
        _ => ExitCodeInfo {
            raw_code: code,
            category: ExitCategory::Unknown,
            retryable: false,
            description: "unknown exit code",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_unsigned_to_negative() {
        assert_eq!(normalize_exit_code(255), -1);
        assert_eq!(normalize_exit_code(245), -11);
    }

    #[test]
    fn normalize_preserves_small_codes() {
        assert_eq!(normalize_exit_code(0), 0);
        assert_eq!(normalize_exit_code(1), 1);
        assert_eq!(normalize_exit_code(4), 4);
    }

    #[test]
    fn classify_success() {
        let info = classify_exit_code(0);
        assert_eq!(info.category, ExitCategory::Success);
        assert!(!info.retryable);
    }

    #[test]
    fn classify_exhausted() {
        let info = classify_exit_code(1);
        assert_eq!(info.category, ExitCategory::Exhausted);
    }

    #[test]
    fn classify_unsigned_gpu_error() {
        // -2 arrives as 254 on Unix
        let info = classify_exit_code(254);
        assert_eq!(info.category, ExitCategory::GpuError);
        assert!(info.retryable);
    }
}
