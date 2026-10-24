#!/usr/bin/env python3
"""Read-only Humaan telemetry bridge. Pipe calibrated measurements as JSON lines.

Each stdin line: {"shoulder_pan": 0.12, "shoulder_lift": -0.2, ...}
No motor driver is included and this program never sends commands to hardware.
"""
import argparse
import json
import math
import socketserver
import sys
import threading


def validate_positions(value):
    if not isinstance(value, dict) or not 1 <= len(value) <= 128:
        raise ValueError("Expected 1–128 named positions")
    if not all(isinstance(k, str) and k and isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v) for k, v in value.items()):
        raise ValueError("Positions must be finite numbers")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=9876)
    args = parser.parse_args()
    lock = threading.Lock()
    packet = None
    sequence = 0

    def receive():
        nonlocal packet, sequence
        for line in sys.stdin:
            try:
                positions = validate_positions(json.loads(line))
                with lock:
                    sequence += 1
                    packet = (json.dumps({"sequence": sequence, "positions": positions}) + "\n").encode()
            except (ValueError, TypeError) as error:
                print(f"Invalid sample: {error}", file=sys.stderr)
        print("Input ended. Live Link will report stale telemetry.", file=sys.stderr)

    class Handler(socketserver.BaseRequestHandler):
        def handle(self):
            with lock:
                current = packet
            if current:
                self.request.settimeout(0.5)
                try:
                    self.request.sendall(current)
                except OSError:
                    pass

    class Server(socketserver.ThreadingTCPServer):
        allow_reuse_address = True
        daemon_threads = True

    with Server(("127.0.0.1", args.port), Handler) as server:
        threading.Thread(target=receive, daemon=True).start()
        print(f"Read-only telemetry bridge on 127.0.0.1:{args.port}. Waiting for calibrated JSON positions on stdin.", file=sys.stderr)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
