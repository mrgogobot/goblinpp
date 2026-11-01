#!/usr/bin/env python3
"""Exercise owned native LAN programs with separate role folders; never alter firewalls."""
import argparse,hashlib,ipaddress,json,os,shutil,socket,subprocess,tempfile,time
from pathlib import Path

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);p.add_argument('--report',required=True,type=Path);p.add_argument('--try-local-lan-interface',action='store_true');a=p.parse_args();programs=a.directory.resolve();manifest=programs/'BUILD_MANIFEST.json';manifest=manifest if manifest.is_file() else programs.parent/'BUILD_MANIFEST.json';m=json.loads(manifest.read_text())
 for entry in m['programs']:assert hashlib.sha256((programs/entry['file']).read_bytes()).hexdigest()==entry['sha256']
 results=[]
 def exchange(server_ip,label):
  with tempfile.TemporaryDirectory(prefix='goblin-native-lan-public-fixtures-')as t:
   root=Path(t);setup=subprocess.run([str(programs/'network_setup')],cwd=root,input=server_ip+'\n',text=True,capture_output=True,timeout=20);assert setup.returncode==0,setup.stdout+setup.stderr
   server_folder=root/'lan-pair/server';client_folder=root/'lan-pair/client'
   assert not(server_folder/'client-key.pem').exists() and not(client_folder/'server-key.pem').exists()
   # Reserve/check the exact address first. Never kill another listener or connect to it deliberately.
   with socket.socket()as reserve:reserve.bind((server_ip,7443))
   server=subprocess.Popen([str(programs/'network_server')],cwd=server_folder,text=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
   try:
    deadline=time.monotonic()+5
    while True:
     client=subprocess.run([str(programs/'network_client')],cwd=client_folder,text=True,capture_output=True,timeout=15)
     if client.returncode==0:break
     if server.poll()is not None or time.monotonic()>deadline:raise AssertionError(client.stdout+client.stderr)
     time.sleep(.02)
    stdout,stderr=server.communicate(timeout=15);assert server.returncode==0,stdout+stderr
    assert 'NETWORK_EXCHANGE_VERIFIED=true'in stdout and 'NETWORK_EXCHANGE_VERIFIED=true'in client.stdout
    assert 'RUN_STATUS=PASS'in stdout and 'RUN_STATUS=PASS'in client.stdout
   finally:
    if server.poll()is None:server.kill();server.communicate()
   # An existing pairing cannot be overwritten or silently re-keyed.
   old=(server_folder/'server-key.pem').read_bytes()
   repeated=subprocess.run([str(programs/'network_setup')],cwd=root,input=server_ip+'\n',text=True,capture_output=True,timeout=20);assert repeated.returncode!=0;assert(server_folder/'server-key.pem').read_bytes()==old
   assert not list(root.rglob('receipt.json'))and not list(root.rglob('.goblin'))
   results.append({'case':label,'status':'PASS','separate_role_keys':True,'expected_greeting_verified_on_both_sides':True,'overwrite_refused':True,'scientific_receipts':False,'server_output':stdout,'client_output':client.stdout})
 exchange('127.0.0.1','NATIVE_LOOPBACK_SEPARATE_ROLES')
 if a.try_local_lan_interface:
  candidate=None
  if Path('/usr/sbin/ipconfig').exists():
   for iface in ['en0','en1']:
    r=subprocess.run(['/usr/sbin/ipconfig','getifaddr',iface],text=True,capture_output=True)
    if r.returncode==0:
     try:address=ipaddress.IPv4Address(r.stdout.strip())
     except ValueError:continue
     if any(address in net for net in [ipaddress.IPv4Network('10.0.0.0/8'),ipaddress.IPv4Network('172.16.0.0/12'),ipaddress.IPv4Network('192.168.0.0/16')]):candidate=str(address);break
  if candidate:exchange(candidate,'NATIVE_NONLOOPBACK_LAN_INTERFACE_SAME_COMPUTER')
  else:results.append({'case':'NATIVE_NONLOOPBACK_LAN_INTERFACE_SAME_COMPUTER','status':'NOT_RUN','reason':'No supported active RFC1918 interface detected; no interface or firewall change attempted.'})
 proof={'schema':'goblin.lan-native-validation.v1','engine_version':m['engine_version'],'target':m['target'],'physical_two_computer_test':'NOT_PERFORMED','programs':m['programs'],'cases':results,'claim_limit':'Separate folders on one computer; the optional test binds an actual private LAN interface but does not cross a physical second host.'};a.report.parent.mkdir(parents=True,exist_ok=True);a.report.write_text(json.dumps(proof,indent=2)+'\n');print(json.dumps({'status':'PASS','cases':[{k:r[k]for k in ['case','status']}for r in results],'physical_two_computer_test':'NOT_PERFORMED'},indent=2))
if __name__=='__main__':main()
