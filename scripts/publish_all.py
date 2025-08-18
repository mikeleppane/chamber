import os
import re
import subprocess
import sys
import time
from pathlib import Path

# Define publishing order based on dependencies
PUBLISHING_ORDER = [
    # Group 1: No internal dependencies
    ["chamber-password-gen", "chamber-vault"],

    # Group 2: Depend on Group 1
    ["chamber-import-export", "chamber-backup", "chamber-api"],

    # Group 3: Depend on Groups 1 & 2
    ["chamber-ui", "chamber-cli"],

    # Group 4: The main binary (depends on everything)
    ["chamber"]
]

VERSION = "0.5.2"  # Your current version

def backup_cargo_toml(crate_path: Path):
    """Create backup of original Cargo.toml"""
    cargo_toml = crate_path / "Cargo.toml"
    backup_file = crate_path / "Cargo.toml.backup"
    if cargo_toml.exists():
        # Read and backup
        content = cargo_toml.read_text()
        backup_file.write_text(content)
        return True
    return False

def update_dependencies_for_publishing(crate_path: Path, published_crates: set):
    """Replace workspace deps with published versions"""
    cargo_toml = crate_path / "Cargo.toml"
    if not cargo_toml.exists():
        return

    content = cargo_toml.read_text()

    # Replace internal workspace dependencies
    for crate_name in published_crates:
        old_pattern = f'{crate_name} = {{workspace = true}}'
        new_pattern = f'{crate_name} = "{VERSION}"'
        content = content.replace(old_pattern, new_pattern)

        # Also handle with spaces
        old_pattern_spaces = f'{crate_name} = {{ workspace = true }}'
        content = content.replace(old_pattern_spaces, new_pattern)

    cargo_toml.write_text(content)
    print(f"  ✅ Updated {crate_path.name}/Cargo.toml")

def restore_cargo_toml(crate_path: Path):
    """Restore original Cargo.toml from backup"""
    cargo_toml = crate_path / "Cargo.toml"
    backup_file = crate_path / "Cargo.toml.backup"

    if backup_file.exists():
        content = backup_file.read_text()
        cargo_toml.write_text(content)
        backup_file.unlink()  # Remove backup
        print(f"  🔄 Restored {crate_path.name}/Cargo.toml")

def get_crate_path(crate_name: str, project_root: Path) -> Path:
    """Get the filesystem path for a crate"""
    if crate_name.startswith("chamber-"):
        folder_name = crate_name[8:]  # Remove the "chamber-" prefix
    else:
        folder_name = crate_name

    return project_root / "crates" / folder_name

# ... existing code ...
def publish_crate(crate_path: Path, dry_run: bool = False) -> bool:
    """Publish a single crate"""
    print(f"🚀 Publishing {crate_path.name}...")

    # First test the build
    result = subprocess.run(["cargo", "check"], cwd=crate_path, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"  ❌ Build failed for {crate_path.name}: {result.stderr}")
        return False

    # Package (no --dry-run flag exists for cargo package)
    result = subprocess.run(["cargo", "package"], cwd=crate_path, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"  ❌ Package failed for {crate_path.name}: {result.stderr}")
        return False

    if dry_run:
        # Validate full publish flow without uploading
        result = subprocess.run(["cargo", "publish", "--dry-run"], cwd=crate_path, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"  ❌ Publish dry-run failed for {crate_path.name}: {result.stderr}")
            return False
        print(f"  ✅ Dry run successful for {crate_path.name}")
    else:
        # Actually publish
        result = subprocess.run(["cargo", "publish"], cwd=crate_path, capture_output=True, text=True)
        if result.returncode != 0:
            print(f"  ❌ Publish failed for {crate_path.name}: {result.stderr}")
            return False
        print(f"  ✅ Successfully published {crate_path.name}")

    return True

def main():
    project_root = Path(__file__).parent.parent
    crates_dir = project_root / "crates"

    dry_run = "--dry-run" in sys.argv

    if dry_run:
        print("🧪 DRY RUN MODE - No actual publishing")
    else:
        print("🚀 PUBLISHING MODE - Will publish to crates.io")

    print(f"📦 Starting Chamber crates publishing process...")
    print(f"📋 Version: {VERSION}")

    published_crates = set()
    all_crate_paths = []

    try:
        for group_index, crate_group in enumerate(PUBLISHING_ORDER):
            print(f"\n📋 Group {group_index + 1}: {', '.join(crate_group)}")

            # Prepare crates in this group
            group_paths = []
            for crate_name in crate_group:
                crate_path = get_crate_path(crate_name, project_root)
                if crate_path.exists():
                    group_paths.append(crate_path)
                    all_crate_paths.append(crate_path)

                    # Backup and update dependencies
                    backup_cargo_toml(crate_path)
                    update_dependencies_for_publishing(crate_path, published_crates)
                else:
                    print(f"  ⚠️  Crate directory not found: {crate_path}")

            # Publish all crates in this group
            group_success = True
            for crate_path in group_paths:
                if not publish_crate(crate_path, dry_run):
                    group_success = False
                    break

            if not group_success:
                print(f"❌ Publishing failed for group {group_index + 1}")
                break

            # Add this group's crates to a published set
            published_crates.update(crate_group)

            # Wait for crates.io to index (except for the last group and dry runs)
            if not dry_run and group_index < len(PUBLISHING_ORDER) - 1:
                print("⏳ Waiting 60 seconds for crates.io indexing...")
                time.sleep(60)

    finally:
        # Always restore original Cargo.toml files
        print("\n🔄 Restoring original Cargo.toml files...")
        for crate_path in all_crate_paths:
            restore_cargo_toml(crate_path)

    if not dry_run:
        print("\n✅ Publishing process complete!")
        print("🎉 Users can now install with: cargo install chamber")
    else:
        print("\n✅ Dry run complete!")
        print("💡 Run without --dry-run to actually publish")

if __name__ == "__main__":
    main()