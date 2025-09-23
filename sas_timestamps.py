#!/usr/bin/env python3
# -*- coding: utf-8 -*-

"""sas_timestamps
==================

Deterministically set creation/modified/access times so root-level folders (and everything
inside them) are ordered newest→oldest by a stable mapping from folder name → timestamp.

This script merges the behaviour of the previous `SAS-TIMESTAMPS-V2.py` and
`timestampdiscrepancyfixtest1.py` utilities into a single configurable CLI.

Key behaviour (configurable via flags):

* **Timeline modes**
  * `local-forward` (default): starts at **12/31/2098 00:00:00 local time** and moves forward by
    category/day. This mirrors the FAT/VFAT-focused script.
  * `fixed-utc`: starts from **2099-01-01 07:59:59Z** and subtracts offsets, matching the original
    UTC-based script.
* **Spacing** – `--seconds-between-items` and `--slots-per-category` control the spacing and slot
  budget. Defaults keep the FAT-safe two-second cadence but you can specify `--seconds-between-items 1`
  to match the older one-second spacing.
* **Stable tie-break nudge** – opt into the deterministic 0/1 second nudge with `--stable-nudge`
  (used by the original UTC script).
* **FAT-safe snapping** – `--fat-safe` snaps timestamps to even seconds (0 µs) so FAT/VFAT devices
  cannot round the values differently.
* **PS2 bias** – `--ps2-bias-seconds` applies an additional signed offset so a skewed PS2 RTC can
  display the same times Windows shows.
* **Dry-run output** – `--dry-run` together with `--dry-run-format {csv,tsv}` writes the planned
  timeline (newest→oldest) without touching the filesystem.

NOTE: Requires Windows (uses SetFileTime via ctypes).
"""

import argparse
import ctypes
import csv
import os
import sys
from datetime import datetime, timezone, timedelta

# =========================
# ===== USER CONFIG =======
# =========================
# FAT-safe default spacing: 2 seconds (FAT mtime has 2-second granularity)
# Keep these defaults in sync with `crates/psu-packer/src/sas.rs` (TimestampRules::default).
DEFAULT_SECONDS_BETWEEN_ITEMS = 2

# Big slot budget so each name gets a unique second within its category, even with many items.
# 43,200 slots × 2 seconds = 86,400 seconds (exactly one day) per category. Nice for viewing in
# a file browser as each category will land on its own day.
DEFAULT_SLOTS_PER_CATEGORY   = 43_200

# Runtime-adjustable spacing settings (overridden by CLI options)
SECONDS_BETWEEN_ITEMS = DEFAULT_SECONDS_BETWEEN_ITEMS
SLOTS_PER_CATEGORY = DEFAULT_SLOTS_PER_CATEGORY

# Enable deterministic 0/1 second nudge (off by default; enabled via --stable-nudge)
ENABLE_STABLE_NUDGE = False

# Comma-separated lists of names (no prefixes) to be treated as if they belong to these categories.
# Edit these to add your own folder names (case-insensitive). Whitespace is ignored.
# Keep this alias map in sync with `psu_packer::sas::canonical_category_aliases()`.
UNPREFIXED_IN_CATEGORY_CSV = {
    "APP_":      "OSDXMB, XEBPLUS",
    "APPS":      "",  # exact "APPS" is its own name
    "PS1_":      "",
    "EMU_":      "",
    "GME_":      "",
    "DST_":      "",
    "DBG_":      "",
    "RAA_":      "RESTART, POWEROFF",
    "RTE_":      "NEUTRINO",
    "SYS_":      "BOOT",
    "ZZY_":      "EXPLOITS",
    "ZZZ_":      "BM, MATRIXTEAM, OPL",
}

# Category order (newest → oldest).
CATEGORY_ORDER = [
    "APP_",
    "APPS",
    "PS1_",
    "EMU_",
    "GME_",
    "DST_",
    "DBG_",
    "RAA_",
    "RTE_",
    "DEFAULT",   # non-matching fallbacks
    "SYS_",
    "ZZY_",
    "ZZZ_",
]

# =========================
# ===== END CONFIG  =======
# =========================

