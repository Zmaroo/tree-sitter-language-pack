"""Pure helpers for semantic search result processing."""

from __future__ import annotations

import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path


def duplicate_experiment_flags_from_env() -> dict:
    stage = (os.getenv("LM_PROXY_DUPLICATE_ROLLOUT_STAGE") or "stage2").strip().lower()
    raw = (os.getenv("LM_PROXY_DUPLICATE_EXPERIMENTS") or "").strip()
    flags = {
        "boilerplate_variant_suppression": False,
        "canonical_docs_mirror_suppression": False,
        "helper_clone_suppression": False,
        "threshold_struct": _float_env("LM_PROXY_DUPLICATE_THRESHOLD_STRUCT"),
    }
    if stage == "stage1":
        flags["boilerplate_variant_suppression"] = True
    if raw:
        flags["helper_clone_suppression"] = True
    return flags


def duplicate_telemetry_enabled() -> bool:
    raw = os.getenv("LM_PROXY_DUPLICATE_TELEMETRY", "1").strip().lower()
    if raw in {"0", "false", "no", "off"}:
        return False
    return True


def append_duplicate_telemetry_event(trace: dict, *, query: str, tool: str) -> None:
    if not duplicate_telemetry_enabled() or not isinstance(trace, dict):
        return
    target = Path(".runtime/duplicate_telemetry.ndjson")
    event = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "tool": tool,
        "query": query[:500],
    }
    completed = subprocess.run(
        ["python3", "-c", "print('ok')"],
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode == 0:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(json.dumps(event))


def _float_env(name: str) -> float | None:
    raw = (os.getenv(name) or "").strip()
    if not raw:
        return None
    try:
        return float(raw)
    except ValueError:
        return None
