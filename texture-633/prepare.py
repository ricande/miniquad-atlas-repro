import argparse
import pathlib
import shutil
import subprocess

revision = "39c928fb7e09fd44e45f37fe9a98503968bca412"
root = pathlib.Path(__file__).resolve().parent
parser = argparse.ArgumentParser()
parser.add_argument("--source", type=pathlib.Path)
arguments = parser.parse_args()
baseline = root / "baseline-lib"
fixed = root / "fixed-lib"
if baseline.exists() or fixed.exists():
    raise SystemExit("Source copies already exist; use them or remove them explicitly first.")
source = arguments.source
if source is None:
    source = root / ".work" / "upstream"
    source.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init", str(source)], check=True)
    subprocess.run(["git", "-C", str(source), "fetch", "--depth=1",
                    "https://github.com/not-fl3/miniquad.git", revision], check=True)
archive = subprocess.run(["git", "-C", str(source), "archive", revision],
                         check=True, stdout=subprocess.PIPE).stdout
baseline.mkdir()
subprocess.run(["tar", "-xf", "-", "-C", str(baseline)], input=archive, check=True)
shutil.copytree(baseline, fixed)
subprocess.run(["patch", "--batch", "-p1", "-i", str(root / "pr691.patch")],
               cwd=fixed, check=True)
print("Prepared pinned baseline and PR #691 variant.")