# --- Build quick-lookup from CSV config ---
def _parse_csv(s: str):
    return {x.strip().upper() for x in s.split(",") if x.strip()} if s else set()

UNPREFIXED_MAP = {k: _parse_csv(v) for k, v in UNPREFIXED_IN_CATEGORY_CSV.items()}

# --- Windows FILETIME helpers (ctypes) ---
_EPOCH_AS_FILETIME = 11644473600
_HUNDREDS_OF_NS = 10_000_000

kernel32 = ctypes.WinDLL('kernel32', use_last_error=True)

CreateFileW = kernel32.CreateFileW
CreateFileW.argtypes = [
    ctypes.c_wchar_p,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_void_p,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_void_p
]
CreateFileW.restype = ctypes.c_void_p

SetFileTime = kernel32.SetFileTime
SetFileTime.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
SetFileTime.restype = ctypes.c_int

CloseHandle = kernel32.CloseHandle
CloseHandle.argtypes = [ctypes.c_void_p]
CloseHandle.restype = ctypes.c_int

GENERIC_WRITE = 0x40000000
FILE_SHARE_READ = 0x00000001
FILE_SHARE_WRITE = 0x00000002
FILE_SHARE_DELETE = 0x00000004
OPEN_EXISTING = 3
FILE_FLAG_BACKUP_SEMANTICS = 0x02000000  # needed to open directories

class FILETIME(ctypes.Structure):
    _fields_ = [("dwLowDateTime", ctypes.c_uint32),
                ("dwHighDateTime", ctypes.c_uint32)]

def _dt_to_filetime(dt_utc: datetime) -> FILETIME:
    unix_seconds = dt_utc.timestamp()
    ft = int((unix_seconds + _EPOCH_AS_FILETIME) * _HUNDREDS_OF_NS)
    return FILETIME(ft & 0xFFFFFFFF, ft >> 32)

def _set_times_windows(path: str, dt_utc: datetime) -> None:
    handle = CreateFileW(
        path,
        GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        None,
        OPEN_EXISTING,
        FILE_FLAG_BACKUP_SEMANTICS,
        None
    )
    if handle == ctypes.c_void_p(-1).value or handle is None:
        raise OSError(f"Failed to open handle for: {path} (WinError {ctypes.get_last_error()})")
    try:
        ft = _dt_to_filetime(dt_utc)
        if not SetFileTime(handle,
                           ctypes.byref(ft),
                           ctypes.byref(ft),
                           ctypes.byref(ft)):
            raise OSError(f"SetFileTime failed for: {path} (WinError {ctypes.get_last_error()})")
    finally:
        CloseHandle(handle)

# --- Category + name → slot mapping ---
CHARSET = tuple(" 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_-.")
CHAR_INDEX = {ch: i for i, ch in enumerate(CHARSET)}
BASE = len(CHARSET)

CATEGORY_BLOCK_SECONDS = SLOTS_PER_CATEGORY * SECONDS_BETWEEN_ITEMS
CATEGORY_INDEX = {name: idx for idx, name in enumerate(CATEGORY_ORDER)}

def _effective_category_key(eff: str) -> str:
    if eff.startswith("APP_"): return "APP_"
    if eff == "APPS": return "APPS"
    if eff.startswith("PS1_"): return "PS1_"
    if eff.startswith("EMU_"): return "EMU_"
    if eff.startswith("GME_"): return "GME_"
    if eff.startswith("DST_"): return "DST_"
    if eff.startswith("DBG_"): return "DBG_"
    if eff.startswith("RAA_"): return "RAA_"
    if eff.startswith("RTE_"): return "RTE_"
    if eff.startswith("SYS_") or eff == "SYS": return "SYS_"
    if eff.startswith("ZZY_"): return "ZZY_"
    if eff.startswith("ZZZ_"): return "ZZZ_"
    return "DEFAULT"

def _category_label_for_effective(eff: str) -> str:
    key = _effective_category_key(eff)
    return "DEFAULT" if key == "DEFAULT" else (key if key == "APPS" else f"{key}*")

