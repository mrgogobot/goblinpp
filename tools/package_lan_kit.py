#!/usr/bin/env python3
"""Create a credential-free standalone LAN test kit from verified native programs."""
import hashlib,json,shutil,tomllib,zipfile
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
def sha(b):return hashlib.sha256(b).hexdigest()
def main():
 version=tomllib.loads((ROOT/'Cargo.toml').read_text())['package']['version'];src=ROOT/'dist/network-demo'/version/'macos-arm64';build=json.loads((src/'BUILD_MANIFEST.json').read_text());assert build['engine_sha256']==sha((ROOT/'target/release/goblinpp').read_bytes());validation=json.loads((ROOT/'dist/releases/alpha45-validation/NATIVE_LAN_VALIDATION.json').read_text());assert validation['cases'][0]['status']=='PASS'
 out=ROOT/'dist/releases/alpha45-lan-kit-reviewed';out.mkdir(parents=True,exist_ok=True);filename=f'Goblinpp-LAN-{version}-macos-arm64.zip';archive=out/filename;receipt=out/'PACKAGE_RECEIPT.json';sums=out/'SHA256SUMS.txt'
 for p in [archive,receipt,sums]:assert not p.exists(),p
 data={}
 for program in build['programs']:
  payload=(src/program['file']).read_bytes();assert sha(payload)==program['sha256'];assert sha((ROOT/program['source']).read_bytes())==program['source_sha256'];data['macos-arm64/'+program['file']]=(payload,0o755)
 for name in ['setup.gbl','network_server.gbl','network_client.gbl']:data['source/'+name]=((ROOT/'examples/network/lan'/name).read_bytes(),0o644)
 data['README.md']=((ROOT/'examples/network/lan/README.md').read_bytes(),0o644)
 for name in ['LICENSE','LICENSE-DOCS.md','THIRD_PARTY_NOTICES.md']:data[name]=((ROOT/name).read_bytes(),0o644)
 data['BUILD_MANIFEST.json']=(json.dumps(build,indent=2).encode()+b'\n',0o644)
 for path in sorted((ROOT/'third-party').rglob('*')):
  if path.is_file():data[str(path.relative_to(ROOT))]=(path.read_bytes(),0o644)
 prefix=f'Goblinpp-LAN-{version}-macos-arm64/'
 with zipfile.ZipFile(archive,'x',compression=zipfile.ZIP_DEFLATED,compresslevel=9)as z:
  for name,(payload,mode)in sorted(data.items()):
   assert 'lan-pair/'not in name and not name.endswith(('.key','.pem'))
   info=zipfile.ZipInfo(prefix+name,(1980,1,1,0,0,0));info.create_system=3;info.external_attr=(0o100000|mode)<<16;info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,payload)
 with zipfile.ZipFile(archive)as z:
  assert z.testzip()is None
  for name,(payload,_)in data.items():assert z.read(prefix+name)==payload
 artifact={'file':filename,'bytes':archive.stat().st_size,'sha256':sha(archive.read_bytes())};proof={'schema':'goblin.lan-kit-package.v1','version':version,'target':'macos-arm64','artifact':artifact,'programs':build['programs'],'inventory':{name:sha(value[0])for name,value in data.items()},'credentials_included':False,'native_loopback_test':'PASS','physical_two_computer_test':'NOT_PERFORMED','linux_native_binaries_included':False,'linux_source_and_build_instructions_included':True,'authentication':'CHECKSUM_ONLY_NOT_A_SIGNATURE'};receipt.write_text(json.dumps(proof,indent=2)+'\n');sums.write_text(artifact['sha256']+'  '+filename+'\n'+sha(receipt.read_bytes())+'  PACKAGE_RECEIPT.json\n');shutil.copy2(ROOT/'dist/releases/alpha45-validation/NATIVE_LAN_VALIDATION.json',out/'LOCAL_VALIDATION.json');print(json.dumps(artifact,indent=2))
if __name__=='__main__':main()
