import http.server
import json
import pathlib
import time
import sys

root = pathlib.Path(sys.argv[1]).resolve()
if not (root / "web" / "index.html").is_file():
    raise SystemExit("Pass a completed run directory containing web/index.html")


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *arguments, **keywords):
        super().__init__(*arguments, directory=str(root / "web"), **keywords)

    def do_POST(self):
        if self.path != "/result":
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", "0"))
        if not 0 < length < 2_000_000:
            self.send_error(413)
            return
        result = json.loads(self.rfile.read(length))
        destination = root / f"browser-depth-{time.time_ns()}.json"
        destination.write_text(json.dumps(result, indent=2))
        self.send_response(200)
        self.end_headers()
        self.wfile.write(destination.name.encode())


http.server.ThreadingHTTPServer(("127.0.0.1", 8765), Handler).serve_forever()
