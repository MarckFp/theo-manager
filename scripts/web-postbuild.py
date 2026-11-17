#!/usr/bin/env python3
"""Finishes the web build for static hosting (GitHub Pages).

Usage: python3 scripts/web-postbuild.py <public-dir>

1. Copies index.html to 404.html, so deep links (/app/users/all) load the app on GitHub Pages.
2. Writes a source map for each JavaScript file that has none. The JavaScript is generated
   glue code (wasm-bindgen) with no other source, so the map points each line at itself and
   embeds the file. Browsers and Lighthouse then have a source map for every first-party script.
"""

import json
import shutil
import sys
from pathlib import Path


def identity_map(name: str, text: str) -> str:
    lines = text.count("\n") + 1
    # First line: column 0 of source 0, line 0. Each next line: one line further ("AACA").
    mappings = ";".join(["AAAA"] + ["AACA"] * (lines - 1))
    return json.dumps(
        {"version": 3, "file": name, "sources": [name], "sourcesContent": [text], "names": [], "mappings": mappings}
    )


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    public = Path(sys.argv[1])
    index = public / "index.html"
    if not index.is_file():
        print(f"no index.html in {public}")
        return 1
    shutil.copyfile(index, public / "404.html")

    for js in sorted(public.rglob("*.js")):
        text = js.read_text(encoding="utf-8")
        if "sourceMappingURL=" in text:
            continue
        map_name = js.name + ".map"
        (js.parent / map_name).write_text(identity_map(js.name, text), encoding="utf-8")
        js.write_text(text.rstrip("\n") + f"\n//# sourceMappingURL={map_name}\n", encoding="utf-8")
        print(f"source map: {js.relative_to(public)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
