"""Select an installed stable Xcode SDK that can compile the native Translation bridge."""
import os
import re
from pathlib import Path

candidates = []
for path in Path("/Applications").glob("Xcode_*.app"):
    match = re.fullmatch(r"Xcode_(\d+(?:\.\d+){1,2})\.app", path.name)
    if match:
        version = tuple(map(int, match[1].split(".")))
        if version[0] >= 26:
            candidates.append((version, path))
if not candidates:
    raise SystemExit("Xcode 26+ is required for the Apple Translation bridge.")
_, selected = max(candidates)
developer_dir = selected / "Contents/Developer"
with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as output:
    output.write(f"DEVELOPER_DIR={developer_dir}\n")
print(f"Selected {selected.name}")
