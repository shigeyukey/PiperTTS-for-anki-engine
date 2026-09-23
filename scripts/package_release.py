# Copyright (C) Shigeyuki <http://patreon.com/Shigeyuki>
# License: GNU AGPL version 3 or later <http://www.gnu.org/licenses/agpl.html>

import argparse
import hashlib
import json
import os
import sys
import zipfile
from datetime import datetime, timezone
from os.path import join

sys.stdout.reconfigure(line_buffering=True)

MANIFEST_SCHEMA = 1
TAG_PREFIX = "engines-v"
ZIP_NAME_PREFIX = "piper-engines-"
READ_BLOCK_SIZE = 1024 * 1024


def parse_arguments():
    parser = argparse.ArgumentParser(description="package the piper engine release")
    parser.add_argument("--artifacts", default="artifacts", help="folder with the build results")
    parser.add_argument("--dist", default="dist", help="folder for the release assets")
    parser.add_argument("--tag", required=True, help="release tag, e.g. engines-v1.0.0")
    parser.add_argument("--repository", required=True, help="owner/name of this repository")
    return parser.parse_args()


def engine_version_from_tag(tag: str) -> str:
    if tag.startswith(TAG_PREFIX):
        return tag[len(TAG_PREFIX):]
    return tag


def format_megabytes(size_bytes: int) -> str:
    size_megabytes = size_bytes / (1024 * 1024)
    return f"{size_megabytes:,.1f} MB"


def sha256_of_file(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as file:
        while True:
            block = file.read(READ_BLOCK_SIZE)
            if not block:
                break
            digest.update(block)
    return digest.hexdigest()


def collect_files(artifacts_directory: str):
    entries = []
    for folder_root, sub_directories, file_names in os.walk(artifacts_directory):
        sub_directories.sort()
        for file_name in sorted(file_names):
            absolute_path = join(folder_root, file_name)
            relative_path = os.path.relpath(absolute_path, artifacts_directory)
            relative_path = relative_path.replace(os.sep, "/")
            entries.append((relative_path, absolute_path))
    return entries


def create_zip(dist_directory: str, zip_name: str, entries) -> str:
    zip_path = join(dist_directory, zip_name)
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED) as archive:
        for relative_path, absolute_path in entries:
            archive.write(absolute_path, relative_path)
    return zip_path


def read_build_info(artifacts_directory: str, relative_path: str):
    path = join(artifacts_directory, relative_path.replace("/", os.sep))
    if not os.path.isfile(path):
        print(f"  WARNING: build info not found: {relative_path}")
        return {}
    with open(path, "r", encoding="utf-8-sig") as file:
        return json.load(file)


def write_release_notes(dist_directory: str, engine_version: str, file_entries, builds):
    lines = []
    lines.append(f"# piper engines {engine_version}")
    lines.append("")
    lines.append("| file | size | sha256 |")
    lines.append("| --- | --- | --- |")
    for entry in file_entries:
        lines.append(f"| `{entry['path']}` | {format_megabytes(entry['size'])} | `{entry['sha256'][:16]}…` |")

    piper_info = builds.get("piper_wasm") or {}
    japanese_info = builds.get("japanese") or {}
    piper_plus_info = builds.get("piper_plus") or {}
    if piper_info or japanese_info or piper_plus_info:
        lines.append("")
        lines.append("## Build environment")
        lines.append("")
        if piper_info:
            lines.append(f"- emscripten: {piper_info.get('emscripten', '')}")
            lines.append(f"- libpiper (piper1-gpl): {piper_info.get('libpiper', '')}")
            lines.append(f"- espeak-ng: {piper_info.get('espeak_ng_commit', '')}")
            lines.append(f"- onnxruntime-web: {piper_info.get('ort_web', '')}")
            lines.append(f"- piper-tts (espeak-ng-data): {piper_info.get('piper_tts', '')}")
        if japanese_info:
            lines.append(f"- rustc (jpreprocess): {japanese_info.get('rustc', '')}")
        if piper_plus_info:
            lines.append(f"- rustc (jpreprocess 0.9.1, piper-plus): {piper_plus_info.get('rustc', '')}")
    lines.append("")

    notes_path = join(dist_directory, "release_notes.md")
    with open(notes_path, "w", encoding="utf-8") as file:
        file.write("\n".join(lines))
    return notes_path


def main() -> int:
    arguments = parse_arguments()
    artifacts_directory = os.path.abspath(arguments.artifacts)
    dist_directory = os.path.abspath(arguments.dist)

    if not os.path.isdir(artifacts_directory):
        print(f"ERROR: artifacts folder not found: {artifacts_directory}")
        return 1

    engine_version = engine_version_from_tag(arguments.tag)
    zip_name = f"{ZIP_NAME_PREFIX}{engine_version}.zip"

    entries = collect_files(artifacts_directory)
    if not entries:
        print(f"ERROR: no files to package in {artifacts_directory}")
        return 1

    print(f"== package {arguments.tag} (engine version {engine_version})")
    os.makedirs(dist_directory, exist_ok=True)

    zip_path = create_zip(dist_directory, zip_name, entries)
    zip_size = os.path.getsize(zip_path)
    print(f"  {zip_name}: {format_megabytes(zip_size)}")

    file_entries = []
    for relative_path, absolute_path in entries:
        file_info = {
            "path": relative_path,
            "size": os.path.getsize(absolute_path),
            "sha256": sha256_of_file(absolute_path),
        }
        file_entries.append(file_info)
        print(f"  {relative_path} ({format_megabytes(file_info['size'])})")

    builds = {
        "piper_wasm": read_build_info(artifacts_directory, "built/build_info.json"),
        "japanese": read_build_info(artifacts_directory, "built/ja/build_info.json"),
        "piper_plus": read_build_info(artifacts_directory, "built/piper_plus/build_info.json"),
    }

    manifest = {
        "schema": MANIFEST_SCHEMA,
        "engine_version": engine_version,
        "release_tag": arguments.tag,
        "repository": arguments.repository,
        "created_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "zip": {
            "name": zip_name,
            "size": zip_size,
            "sha256": sha256_of_file(zip_path),
        },
        "files": file_entries,
        "builds": builds,
    }

    manifest_path = join(dist_directory, "manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as file:
        json.dump(manifest, file, indent=2, ensure_ascii=False)
        file.write("\n")
    print(f"  manifest.json ({len(file_entries)} files)")

    notes_path = write_release_notes(dist_directory, engine_version, file_entries, builds)
    print(f"  {os.path.basename(notes_path)}")

    print("== done")
    return 0


if __name__ == "__main__":
    sys.exit(main())
