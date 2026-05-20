from pathlib import Path
from datetime import datetime
import filecmp
import fnmatch
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile

import py7zr
import requests

from config import (
    FIRS,
    REPO_LAYOUT_MAP,
    GNG_ONLY_FILES,
    GITHUB_OWNER,
    GITHUB_REPO,
    GITHUB_BRANCH,
    VERSION_FILE,
    GITHUB_RELEASE_OWNER,
    GITHUB_RELEASE_REPO,
)

try:
    from build_info import BUILD_TAG
except Exception:
    BUILD_TAG = "dev"


COPYRIGHT_FILE = "aeronav_copyright.txt"


def matches(path: Path, patterns: list[str]) -> bool:
    normalized = path.as_posix()

    for pattern in patterns:
        if fnmatch.fnmatch(normalized, pattern):
            return True

        if pattern.endswith("/*"):
            folder_pattern = pattern[:-2]
            if normalized == folder_pattern:
                return True

    return False


def get_github_version() -> str:
    url = f"https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/commits/{GITHUB_BRANCH}"
    response = requests.get(url, timeout=30)
    response.raise_for_status()
    return response.json()["sha"][:7]


def get_local_version(install_root: Path) -> str:
    version_file = install_root / VERSION_FILE

    if version_file.exists():
        return version_file.read_text(encoding="utf-8", errors="ignore").strip()

    return "Not installed"


def write_installed_version(install_root: Path, version: str):
    version_file = install_root / VERSION_FILE
    version_file.parent.mkdir(parents=True, exist_ok=True)
    version_file.write_text(version, encoding="utf-8")


def download_github_repo() -> Path:
    tmp = Path(tempfile.mkdtemp(prefix="cofrance_repo_"))
    zip_path = tmp / "repo.zip"

    url = f"https://api.github.com/repos/{GITHUB_OWNER}/{GITHUB_REPO}/zipball/{GITHUB_BRANCH}"
    response = requests.get(url, allow_redirects=True, timeout=60)
    response.raise_for_status()
    zip_path.write_bytes(response.content)

    extract_dir = tmp / "repo"
    extract_dir.mkdir(parents=True, exist_ok=True)

    with zipfile.ZipFile(zip_path, "r") as archive:
        archive.extractall(extract_dir)

    return next(path for path in extract_dir.iterdir() if path.is_dir())


def get_latest_release_info() -> dict | None:
    url = f"https://api.github.com/repos/{GITHUB_RELEASE_OWNER}/{GITHUB_RELEASE_REPO}/releases/latest"

    response = requests.get(url, timeout=30)

    if response.status_code == 404:
        return None

    response.raise_for_status()
    return response.json()


def tool_update_available() -> tuple[bool, str]:
    release = get_latest_release_info()

    if not release:
        return False, "No release found"

    latest_tag = release.get("tag_name", "")

    if not latest_tag:
        return False, "No release tag found"

    if BUILD_TAG in ["dev", "local", ""]:
        return True, latest_tag

    return latest_tag != BUILD_TAG, latest_tag


def download_release_asset(asset_name: str, destination: Path) -> bool:
    release = get_latest_release_info()

    if not release:
        return False

    for asset in release.get("assets", []):
        if asset.get("name") == asset_name:
            url = asset.get("browser_download_url")
            response = requests.get(url, timeout=120)
            response.raise_for_status()
            destination.write_bytes(response.content)
            return True

    return False