def _payload_for_effective(eff: str) -> str:
    """Use only the part after the category key, ignoring dashes for ordering."""
    key = _effective_category_key(eff)
    if key == "APPS": return "APPS"
    if key == "DEFAULT": return eff.replace("-", "")
    payload = eff[len(key):] if eff.startswith(key) else eff
    return payload.replace("-", "")

def _lex_fraction(payload: str) -> float:
    """Map payload string to [0,1) preserving lexicographic order (dashes ignored already)."""
    s = payload.upper()
    total = 0.0
    scale = 1.0
    for ch in s[:128]:
        scale *= BASE
        code = CHAR_INDEX.get(ch, BASE - 1)
        total += (code + 1) / scale
    return total

def _normalize_name_for_rules(name: str) -> str:
    """
    Return the EFFECTIVE (possibly prefixed) name for all logic.
    We do not strip dashes here; dashes are ignored later during ordering only.
    """
    n = name.strip().upper()

    # 1) User-configured "no-prefix" names
    for cat_key, names in UNPREFIXED_MAP.items():
        if n in names:
            return "APPS" if cat_key == "APPS" else f"{cat_key}{n}"

    # 2) Built-in defaults
    if n in ("OSDXMB", "XEBPLUS"):
        return "APP_" + n
    if n in ("RESTART", "POWEROFF"):
        return "RAA_" + n
    if n == "NEUTRINO":
        return "RTE_" + n
    if n == "BOOT":
        return "SYS_BOOT"
    if n == "EXPLOITS":
        return "ZZY_EXPLOITS"
    if n in ("BM", "MATRIXTEAM", "OPL"):
        return "ZZZ_" + n

    # 3) Otherwise, leave as-is
    return n

def _category_priority_index(effective: str) -> int:
    key = _effective_category_key(effective)
    return CATEGORY_INDEX[key]

def _slot_index_within_category(effective: str) -> int:
    """
    Compute the within-category slot index using the EFFECTIVE name (e.g., 'SYS_BOOT').
    Dashes are ignored for ordering; underscores are kept.
    """
    payload = _payload_for_effective(effective)
    frac = _lex_fraction(payload)  # [0,1)
    slot = int(frac * SLOTS_PER_CATEGORY)
    if slot >= SLOTS_PER_CATEGORY:
        slot = SLOTS_PER_CATEGORY - 1
    return slot

def _stable_hash01(s: str) -> int:
    """Very small deterministic hash returning 0 or 1."""

    h = 2166136261
    for ch in s:
        h ^= ord(ch)
        h = (h * 16777619) & 0xFFFFFFFF
    return h & 1


def _deterministic_offset_seconds(folder_name: str):
    """Return (offset_seconds, cat_idx, slot, effective_name)."""

    eff = _normalize_name_for_rules(folder_name)
    cat_idx = _category_priority_index(eff)
    slot = _slot_index_within_category(eff)

    nudge = _stable_hash01(eff) if ENABLE_STABLE_NUDGE else 0

    cat_offset = cat_idx * CATEGORY_BLOCK_SECONDS
    name_offset = (slot * SECONDS_BETWEEN_ITEMS) + nudge
    return cat_offset + name_offset, cat_idx, slot, eff

# --- Timestamp planners ---
FIXED_BASE_UTC = datetime(2099, 1, 1, 7, 59, 59, tzinfo=timezone.utc)


def _recompute_spacing(seconds_between: int, slots_per_category: int) -> None:
    global SECONDS_BETWEEN_ITEMS, SLOTS_PER_CATEGORY, CATEGORY_BLOCK_SECONDS

    SECONDS_BETWEEN_ITEMS = seconds_between
    SLOTS_PER_CATEGORY = slots_per_category
    CATEGORY_BLOCK_SECONDS = SLOTS_PER_CATEGORY * SECONDS_BETWEEN_ITEMS


def _anchor_local_datetime() -> datetime:
    """
    Return the local anchor datetime (used as ANCHOR_START in the forward timeline).
    """
    local_naive = datetime(2098, 12, 31, 0, 0, 0)
    local_tz = datetime.now().astimezone().tzinfo
    return local_naive.replace(tzinfo=local_tz)

