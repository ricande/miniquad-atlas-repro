import argparse
import json
import pathlib
import platform
import shutil
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent
REVISION = "39c928fb7e09fd44e45f37fe9a98503968bca412"
TESTS = ["depth-670", "offset-597", "regressions", "macroquad-integration"]


def command(arguments, logfile, **kwargs):
    with logfile.open("w") as output:
        result = subprocess.run(arguments, stdout=output, stderr=subprocess.STDOUT, **kwargs)
    if result.returncode:
        raise RuntimeError(f"Command failed ({result.returncode}); see {logfile}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=pathlib.Path)
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--wasm", action="store_true")
    parser.add_argument("--runs", type=int, default=3)
    options = parser.parse_args()
    if options.runs < 1:
        parser.error("--runs must be positive")
    work_root = ROOT / ".work"
    work_root.mkdir(exist_ok=True)
    work = pathlib.Path(tempfile.mkdtemp(prefix="run-", dir=work_root))
    print(f"Artifacts: {work}", flush=True)
    source = options.source.resolve() if options.source else work / "upstream"
    if options.source is None:
        if options.offline:
            parser.error("--offline requires --source pointing to the pinned Git checkout")
        command(["git", "clone", "https://github.com/not-fl3/miniquad.git", str(source)], work / "clone.log", timeout=300)
        command(["git", "checkout", "--detach", REVISION], work / "checkout.log", cwd=source, timeout=30)
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=source, text=True).strip()
    dirty = subprocess.check_output(["git", "status", "--porcelain"], cwd=source, text=True).strip()
    if revision != REVISION or dirty:
        raise RuntimeError("Source must be a clean checkout of the pinned revision")
    environment = {
        "revision": revision,
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "macos": platform.mac_ver()[0],
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "cargo": subprocess.check_output(["cargo", "--version"], text=True).strip(),
    }
    (work / "environment.json").write_text(json.dumps(environment, indent=2) + "\n")
    results = []
    for variant in ("baseline", "fixed"):
        directory = work / variant
        directory.mkdir()
        library = directory / "miniquad"
        shutil.copytree(source, library, ignore=shutil.ignore_patterns(".git", "target"))
        if variant == "fixed":
            command(["patch", "--batch", "--fuzz=0", "-p1", "-i", str(ROOT / "combined-depth.patch")], work / "patch.log", cwd=library, timeout=30)
        crate = directory / "harness"
        crate.mkdir()
        shutil.copytree(ROOT / "src", crate / "src")
        manifest = '''[package]
name = "miniquad-depth-verification"
version = "0.1.0"
edition = "2021"
publish = false

[dependencies]
miniquad = { path = "../miniquad" }
macroquad = "=0.4.16"

[patch.crates-io]
miniquad = { path = "../miniquad" }

[features]
webgl2 = []
'''
        for name in TESTS + ["web-depth"]:
            manifest += f'\n[[bin]]\nname = "{name}"\npath = "src/{name}.rs"\n'
        (crate / "Cargo.toml").write_text(manifest)
        lock = ROOT / "Cargo.lock"
        if lock.exists():
            shutil.copyfile(lock, crate / "Cargo.lock")
        build = ["cargo", "build", "--release", "--manifest-path", str(crate / "Cargo.toml")]
        if lock.exists():
            build.append("--locked")
        if options.offline:
            build.append("--offline")
        command(build, work / f"build-{variant}.log", timeout=300)
        for name in TESTS:
            expected = 1 if variant == "baseline" and name != "macroquad-integration" else 0
            for run in range(1, options.runs + 1):
                logfile = work / f"{variant}-{name}-{run}.log"
                with logfile.open("w") as output:
                    try:
                        status = subprocess.run([str(crate / "target" / "release" / name)], stdout=output, stderr=subprocess.STDOUT, timeout=45).returncode
                    except subprocess.TimeoutExpired:
                        status = "timeout"
                content = logfile.read_text()
                valid = status == expected and "RESULT:" in content
                if variant == "baseline" and name == "depth-670":
                    valid = valid and "issue 670 reproduced" in content
                if variant == "baseline" and name == "offset-597":
                    valid = valid and "issue 597 reproduced" in content
                if variant == "fixed" or name == "macroquad-integration":
                    valid = valid and "FAIL " not in content
                results.append({"variant": variant, "test": name, "run": run, "exit": status, "expected": expected, "valid": valid, "summary": [line for line in content.splitlines() if line.startswith("RESULT:")]})
                (work / "results.json").write_text(json.dumps(results, indent=2) + "\n")
                print(f"{variant} {name} #{run}: {'PASS' if valid else 'UNEXPECTED'}", flush=True)
        if options.wasm:
            web = work / "web"
            web.mkdir(exist_ok=True)
            for version in (1, 2):
                target = crate / "target" / f"webgl{version}"
                wasm_build = build + ["--bin", "web-depth", "--target", "wasm32-unknown-unknown", "--target-dir", str(target)]
                if version == 2:
                    wasm_build += ["--features", "webgl2"]
                command(wasm_build, work / f"wasm-{variant}-{version}.log", timeout=300)
                shutil.copyfile(target / "wasm32-unknown-unknown/release/web-depth.wasm", web / f"{variant}-{version}.wasm")
            if variant == "fixed":
                shutil.copyfile(library / "js/gl.js", web / "patched-gl.js")
            for filename in ("index.html", "case.html"):
                shutil.copyfile(ROOT / "web" / filename, web / filename)
    if not all(result["valid"] for result in results):
        raise SystemExit("Unexpected test results; inspect results.json and logs")
    print("All native outcomes match expectations. Web builds, if requested, still require browser execution.")


if __name__ == "__main__":
    main()
