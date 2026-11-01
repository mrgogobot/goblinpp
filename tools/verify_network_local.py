#!/usr/bin/env python3
"""Public localhost fixtures: installed Goblin++ versus independent OpenSSL TLS 1.3.

No secrets from the user, public listeners, user-prefix installs or scientific receipts.
"""
from pathlib import Path
import argparse, base64, shutil, socket, ssl, struct, subprocess, tempfile, threading, time


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--goblinpp', type=Path, required=True)
    p.add_argument('--compile', action='store_true', help='also test native client/server executables')
    a = p.parse_args(); binary = a.goblinpp.resolve()
    source = Path(__file__).resolve().parents[1] / 'examples'
    with tempfile.TemporaryDirectory(prefix='goblin-network-installed-public-') as temporary:
        root = Path(temporary)
        for name in ['network_setup.gbl', 'network_client.gbl', 'network_server.gbl']:
            shutil.copy2(source / name, root / name)
        def run(args):
            result = subprocess.run([str(binary), *args], cwd=root, text=True, capture_output=True, timeout=300)
            assert result.returncode == 0, result.stdout + result.stderr
            return result.stdout
        run(['run', 'network_setup.gbl'])
        creds = root / 'network-demo'
        server_source = (root / 'network_server.gbl').read_text().replace('"127.0.0.1:7443"', 'argv(1)')
        (root / 'test_server.gbl').write_text(server_source)
        if a.compile:
            run(['compile', 'network_client.gbl', '--output', 'client-native'])
            run(['compile', 'test_server.gbl', '--output', 'server-native'])
        envelope_source = '''GO_PRIVATE
recipient = read_text("network-demo/recipient.txt")
envelope = e2e_encrypt("Hello from Goblin++!", recipient)
print("ENVELOPE={envelope}")
'''
        (root / 'make_envelope.gbl').write_text(envelope_source)
        envelope = next(line.removeprefix('ENVELOPE=') for line in run(['run', 'make_envelope.gbl']).splitlines() if line.startswith('ENVELOPE='))
        assert base64.b64decode(envelope, validate=True).startswith(b'age-encryption.org/v1\n')
        def context(server=False):
            c = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER if server else ssl.PROTOCOL_TLS_CLIENT)
            c.minimum_version = c.maximum_version = ssl.TLSVersion.TLSv1_3
            c.load_verify_locations(str(creds / 'ca.pem'))
            role = 'server' if server else 'client'
            c.load_cert_chain(str(creds / f'{role}.pem'), str(creds / f'{role}-key.pem'))
            c.verify_mode = ssl.CERT_REQUIRED
            c.set_alpn_protocols(['goblin-message/1'])
            return c
        def read_exact(s, n):
            parts = bytearray()
            while len(parts) < n:
                part = s.recv(n - len(parts)); assert part, 'truncated frame'; parts.extend(part)
            return bytes(parts)
        def receive(s):
            n = struct.unpack('!I', read_exact(s, 4))[0]; assert n <= 1048576
            return read_exact(s, n).decode('utf-8')
        def send(s, message):
            data = message.encode(); s.sendall(struct.pack('!I', len(data)) + data)
        for native in ([False, True] if a.compile else [False]):
            # The listening OpenSSL peer is ready before Goblin++ starts.
            listener = socket.socket(); listener.bind(('127.0.0.1', 0)); listener.listen(1); listener.settimeout(5)
            address = f'127.0.0.1:{listener.getsockname()[1]}'
            errors = []; observations = []
            def openssl_server():
                try:
                    with listener:
                        connection, _ = listener.accept(); connection.settimeout(5)
                        with context(True).wrap_socket(connection, server_side=True) as secure:
                            assert secure.version() == 'TLSv1.3'
                            assert secure.selected_alpn_protocol() == 'goblin-message/1'
                            observations.append(receive(secure)); send(secure, 'OpenSSL transport verified')
                except BaseException as e: errors.append(e)
            t = threading.Thread(target=openssl_server); t.start()
            command = [str(root / 'client-native'), address] if native else [str(binary), 'run', 'network_client.gbl', '--', address]
            result = subprocess.run(command, cwd=root, text=True, capture_output=True, timeout=10)
            t.join(6); assert not t.is_alive(); assert not errors, errors
            assert result.returncode == 0, result.stdout + result.stderr
            assert base64.b64encode(b'OpenSSL transport verified').decode() in result.stdout
            assert len(observations) == 1 and 'Hello from' not in observations[0]
            (root / 'received-envelope.txt').write_text(observations[0])
            (root / 'decrypt_check.gbl').write_text('GO_PRIVATE\nenvelope=read_text("received-envelope.txt")\nmessage=e2e_decrypt(envelope,"network-demo/recipient.key")\nif message == "Hello from Goblin++!" { print("recipient-check-pass") }\n')
            assert 'recipient-check-pass' in run(['run', 'decrypt_check.gbl'])
            # The inverse direction exercises a Goblin++ server, verified by OpenSSL.
            with socket.socket() as reserve:
                reserve.bind(('127.0.0.1', 0)); port = reserve.getsockname()[1]
            address = f'127.0.0.1:{port}'
            command = [str(root / 'server-native'), address] if native else [str(binary), 'run', 'test_server.gbl', '--', address]
            child = subprocess.Popen(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                deadline = time.monotonic() + 5
                while True:
                    try: connection = socket.create_connection(('127.0.0.1', port), timeout=2); break
                    except ConnectionRefusedError:
                        if child.poll() is not None or time.monotonic() > deadline: raise
                        time.sleep(.01)
                with context().wrap_socket(connection, server_hostname='localhost') as secure:
                    assert secure.version() == 'TLSv1.3'
                    assert secure.selected_alpn_protocol() == 'goblin-message/1'
                    send(secure, envelope)
                    assert receive(secure) == 'Message decrypted by the recipient.'
                stdout, stderr = child.communicate(timeout=10)
                assert child.returncode == 0, stdout + stderr
                assert 'Recipient decrypted 20 characters.' in stdout
            finally:
                if child.poll() is None: child.kill(); child.communicate()
            print(f'Installed {"native" if native else "interpreted"} client/server + independent OpenSSL mutual TLS 1.3 + age recipient decryption: PASS')
        assert not (root / 'runs').exists() and not (root / '.goblin').exists()
        assert not list(root.rglob('receipt.json'))
        print('Networking public fixtures: no scientific receipts/ledger, no bundled credentials: PASS')

if __name__ == '__main__':
    main()