def self_update_tools(current_dir: Path):
    temp_dir = current_dir / "_tool_update"
    temp_dir.mkdir(parents=True, exist_ok=True)

    installer_new = temp_dir / "ControllerPackInstaller.exe"
    profile_new = temp_dir / "ProfileConfigurator.exe"

    downloaded_installer = download_release_asset("ControllerPackInstaller.exe", installer_new)
    downloaded_profile = download_release_asset("ProfileConfigurator.exe", profile_new)

    if not downloaded_installer and not downloaded_profile:
        raise RuntimeError(
            "No ControllerPackInstaller.exe or ProfileConfigurator.exe assets found in the latest release."
        )

    bat_path = temp_dir / "apply_update.bat"

    current_installer = current_dir / "ControllerPackInstaller.exe"
    current_profile = current_dir / "ProfileConfigurator.exe"

    commands = [
        "@echo off",
        "timeout /t 2 /nobreak >nul",
    ]

    if downloaded_installer:
        commands.append(f'copy /Y "{installer_new}" "{current_installer}"')

    if downloaded_profile:
        commands.append(f'copy /Y "{profile_new}" "{current_profile}"')

    commands.extend(
        [
            f'start "" "{current_installer}"',
            f'rmdir /S /Q "{temp_dir}"',
            "exit",
        ]
    )

    bat_path.write_text("\n".join(commands), encoding="utf-8")

    subprocess.Popen(
        ["cmd", "/c", str(bat_path)],
        creationflags=subprocess.CREATE_NEW_CONSOLE,
    )

    sys.exit(0)


def extract_archive(archive: Path, destination: Path) -> Path:
    output = destination / archive.stem
    output.mkdir(parents=True, exist_ok=True)

    if archive.suffix.lower() == ".zip":
        with zipfile.ZipFile(archive, "r") as z:
            z.extractall(output)

    elif archive.suffix.lower() == ".7z":
        with py7zr.SevenZipFile(archive, "r") as z:
            z.extractall(output)

    else:
        raise ValueError(f"Unsupported archive type: {archive}")

    return output


def sync_tree(
    src: Path,
    dst: Path,
    dst_prefix: Path,
    exclude: list[str] | None = None,
):
    """
    Safe overlay sync.

    Copies official GitHub-managed files into the installed controller pack
    WITHOUT deleting unrelated local files/folders.

    Preserves:
    - custom plugins
    - custom ASRs
    - custom settings
    - user-added folders/files

    Official files with matching paths are updated/replaced.
    """

    dst.mkdir(parents=True, exist_ok=True)

    for file in src.rglob("*"):
        if not file.is_file():
            continue

        # Skip duplicate Aeronav copyright files.
        if file.name.lower() == COPYRIGHT_FILE:
            continue

        rel = file.relative_to(src)
        full_rel = dst_prefix / rel

        # Skip protected/GNG-managed paths.
        if exclude and matches(full_rel, exclude):
            continue

        target = dst / rel
        target.parent.mkdir(parents=True, exist_ok=True)

        # Overlay/replace official file only.
        shutil.copy2(file, target)


def apply_repo_layout(repo_root: Path, install_root: Path):
    for src_rel, dst_rel in REPO_LAYOUT_MAP.items():
        src = repo_root / src_rel
        dst = install_root / dst_rel

        if src.exists():
            sync_tree(
                src=src,
                dst=dst,
                dst_prefix=Path(dst_rel),
                exclude=GNG_ONLY_FILES,
            )


def restore_cofrance_loader(repo_root: Path, install_root: Path):
    source = repo_root / "LFXX" / "Plugins" / "CoFrance" / "CoFranceLoader.dll"

    if not source.exists():
        return

    target = install_root / "LFXX" / "Plugins" / "CoFrance" / "CoFranceLoader.dll"
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)


def detect_sector_code(filename: str) -> str | None:
    upper = filename.upper()

    for code in FIRS + ["LFFM", "LFXX"]:
        if upper.startswith(code):
            return "LFXX" if code == "LFFM" else code

    return None


def should_skip_sector_file(file: Path) -> bool:
    upper = file.name.upper()
    return upper.startswith("LFFM") and file.suffix.lower() == ".sct"


def extract_airac_cycle_from_name(name: str) -> str | None:
    match = re.search(r"-(\d{6})-", name)

    if match:
        return match.group(1)

    return None


def get_existing_airac_cycle(install_root: Path) -> str:
    marker = install_root / "LFXX" / "Sectors" / "current_airac.txt"

    if marker.exists():
        value = marker.read_text(encoding="utf-8", errors="ignore").strip()

        if value:
            return value

    sector_dir = install_root / "LFXX" / "Sectors"

    if sector_dir.exists():
        for file in sector_dir.iterdir():
            if not file.is_file():
                continue

            cycle = extract_airac_cycle_from_name(file.name)

            if cycle:
                return cycle

    return "unknown"


