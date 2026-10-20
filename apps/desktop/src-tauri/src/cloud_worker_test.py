import io
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from cloud_worker import run_job, make_server


class CloudSupervisorTests(unittest.TestCase):
    def package(self): return {'experiment': {'id': '123'}, 'worker': '# test'}

    def test_install_failure_persists_error(self):
        def call(args, **kwargs):
            if 'pip' in args: raise subprocess.CalledProcessError(1, args)
            return subprocess.CompletedProcess(args, 0, '')
        with tempfile.TemporaryDirectory() as directory:
            result = run_job(self.package(), directory, call=call)
            self.assertEqual(result['status'], 'failed')
            self.assertEqual(json.loads((Path(directory)/'status.json').read_text())['status'], 'failed')

    def test_success_persists_result_before_completion(self):
        def call(args, **kwargs): return subprocess.CompletedProcess(args, 0, 'GPU, 0 %, 0 MiB, 24000 MiB')
        class Process:
            returncode = 0
            def __init__(self, args, stdin, stdout, **kwargs):
                stdout.write('HUMAAN_PROGRESS:{"iteration":1,"loss":0.01}\nHUMAAN_RESULT:' + json.dumps({'experimentId': '123', 'policy': {}, 'evaluation': {'successRate': 1}})); stdout.flush()
            def poll(self): return 0
            def wait(self): return 0
        with tempfile.TemporaryDirectory() as directory, patch('cloud_worker.subprocess.Popen', Process):
            state = run_job(self.package(), directory, call=call)
            self.assertEqual(state['status'], 'completed')
            self.assertEqual(state['progress']['iteration'], 1)
            self.assertEqual(json.loads((Path(directory)/'result.json').read_text())['experimentId'], '123')

    def test_http_authentication_idempotency_and_restart(self):
        class Socket:
            def __init__(self, data): self.input = io.BytesIO(data); self.output = b''
            def makefile(self, *args): return self.input
            def sendall(self, data): self.output += data
            def settimeout(self, timeout): pass
        with tempfile.TemporaryDirectory() as directory, patch('cloud_worker.ThreadingHTTPServer', side_effect=lambda address, handler: handler), patch('cloud_worker.threading.Thread') as worker:
            handler = make_server(directory, 'secret-token')
            def send(path, token='secret-token', package=None):
                body = json.dumps(package).encode() if package else b''
                request = ('POST' if package else 'GET')+' '+path+' HTTP/1.1\r\nHost: test\r\nAuthorization: Bearer '+token+'\r\nContent-Length: '+str(len(body))+'\r\n\r\n'
                socket = Socket(request.encode()+body); handler(socket, ('127.0.0.1', 1), object()); return socket.output
            self.assertIn(b'401', send('/status', 'wrong'))
            self.assertIn(b'waiting', send('/status'))
            self.assertIn(b'202', send('/start', package=self.package()))
            self.assertIn(b'200', send('/start', package=self.package()))
            self.assertEqual(worker.call_count, 1)
            self.assertIn(b'409', send('/start', package={'experiment': {'id': 'different'}, 'worker': '# test'}))
            self.assertIn(b'404', send('/../../etc/passwd'))
            make_server(directory, 'secret-token')
            self.assertEqual(json.loads((Path(directory)/'status.json').read_text())['status'], 'interrupted')


if __name__ == '__main__': unittest.main()
