fn env_trimmed(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn contains_normalized(haystack: &str, needle: &str) -> bool {
    haystack.to_ascii_lowercase().contains(needle)
}

fn symbol_filter() -> Option<String> {
    env_trimmed("TS_PACK_DEBUG_PROVENANCE_SYMBOL").map(|value| normalize(&value))
}

fn file_filter() -> Option<String> {
    env_trimmed("TS_PACK_DEBUG_PROVENANCE_FILE").map(|value| normalize(&value.replace('\\', "/")))
}

pub fn provenance_enabled() -> bool {
    symbol_filter().is_some() || file_filter().is_some()
}

pub fn call_matches(
    caller_filepath: &str,
    callee: &str,
    qualified_hint: Option<&str>,
    receiver_hint: Option<&str>,
) -> bool {
    let symbol_match = symbol_filter().is_none_or(|needle| {
        contains_normalized(callee, &needle)
            || qualified_hint.is_some_and(|value| contains_normalized(value, &needle))
            || receiver_hint.is_some_and(|value| contains_normalized(value, &needle))
    });
    let file_match =
        file_filter().is_none_or(|needle| contains_normalized(&caller_filepath.replace('\\', "/"), &needle));
    symbol_match && file_match
}

pub fn file_pair_matches(src_filepath: &str, dst_filepath: &str) -> bool {
    file_filter().is_none_or(|needle| {
        contains_normalized(&src_filepath.replace('\\', "/"), &needle)
            || contains_normalized(&dst_filepath.replace('\\', "/"), &needle)
    })
}

pub fn emit(stage: &str, event: &str, fields: &[(&str, String)]) {
    if !provenance_enabled() {
        return;
    }
    let mut parts = vec![format!("stage={stage}"), format!("event={event}")];
    for (key, value) in fields {
        parts.push(format!("{key}={value:?}"));
    }
    eprintln!("[ts-pack-provenance] {}", parts.join(" "));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_guard() -> &'static Mutex<()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn file_pair_filter_matches_either_side() {
        let _guard = env_guard().lock().unwrap();
        unsafe {
            std::env::set_var("TS_PACK_DEBUG_PROVENANCE_FILE", "src/main.rs");
            std::env::remove_var("TS_PACK_DEBUG_PROVENANCE_SYMBOL");
        }
        assert!(file_pair_matches("src/main.rs", "src/lib.rs"));
        assert!(file_pair_matches("src/lib.rs", "src/main.rs"));
        assert!(!file_pair_matches("src/lib.rs", "src/bin.rs"));
        unsafe {
            std::env::remove_var("TS_PACK_DEBUG_PROVENANCE_FILE");
        }
    }

    #[test]
    fn call_filter_matches_symbol_and_file() {
        let _guard = env_guard().lock().unwrap();
        unsafe {
            std::env::set_var("TS_PACK_DEBUG_PROVENANCE_FILE", "src/main.rs");
            std::env::set_var("TS_PACK_DEBUG_PROVENANCE_SYMBOL", "process");
        }
        assert!(call_matches(
            "src/main.rs",
            "process",
            Some("crate::registry::process"),
            Some("registry"),
        ));
        assert!(!call_matches("src/lib.rs", "process", None, None));
        assert!(!call_matches("src/main.rs", "render", None, None));
        unsafe {
            std::env::remove_var("TS_PACK_DEBUG_PROVENANCE_FILE");
            std::env::remove_var("TS_PACK_DEBUG_PROVENANCE_SYMBOL");
        }
    }
}
