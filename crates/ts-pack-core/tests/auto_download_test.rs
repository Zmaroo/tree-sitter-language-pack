use tree_sitter_language_pack::{ProcessConfig, clean_cache, downloaded_languages, process};

#[test]
fn test_auto_download_on_process() {
    // 1. Clean cache to ensure language is missing
    clean_cache().expect("Failed to clean cache");

    let target_lang = "go"; // Choose a common language likely to be in manifest

    // Verify it's not downloaded
    let initial_langs = downloaded_languages();
    assert!(
        !initial_langs.contains(&target_lang.to_string()),
        "Language should not be in cache initially"
    );

    // 2. Call process() - this should trigger auto-download
    let source = "package main\nfunc main() {}";
    let config = ProcessConfig::new(target_lang).all();

    let result = process(source, &config);

    // 3. Verify result
    assert!(
        result.is_ok(),
        "process() should succeed and trigger auto-download: {:?}",
        result.err()
    );
    let intel = result.unwrap();
    assert_eq!(intel.language, target_lang);

    // 4. Verify it's now in the downloaded list
    let final_langs = downloaded_languages();
    assert!(
        final_langs.contains(&target_lang.to_string()),
        "Language should be in cache after process()"
    );
}
