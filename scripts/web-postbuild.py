#!/usr/bin/env python3
"""Finishes the web build for static hosting (GitHub Pages).

Usage: python3 scripts/web-postbuild.py <public-dir> [--base /theo-manager/] [--version 1.2.3]
                                        [--icons theo-app/icons/gen]

1. Makes the web app installable (a PWA), when --icons is given: copies the icons, writes
   manifest.webmanifest and a service worker (sw.js) that keeps the app's files so it opens
   offline, and adds the manifest, the touch icon and the service worker to index.html.
2. Copies index.html to 404.html, so deep links (/app/users/all) load the app on GitHub Pages.
3. Writes a source map for each JavaScript file that has none. The JavaScript is generated
   glue code (wasm-bindgen) with no other source, so the map points each line at itself and
   embeds the file. Browsers and Lighthouse then have a source map for every first-party script.

--base is the path the site is served under (GitHub Pages: /<repository>/); it must start and end
with "/". --version names the service worker's cache, so a new release replaces the old files.
"""

import argparse
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


# The icons the PWA needs, from the files scripts/icons.sh generates (Tauri CLI layout).
ICONS = {
    "icon-192.png": "android/mipmap-xxxhdpi/ic_launcher.png",  # 192 px
    "icon-512.png": "icon.png",  # 512 px
    "icon-maskable.png": "ios/AppIcon-512@2x.png",  # 1024 px, no transparent corners
    "apple-touch-icon.png": "ios/AppIcon-60x60@3x.png",  # 180 px, no transparent corners
}

SERVICE_WORKER = """// Theo Manager service worker, written by scripts/web-postbuild.py. It keeps the app's files so
// the installed app opens without a connection. Each release has its own cache.
const CACHE = "theo-manager-%(version)s";
const BASE = "%(base)s";
const FILES = %(files)s;

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(FILES)).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("theo-manager-") && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET" || !request.url.startsWith(self.location.origin + BASE)) return;
  if (request.mode === "navigate") {
    // The app is one page: from the network when online (fresh after a release), else the kept one.
    event.respondWith(fetch(request).catch(() => caches.match(BASE + "index.html")));
    return;
  }
  // Everything else: the kept file first; files fetched later are kept too.
  event.respondWith(
    caches.match(request).then(
      (hit) =>
        hit ||
        fetch(request).then((response) => {
          if (response.ok) {
            const copy = response.clone();
            caches.open(CACHE).then((cache) => cache.put(request, copy));
          }
          return response;
        })
    )
  );
});
"""


def make_pwa(public: Path, base: str, version: str, icons: Path) -> None:
    out = public / "icons"
    out.mkdir(exist_ok=True)
    for name, source in ICONS.items():
        src = icons / source
        if not src.is_file():
            sys.exit(f"missing icon {src}: run scripts/icons.sh first")
        shutil.copyfile(src, out / name)

    manifest = {
        "id": base,
        "name": "Theo Manager",
        "short_name": "Theo Manager",
        "description": "Organise a congregation: private and end-to-end encrypted, works offline.",
        "lang": "en",
        "start_url": base,
        "scope": base,
        "display": "standalone",
        "background_color": "#0f0f12",
        "theme_color": "#3756d6",
        "icons": [
            {"src": base + "icons/icon-192.png", "sizes": "192x192", "type": "image/png", "purpose": "any"},
            {"src": base + "icons/icon-512.png", "sizes": "512x512", "type": "image/png", "purpose": "any"},
            {"src": base + "icons/icon-maskable.png", "sizes": "1024x1024", "type": "image/png", "purpose": "maskable"},
        ],
    }
    (public / "manifest.webmanifest").write_text(json.dumps(manifest, indent=2), encoding="utf-8")

    # Keep every file of the app (not source maps, not 404.html which is the same as index.html).
    files = [base, base + "index.html"]
    for f in sorted(public.rglob("*")):
        if f.is_file() and f.suffix != ".map" and f.name not in ("404.html", "sw.js", "index.html"):
            files.append(base + f.relative_to(public).as_posix())
    sw = SERVICE_WORKER % {"version": version, "base": base, "files": json.dumps(files, indent=2)}
    (public / "sw.js").write_text(sw, encoding="utf-8")

    head = (
        f'<link rel="manifest" href="{base}manifest.webmanifest">'
        f'<link rel="apple-touch-icon" href="{base}icons/apple-touch-icon.png">'
        '<meta name="mobile-web-app-capable" content="yes">'
        '<meta name="apple-mobile-web-app-capable" content="yes">'
        '<meta name="apple-mobile-web-app-title" content="Theo Manager">'
        '<script>if("serviceWorker" in navigator){addEventListener("load",function(){'
        f'navigator.serviceWorker.register("{base}sw.js",{{scope:"{base}"}})'
        "})}</script>"
    )
    index = public / "index.html"
    html = index.read_text(encoding="utf-8")
    if "manifest.webmanifest" not in html:
        html = html.replace("</head>", head + "</head>", 1)
        index.write_text(html, encoding="utf-8")
    print(f"pwa: manifest, service worker ({len(files)} files kept) and icons")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("public", type=Path)
    parser.add_argument("--base", default="/")
    parser.add_argument("--version", default="dev")
    parser.add_argument("--icons", type=Path)
    args = parser.parse_args()
    public = args.public
    base = args.base if args.base.endswith("/") else args.base + "/"
    if not base.startswith("/"):
        base = "/" + base
    index = public / "index.html"
    if not index.is_file():
        print(f"no index.html in {public}")
        return 1

    for js in sorted(public.rglob("*.js")):
        text = js.read_text(encoding="utf-8")
        if "sourceMappingURL=" in text:
            continue
        map_name = js.name + ".map"
        (js.parent / map_name).write_text(identity_map(js.name, text), encoding="utf-8")
        js.write_text(text.rstrip("\n") + f"\n//# sourceMappingURL={map_name}\n", encoding="utf-8")
        print(f"source map: {js.relative_to(public)}")

    if args.icons:
        make_pwa(public, base, args.version, args.icons)
    shutil.copyfile(index, public / "404.html")
    return 0


if __name__ == "__main__":
    sys.exit(main())
