from typing import TypeAlias

from tree_sitter_language_pack._native import (
    DownloadError,
    LanguageNotFoundError,
    ParseError,
    ProcessConfig,
    QueryError,
    TreeHandle,
    available_languages,
    cache_dir,
    clean_cache,
    configure,
    detect_language,
    detect_language_from_extension,
    detect_language_from_path,
    download,
    download_all,
    downloaded_languages,
    get_binding,
    get_language,
    get_parser,
    has_language,
    init,
    language_count,
    manifest_languages,
    parse_string,
    process,
)

try:
    from tree_sitter_language_pack._native import detect_language_from_content
except Exception:

    def detect_language_from_content(_: bytes | str) -> str | None:
        raise NotImplementedError("detect_language_from_content is not available in this build")


SupportedLanguage: TypeAlias = str

__all__ = [
    "DownloadError",
    "LanguageNotFoundError",
    "ParseError",
    "ProcessConfig",
    "QueryError",
    "SupportedLanguage",
    "TreeHandle",
    "available_languages",
    "cache_dir",
    "clean_cache",
    "configure",
    "detect_language",
    "detect_language_from_content",
    "detect_language_from_extension",
    "detect_language_from_path",
    "download",
    "download_all",
    "downloaded_languages",
    "get_binding",
    "get_language",
    "get_parser",
    "has_language",
    "init",
    "language_count",
    "manifest_languages",
    "parse_string",
    "process",
]
