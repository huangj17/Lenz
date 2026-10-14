"""Generate offline notices from the resolved Rust and pnpm dependency metadata."""
import json
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--offline", "--manifest-path", str(root / "src-tauri/Cargo.toml"), "--format-version", "1"],
    cwd=root, text=True,
))
entries = {}
def add(name, version, license_name, location, repository):
    key = (name, version)
    if key in entries:
        return
    texts = []
    for path in sorted(location.iterdir()):
        if path.is_file() and path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE")):
            texts.append(path.read_text(errors="replace"))
    entries[key] = f"{name} {version}\nLicense: {license_name or 'See upstream license'}\n{repository or ''}\n\n" + "\n\n".join(texts)

for package in metadata["packages"]:
    if package.get("source"):
        add(package["name"], package["version"], package.get("license"),
            Path(package["manifest_path"]).parent, package.get("repository"))
for folder in (root / "node_modules/.pnpm").iterdir():
    modules = folder / "node_modules"
    if not modules.is_dir():
        continue
    for manifest in modules.glob("*/package.json"):
        package = json.loads(manifest.read_text())
        repository = package.get("repository", "")
        if isinstance(repository, dict):
            repository = repository.get("url", "")
        add(package["name"], package["version"], package.get("license"), manifest.parent, repository)
    for manifest in modules.glob("@*/*/package.json"):
        package = json.loads(manifest.read_text())
        repository = package.get("repository", "")
        if isinstance(repository, dict):
            repository = repository.get("url", "")
        add(package["name"], package["version"], package.get("license"), manifest.parent, repository)

output = root / "src-tauri/resources/LICENSES.txt"
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text((root / "LICENSE").read_text() + "\n\nTHIRD-PARTY DEPENDENCY NOTICES\n\n" +
    "\n\n".join("========================================\n" + entries[key] for key in sorted(entries)))
print(f"Generated notices for {len(entries)} resolved dependencies.")
