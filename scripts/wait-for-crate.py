#!/usr/bin/env python3
"""Wait for a newly published crate to become visible in the sparse index."""

import json
import sys
import time
import urllib.error
import urllib.request

name, version = sys.argv[1:]
if len(name) < 4:
    raise SystemExit("only four-character-or-longer crate names are supported")
url = f"https://index.crates.io/{name[:2]}/{name[2:4]}/{name}"
for attempt in range(40):
    try:
        with urllib.request.urlopen(url, timeout=15) as response:
            if any(json.loads(line)["vers"] == version for line in response):
                print(f"{name} {version} is indexed")
                sys.exit(0)
    except (urllib.error.URLError, ValueError):
        pass
    print(f"Waiting for {name} {version} in the crates.io index ({attempt + 1}/40)", flush=True)
    time.sleep(15)
raise SystemExit(f"{name} {version} did not appear in the crates.io index")
