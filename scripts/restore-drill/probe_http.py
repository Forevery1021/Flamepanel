"""R-01B trusted, dedicated probe process. Not a sandbox for arbitrary executables."""
import json
import math
import os
from pathlib import Path
import secrets
import selectors
import signal
import subprocess
import time
import urllib.error
import urllib.request

MODE = 'flamepanel/restore-probe/v1'


class ProbeFailure(Exception):
    pass


def remaining(deadline):
    value = deadline - time.monotonic()
    if value <= 0:
        raise ProbeFailure('PROBE_TIMEOUT')
    return value


def readiness(process, deadline):
    data = b''
    with selectors.DefaultSelector() as selector:
        selector.register(process.stdout, selectors.EVENT_READ)
        while b'\n' not in data:
            if not selector.select(timeout=remaining(deadline)):
                raise ProbeFailure('PROBE_TIMEOUT')
            chunk = os.read(process.stdout.fileno(), 4096)
            if not chunk:
                raise ProbeFailure('PROBE_START_FAILED')
            data += chunk
            if len(data) > 4096:
                raise ProbeFailure('PROBE_PROTOCOL_FAILED')
    try:
        result = json.loads(data.split(b'\n', 1)[0])
        port = result['port']
        if result['mode'] != MODE or result.get('migrated') is not True:
            raise ValueError
        if type(port) is not int or not 0 < port < 65536:
            raise ValueError
        return port
    except (ValueError, KeyError, TypeError) as error:
        raise ProbeFailure('PROBE_PROTOCOL_FAILED') from error


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise ProbeFailure('PROBE_REDIRECT_REFUSED')


def request(port, path, token, deadline, method='GET'):
    headers = {'Authorization': 'Bearer ' + token} if token else {}
    req = urllib.request.Request(f'http://127.0.0.1:{port}{path}', headers=headers, method=method)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        response = opener.open(req, timeout=min(2, remaining(deadline)))
    except urllib.error.HTTPError as error:
        response = error
    with response:
        body = response.read(16385)
        if len(body) > 16384:
            raise ProbeFailure('PROBE_RESPONSE_LIMIT')
        return response.code, body


def exercise(port, token, deadline):
    # Verify fail-closed auth and absent production/write routes on the actual TCP listener.
    for path, supplied, method, expected in [
            ('/health', None, 'GET', 401),
            ('/summary', '0' * 64, 'GET', 401),
            ('/health?token=' + token, None, 'GET', 401),
            ('/summary', token, 'POST', 405),
            ('/api/tasks', token, 'GET', 404)]:
        if request(port, path, supplied, deadline, method)[0] != expected:
            raise ProbeFailure('PROBE_AUTH_OR_ROUTE_FAILED')
    status, body = request(port, '/health', token, deadline)
    health = json.loads(body)
    if status != 200 or health.get('mode') != MODE or health.get('production_kernel') is not False:
        raise ProbeFailure('PROBE_HEALTH_FAILED')
    if health.get('status') != 'ready':
        raise ProbeFailure('PROBE_HEALTH_FAILED')
    status, body = request(port, '/summary', token, deadline)
    summary = json.loads(body)
    if status != 200 or summary.get('mode') != MODE:
        raise ProbeFailure('PROBE_READ_FAILED')
    counts = summary.get('counts', {})
    if set(counts) != {'users', 'websites', 'scheduled_tasks'} or any(
            type(v) is not int or v < 0 for v in counts.values()):
        raise ProbeFailure('PROBE_READ_FAILED')
    return {'mode': MODE, 'migration_passed': True, 'http_auth_passed': True,
            'write_routes_absent': True, 'counts': counts,
            'production_kernel': False, 'production_auth_verified': False}


def stop(process):
    if process.poll() is None:
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    try:
        code = process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait(timeout=5)
        raise ProbeFailure('PROBE_FORCED_STOP')
    if code != 0:
        raise ProbeFailure('PROBE_EXIT_FAILED')


def run_probe(binary, snapshot, work, deadline):
    binary = Path(binary).resolve(strict=True)
    if binary.name != 'fp-restore-probe' or not binary.is_file():
        raise ProbeFailure('DEDICATED_PROBE_REQUIRED')
    token = secrets.token_hex(32)
    started = time.monotonic()
    ttl = min(300, max(1, math.ceil(remaining(deadline))))
    # No shell, inherited OP_ settings, proxy variables, credentials or dynamic loader injection.
    process = subprocess.Popen(
        [str(binary), '--snapshot', str(snapshot), '--ttl', str(ttl)],
        cwd=work, env={'PATH': os.defpath, 'LANG': 'C.UTF-8', 'TMPDIR': str(work)},
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        start_new_session=True)
    try:
        process.stdin.write((token + '\n').encode('ascii'))
        process.stdin.close()
        port = readiness(process, deadline)
        result = exercise(port, token, deadline)
        result['probe_seconds'] = round(time.monotonic() - started, 6)
    except BaseException:
        # Preserve the primary failure, but always reap the owned child before workdir cleanup.
        try:
            stop(process)
        except ProbeFailure:
            pass
        raise
    else:
        stop(process)
        result['stopped_cleanly'] = True
        if list(Path(work).glob('.probe-*')):
            raise ProbeFailure('PROBE_COPY_REMAINS')
        return result
    finally:
        if not process.stdin.closed:
            process.stdin.close()
        process.stdout.close()