def _planned_timestamp_for_folder(folder_name: str, timeline: str):
    """Return (utc_dt, effective_name, category_label, cat_idx, slot_idx, offset_sec)."""

    offset_sec, cat_idx, slot_idx, eff = _deterministic_offset_seconds(folder_name)

    if timeline == "fixed-utc":
        base_utc = FIXED_BASE_UTC
        ts_utc = datetime.fromtimestamp(base_utc.timestamp() - offset_sec, tz=timezone.utc)
    else:
        anchor_local = _anchor_local_datetime()
        planned_local = anchor_local + timedelta(seconds=offset_sec)
        ts_utc = planned_local.astimezone(timezone.utc)

    return ts_utc, eff, _category_label_for_effective(eff), cat_idx, slot_idx, offset_sec

# --- FAT-safe snapping ---
def _snap_even_second(dt: datetime) -> datetime:
    """
    Force timestamp to an even second and zero microseconds.
    """
    dt = dt.replace(microsecond=0)
    if dt.second % 2 == 1:
        dt = dt + timedelta(seconds=1)
    return dt

# --- Walk and set ---
def _set_folder_and_contents_times(root_folder: str, dt_utc: datetime, verbose=False):
    # Recurse and set times, then set root last (so mtime doesn't bump)
    for dirpath, dirnames, filenames in os.walk(root_folder):
        for fname in filenames:
            fpath = os.path.join(dirpath, fname)
            try:
                _set_times_windows(fpath, dt_utc)
                if verbose: print(f"Set file  : {fpath}")
            except Exception as e:
                print(f"[WARN] Could not set times for file {fpath}: {e}", file=sys.stderr)
        for dname in dirnames:
            dpath = os.path.join(dirpath, dname)
            try:
                _set_times_windows(dpath, dt_utc)
                if verbose: print(f"Set dir   : {dpath}")
            except Exception as e:
                print(f"[WARN] Could not set times for dir  {dpath}: {e}", file=sys.stderr)
    try:
        _set_times_windows(root_folder, dt_utc)
        if verbose: print(f"Set ROOT  : {root_folder}")
    except Exception as e:
        print(f"[WARN] Could not set times for ROOT {root_folder}: {e}", file=sys.stderr)

