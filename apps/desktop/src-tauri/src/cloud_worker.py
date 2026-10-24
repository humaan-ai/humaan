"""One authenticated RunPod job, persisted on its pod volume. No account key is sent here."""
import hashlib
import hmac
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import threading
import time


def atomic(path, value):
    temporary = path.with_suffix('.tmp')
    temporary.write_text(json.dumps(value, allow_nan=False))
    temporary.replace(path)


def run_job(package, directory, call=subprocess.run):
    directory = Path(directory)
    state = {'status': 'preparing', 'experimentId': package['experiment']['id'], 'progress': None}
    started = time.monotonic()
    def publish():
        state['updatedAt'] = int(time.time()*1000)
        state['elapsedSeconds'] = int(time.monotonic()-started)
        atomic(directory/'status.json', state)
    def progress():
        text = (directory/'worker.log').read_text(errors='replace')
        for line in reversed(text.splitlines()):
            if line.startswith('HUMAAN_PROGRESS:'):
                state['progress'] = json.loads(line.split(':', 1)[1]); break
        try:
            gpu = call(['nvidia-smi', '--query-gpu=name,utilization.gpu,memory.used,memory.total', '--format=csv,noheader'], capture_output=True, text=True, timeout=5)
            state['gpu'] = gpu.stdout.strip() if gpu.returncode == 0 else 'Unavailable'
        except (OSError, subprocess.SubprocessError): state['gpu'] = 'Unavailable'
        publish()
    try:
        publish()
        (directory/'worker.py').write_text(package['worker'])
        (directory/'experiment.json').write_text(json.dumps(package['experiment']))
        call(['python3', '-m', 'venv', str(directory/'venv')], check=True, timeout=60)
        python = str(directory/'venv/bin/python')
        with (directory/'worker.log').open('w') as log:
            call([python, '-m', 'pip', 'install', '--disable-pip-version-check', 'mujoco==3.2.7', 'numpy==2.0.2'], stdout=log, stderr=subprocess.STDOUT, check=True, timeout=240)
        state['status'] = 'running'; publish()
        with (directory/'experiment.json').open() as payload, (directory/'worker.log').open('a') as log:
            process = subprocess.Popen([python, '-I', str(directory/'worker.py')], stdin=payload, stdout=log, stderr=subprocess.STDOUT)
            try:
                deadline = time.monotonic()+240
                while process.poll() is None:
                    if time.monotonic() > deadline: raise TimeoutError('Training exceeded 240 seconds.')
                    progress(); time.sleep(3)
            finally:
                if process.poll() is None: process.kill()
                process.wait()
        progress()
        output = (directory/'worker.log').read_text(errors='replace')
        if process.returncode != 0 or 'HUMAAN_RESULT:' not in output: raise RuntimeError('Training failed. See logs.')
        result = json.loads(output.rsplit('HUMAAN_RESULT:', 1)[1])
        if 'error' not in result and result.get('experimentId') != package['experiment']['id']: raise ValueError('Result experiment mismatch.')
        atomic(directory/'result.json', result)
        state['status'] = 'failed' if 'error' in result else 'completed'
        state['error'] = result.get('error')
        if 'evaluation' in result: state['evaluation'] = result['evaluation']
    except Exception as error:
        state['status'] = 'failed'; state['error'] = str(error)
    finally: publish()
    return state


def make_server(directory, token, address=('0.0.0.0', 8080)):
    directory = Path(directory); directory.mkdir(parents=True, exist_ok=True)
    lock = threading.Lock()
    if (directory/'status.json').exists():
        state = json.loads((directory/'status.json').read_text())
        if state['status'] in ('running', 'preparing'):
            state.update(status='interrupted', error='Pod restarted before completion.')
            atomic(directory/'status.json', state)
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *args): pass  # Never log credentials or request bodies.
        def setup(self):
            super().setup(); self.connection.settimeout(120)
        def respond(self, code, value):
            body = json.dumps(value, allow_nan=False).encode()
            self.send_response(code); self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(body))); self.end_headers(); self.wfile.write(body)
        def authorized(self):
            if not hmac.compare_digest(self.headers.get('Authorization', ''), 'Bearer '+token):
                self.respond(401, {'error': 'Unauthorized'}); return False
            return True
        def do_GET(self):
            if not self.authorized(): return
            if self.path == '/status':
                state = json.loads((directory/'status.json').read_text()) if (directory/'status.json').exists() else {'status': 'waiting'}
                log = directory/'worker.log'
                if log.exists():
                    with log.open('rb') as stream:
                        stream.seek(max(0, log.stat().st_size-65536)); state['logs'] = stream.read().decode(errors='replace')
                self.respond(200, state)
            elif self.path == '/result' and (directory/'result.json').exists():
                self.respond(200, json.loads((directory/'result.json').read_text()))
            else: self.respond(404, {'error': 'Not available'})
        def do_POST(self):
            if not self.authorized(): return
            if self.path != '/start': self.respond(404, {}); return
            try:
                length = int(self.headers.get('Content-Length', '0'))
                if not 0 < length <= 80*1024*1024: self.respond(413, {'error': 'Package exceeds 80 MB'}); return
                body = self.rfile.read(length)
                if len(body) != length: raise ValueError('Incomplete upload')
                package = json.loads(body)
                if not isinstance(package.get('worker'), str) or not isinstance(package.get('experiment'), dict): raise ValueError('Invalid package')
                digest = hashlib.sha256(body).hexdigest()
                with lock:
                    if (directory/'package.sha256').exists():
                        self.respond(200 if (directory/'package.sha256').read_text() == digest else 409, {'status': 'already-submitted'}); return
                    # A retry after lost response cannot start a second worker.
                    (directory/'package.sha256').write_text(digest)
                    atomic(directory/'status.json', {'status': 'preparing', 'updatedAt': int(time.time()*1000)})
                    threading.Thread(target=run_job, args=(package, directory), daemon=True).start()
                self.respond(202, {'status': 'preparing'})
            except (ValueError, KeyError) as error: self.respond(400, {'error': str(error)})
    return ThreadingHTTPServer(address, Handler)


if __name__ == '__main__':
    make_server('/workspace/humaan', os.environ['HUMAAN_JOB_TOKEN']).serve_forever()