def write_current_airac_cycle(install_root: Path, cycle: str):
    marker = install_root / "LFXX" / "Sectors" / "current_airac.txt"
    marker.parent.mkdir(parents=True, exist_ok=True)
    marker.write_text(cycle, encoding="utf-8")


def backup_existing_sector_files_for_previous_airac(install_root: Path):
    sector_dir = install_root / "LFXX" / "Sectors"
    sector_dir.mkdir(parents=True, exist_ok=True)

    existing_files = [
        file
        for file in sector_dir.iterdir()
        if file.is_file() and file.suffix.lower() in [".sct", ".ese"]
    ]

    if not existing_files:
        return None

    previous_cycle = get_existing_airac_cycle(install_root)
    backup_dir = sector_dir / f"AIRAC_{previous_cycle}"

    if backup_dir.exists():
        backup_dir = sector_dir / f"AIRAC_{previous_cycle}_{datetime.now().strftime('%Y%m%d_%H%M%S')}"

    backup_dir.mkdir(parents=True, exist_ok=True)

    for file in existing_files:
        shutil.copy2(file, backup_dir / file.name)

    return backup_dir


def gng_contains_sector_update(extracted_gng_root: Path) -> tuple[bool, str | None]:
    new_cycle = None

    for file in extracted_gng_root.rglob("*"):
        if not file.is_file():
            continue

        if file.suffix.lower() not in [".sct", ".ese"]:
            continue

        if should_skip_sector_file(file):
            continue

        if not detect_sector_code(file.name):
            continue

        cycle = extract_airac_cycle_from_name(file.name)

        if cycle:
            new_cycle = cycle

        return True, new_cycle

    return False, None


def settings_backup_already_exists(install_root: Path) -> bool:
    backup_dir = install_root / "LFXX" / "Settings" / "settings_backup"

    return backup_dir.exists() and any(backup_dir.iterdir())


def backup_lfxx_settings_to_settings_backup(install_root: Path):
    settings_dir = install_root / "LFXX" / "Settings"
    settings_dir.mkdir(parents=True, exist_ok=True)

    backup_root = settings_dir / "settings_backup"
    backup_root.mkdir(parents=True, exist_ok=True)

    backup_dir = backup_root / datetime.now().strftime("%Y%m%d_%H%M%S")
    backup_dir.mkdir(parents=True, exist_ok=True)

    for item in settings_dir.rglob("*"):
        if not item.is_file():
            continue

        if backup_root in item.parents:
            continue

        rel = item.relative_to(settings_dir)
        target = backup_dir / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(item, target)

    return backup_dir


def lfxx_settings_would_change(repo_root: Path, install_root: Path) -> bool:
    repo_settings = repo_root / "LFXX" / "Settings"
    installed_settings = install_root / "LFXX" / "Settings"

    if not repo_settings.exists():
        return False

    if not installed_settings.exists():
        return True

    repo_files = {
        file.relative_to(repo_settings)
        for file in repo_settings.rglob("*")
        if file.is_file()
    }

    installed_files = {
        file.relative_to(installed_settings)
        for file in installed_settings.rglob("*")
        if file.is_file()
    }

    if repo_files != installed_files:
        return True

    for rel in repo_files:
        if not filecmp.cmp(repo_settings / rel, installed_settings / rel, shallow=False):
            return True

    return False


def copy_single_copyright_file(source_root: Path, install_root: Path):
    target = install_root / COPYRIGHT_FILE

    if target.exists():
        return

    for file in source_root.rglob(COPYRIGHT_FILE):
        if file.is_file():
            shutil.copy2(file, target)
            return


def remove_duplicate_copyright_files(install_root: Path):
    root_copy = install_root / COPYRIGHT_FILE

    for file in install_root.rglob(COPYRIGHT_FILE):
        if file.resolve() == root_copy.resolve():
            continue

        file.unlink()


