"""Tor Expert Bundle arm64 + x86_64 → одна папка с universal-файлами.
python3 tor_bundle.py <распакованный arm64> <распакованный x86_64> <куда>"""
import os
import shutil
import subprocess
import sys

arm, x86, out = sys.argv[1:4]
here = os.path.dirname(os.path.abspath(__file__))
for sub in ("tor", "data"):
    for root, _, files in os.walk(os.path.join(arm, sub)):
        for name in files:
            if "conjure" in name:  # мосты conjure Z.UST не использует
                continue
            src = os.path.join(root, name)
            rel = os.path.relpath(src, arm)
            dst = os.path.join(out, rel)
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            with open(src, "rb") as f:
                macho = f.read(4) == b"\xcf\xfa\xed\xfe"
            if macho:
                subprocess.run([sys.executable, os.path.join(here, "fat.py"), dst, src,
                                os.path.join(x86, rel)], check=True)
                os.chmod(dst, 0o755)
            else:
                shutil.copy2(src, dst)
