#!/usr/bin/env python3
"""Compile the public LAN examples with the reviewed local engine; no private keys."""
import hashlib,json,subprocess,sys,platform
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def main():
 engine=ROOT/'target/release/goblinpp';version=subprocess.check_output([str(engine),'--version'],text=True).strip().removeprefix('goblin++ ')
 target='macos-arm64'if sys.platform=='darwin'and platform.machine()=='arm64'else platform.system().lower()+'-'+platform.machine()
 out=ROOT/'dist/network-demo'/version/target;out.mkdir(parents=True,exist_ok=True);records=[]
 for name,source in [('network_setup','setup.gbl'),('network_server','network_server.gbl'),('network_client','network_client.gbl')]:
  path=ROOT/'examples/network/lan'/source;exe=out/name
  subprocess.run([str(engine),'compile',str(path),'--output',str(exe)],check=True)
  records.append({'file':name,'bytes':exe.stat().st_size,'sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'source':str(path.relative_to(ROOT)),'source_sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
 (out/'BUILD_MANIFEST.json').write_text(json.dumps({'schema':'goblin.lan-native-build.v1','engine_version':version,'engine_sha256':hashlib.sha256(engine.read_bytes()).hexdigest(),'target':target,'programs':records},indent=2)+'\n');print(out)
if __name__=='__main__':main()