def apply_gng_packages(packages: list[Path], install_root: Path):
    sector_dir = install_root / "LFXX" / "Sectors"
    sector_dir.mkdir(parents=True, exist_ok=True)

    new_airac_cycle = None

    with tempfile.TemporaryDirectory(prefix="cofrance_gng_") as tmp_name:
        tmp = Path(tmp_name)

        for package in packages:
            extract_archive(package, tmp)

        has_sector_update, detected_cycle = gng_contains_sector_update(tmp)

        if has_sector_update:
            backup_existing_sector_files_for_previous_airac(install_root)

        if detected_cycle:
            new_airac_cycle = detected_cycle

        copy_single_copyright_file(tmp, install_root)

        for file in tmp.rglob("*"):
            if not file.is_file():
                continue

            if file.name.lower() == COPYRIGHT_FILE:
                continue

            suffix = file.suffix.lower()

            if suffix in [".sct", ".ese", ".rwy"]:
                if should_skip_sector_file(file):
                    continue

                code = detect_sector_code(file.name)

                if code:
                    cycle = extract_airac_cycle_from_name(file.name)

                    if cycle:
                        new_airac_cycle = cycle

                    target = sector_dir / f"{code}{suffix}"

                    if target.exists():
                        target.unlink()

                    shutil.copy2(file, target)

                continue

            parts = file.parts

            for fir in FIRS + ["LFXX", "LFFM"]:
                if fir in parts:
                    index = parts.index(fir)
                    rel = Path(*parts[index:])

                    if rel.parts[0] == "LFFM":
                        rel = Path("LFXX", *rel.parts[1:])

                    if matches(rel, GNG_ONLY_FILES):
                        target = install_root / rel
                        target.parent.mkdir(parents=True, exist_ok=True)
                        shutil.copy2(file, target)

                    break

        if new_airac_cycle:
            write_current_airac_cycle(install_root, new_airac_cycle)

    remove_duplicate_copyright_files(install_root)


def normalize_sectors(install_root: Path):
    sector_dir = install_root / "LFXX" / "Sectors"
    sector_dir.mkdir(parents=True, exist_ok=True)

    for file in list(sector_dir.iterdir()):
        if not file.is_file():
            continue

        if file.suffix.lower() not in [".sct", ".ese", ".rwy"]:
            continue

        if should_skip_sector_file(file):
            file.unlink()
            continue

        code = detect_sector_code(file.name)

        if not code:
            continue

        target = sector_dir / f"{code}{file.suffix.lower()}"

        if file.resolve() != target.resolve():
            if target.exists():
                target.unlink()

            file.rename(target)


def cleanup_legacy_root_files(install_root: Path):
    allowed_root_files = {
        "aeronav_copyright.txt",
        "ProfileConfigurator.exe",
        "ControllerPackInstaller.exe",
    }

    for item in install_root.iterdir():
        if item.is_dir():
            if item.name.upper() == "EXE":
                shutil.rmtree(item, ignore_errors=True)

            continue

        if item.name in allowed_root_files:
            continue

        if item.suffix.lower() in [".sct", ".ese", ".rwy", ".prf"]:
            item.unlink()


def cleanup_install(install_root: Path):
    remove_duplicate_copyright_files(install_root)
    cleanup_legacy_root_files(install_root)

    sector_dir = install_root / "LFXX" / "Sectors"

    for bad_name in ["LFFM.sct", "LFXX.sct"]:
        bad_file = sector_dir / bad_name

        if bad_file.exists():
            bad_file.unlink()


def update_controller_pack(
    install_root: Path,
    gng_packages: list[Path] | None = None,
    backup_settings_callback=None,
):
    github_version = get_github_version()
    repo_root = download_github_repo()

    if lfxx_settings_would_change(repo_root, install_root):
        if not settings_backup_already_exists(install_root):
            if backup_settings_callback and backup_settings_callback():
                backup_lfxx_settings_to_settings_backup(install_root)

    apply_repo_layout(repo_root, install_root)

    restore_cofrance_loader(repo_root, install_root)

    if gng_packages:
        apply_gng_packages(gng_packages, install_root)

    normalize_sectors(install_root)
    cleanup_install(install_root)

    write_installed_version(install_root, github_version)