# --- Dry-run writer ---
def _write_dryrun(plan, base_path: str, fmt: str, verbose=False) -> str:
    """Write the dry-run plan to CSV or TSV (newest→oldest)."""

    cwd = os.getcwd()
    out_path = os.path.join(cwd, f"SAS-TIMESTAMPS-dryrun.{fmt}")
    plan_sorted = sorted(plan, key=lambda x: x[1], reverse=True)

    dialect = "excel" if fmt == "csv" else "excel-tab"
    with open(out_path, "w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f, dialect=dialect)
        writer.writerow([
            "Order",
            "Category",
            "CatIndex",
            "Slot",
            "OffsetSec",
            "Name",
            "EffectiveName",
            "Payload",
            "LocalTime",
            "UTC",
            "FullPath",
        ])
        for idx, (name, ts_utc, eff, cat_lbl, cat_idx, slot_idx, offset_sec) in enumerate(plan_sorted, start=1):
            payload = _payload_for_effective(eff)
            local_str = ts_utc.astimezone().strftime("%m/%d/%Y %H:%M:%S %Z")
            utc_str = ts_utc.strftime("%Y-%m-%d %H:%M:%S UTC")
            full = os.path.join(base_path, name)
            writer.writerow([
                idx,
                cat_lbl,
                cat_idx,
                slot_idx,
                offset_sec,
                name,
                eff,
                payload,
                local_str,
                utc_str,
                full,
            ])

    if verbose:
        print(f"[DRY-RUN] Wrote plan to: {out_path}")
        print(f"[DRY-RUN] {len(plan_sorted)} root folders listed (newest → oldest).")
    return out_path

# --- Main ---
def main():
    ap = argparse.ArgumentParser(
        description="Deterministically set ctime/mtime recursively by folder name and category."
    )
    ap.add_argument("path", nargs="?", default=".",
                    help="Top-level directory containing the root folders to timestamp (default: current dir).")
    ap.add_argument("--dry-run", action="store_true",
                    help="Do NOT modify timestamps; write SAS-TIMESTAMPS-dryrun.[csv|tsv] in the current working directory.")
    ap.add_argument("--verbose", action="store_true", help="Extra logging.")
    ap.add_argument("--fat-safe", action="store_true",
                    help="Snap all times to even seconds (0 µs) to match FAT/VFAT mtime precision.")
    ap.add_argument("--ps2-bias-seconds", type=int, default=0,
                    help="Signed seconds to bias planned timestamps so PS2 display matches Windows. "
                         "Example: -3563 to counter a +59m23s skew on PS2.")
    ap.add_argument("--timeline", choices=["local-forward", "fixed-utc"], default="local-forward",
                    help="Timeline strategy. 'local-forward' matches the FAT-safe tool; 'fixed-utc' matches the original UTC tool.")
    ap.add_argument("--seconds-between-items", type=int, default=DEFAULT_SECONDS_BETWEEN_ITEMS,
                    help="Spacing (seconds) between projects inside a category (default: %(default)s).")
    ap.add_argument("--slots-per-category", type=int, default=DEFAULT_SLOTS_PER_CATEGORY,
                    help="Slot budget per category (default: %(default)s).")
    ap.add_argument("--dry-run-format", choices=["csv", "tsv"], default="tsv",
                    help="File format for the --dry-run output (default: %(default)s).")
    ap.add_argument("--stable-nudge", dest="stable_nudge", action="store_true",
                    help="Apply a deterministic 0/1-second nudge to break ties (used by the original UTC script).")
    ap.add_argument("--no-stable-nudge", dest="stable_nudge", action="store_false",
                    help="Disable the deterministic 0/1-second nudge (default).")
    ap.set_defaults(stable_nudge=False)

    args = ap.parse_args()

    if args.seconds_between_items <= 0:
        print("--seconds-between-items must be positive.", file=sys.stderr)
        sys.exit(1)
    if args.slots_per_category <= 0:
        print("--slots-per-category must be positive.", file=sys.stderr)
        sys.exit(1)

    _recompute_spacing(args.seconds_between_items, args.slots_per_category)

    global ENABLE_STABLE_NUDGE
    ENABLE_STABLE_NUDGE = args.stable_nudge

    base_path = os.path.abspath(args.path)
    if not os.path.isdir(base_path):
        print(f"Not a directory: {base_path}", file=sys.stderr)
        sys.exit(1)

    root_folders = [d for d in os.listdir(base_path) if os.path.isdir(os.path.join(base_path, d))]

    if args.verbose:
        print(f"Found {len(root_folders)} root folders under {base_path}")

    plan = []
    for name in root_folders:
        try:
            ts, eff, cat_lbl, cat_idx, slot_idx, offset_sec = _planned_timestamp_for_folder(name, args.timeline)

            # Apply free-form PS2 bias (to counter PS2 skew so displays match)
            if args.ps2_bias_seconds:
                ts = ts + timedelta(seconds=args.ps2_bias_seconds)

            # FAT-safe snapping to even seconds (prevents copy/rounding drift)
            if args.fat_safe:
                ts = _snap_even_second(ts)

        except Exception as e:
            print(f"[WARN] Failed to compute timestamp for {name}: {e}", file=sys.stderr)
            continue
        plan.append((name, ts, eff, cat_lbl, cat_idx, slot_idx, offset_sec))

    if args.dry_run:
        out_path = _write_dryrun(plan, base_path, args.dry_run_format, verbose=args.verbose)
        print(f"Dry-run complete. Plan written to: {out_path}")
        return

    for name, ts, eff, cat_lbl, cat_idx, slot_idx, offset_sec in plan:
        full = os.path.join(base_path, name)
        if args.verbose:
            print(f"=== {name} [{cat_lbl}] cat={cat_idx} slot={slot_idx} offset={offset_sec}s -> "
                  f"{ts.astimezone().strftime('%m/%d/%Y %H:%M:%S %Z')} (UTC {ts.strftime('%Y-%m-%d %H:%M:%S')}) ===")
        _set_folder_and_contents_times(full, ts, verbose=args.verbose)

if __name__ == "__main__":
    if os.name != "nt":
        print("This script is intended for Windows (uses SetFileTime).", file=sys.stderr)
        sys.exit(1)
    main()