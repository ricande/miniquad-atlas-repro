import pathlib
import shutil
import subprocess

root = pathlib.Path(__file__).resolve().parent
web = root / "web"
web.mkdir(exist_ok=True)
for variant in ("baseline", "fixed"):
    for version in (1, 2):
        for case, feature in (("cache", ""), ("live", "live-control"), ("invalid", "invalid-handle")):
            features = [feature] if feature else []
            if version == 2:
                features.append("webgl2")
            command = ["cargo", "build", "--locked", "--release", "--target", "wasm32-unknown-unknown", "--manifest-path", str(root / variant / "Cargo.toml")]
            if features:
                command += ["--features", ",".join(features)]
            with (root / f"build-{variant}-{version}-{case}.log").open("w") as log:
                subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True, timeout=180)
            shutil.copyfile(root / variant / "target/wasm32-unknown-unknown/release/texture-633.wasm", web / f"{variant}-{version}-{case}.wasm")
            print(variant, version, case, "built", flush=True)
shutil.copyfile(root / "baseline-lib/js/gl.js", web / "gl.js")
