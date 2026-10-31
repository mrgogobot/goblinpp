#!/usr/bin/env python3
"""Public fixtures only: test installed private-script functions, never a real secret."""
from pathlib import Path
import argparse,subprocess,tempfile,shutil


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--goblinpp',type=Path,required=True)
    args=parser.parse_args();binary=args.goblinpp.resolve()
    example=Path(__file__).resolve().parents[1]/'examples/private_encryption.gbl'
    key='public installed private test passphrase'
    message='Public installed demonstration: {braces stay literal}.'
    answers='\n'.join([key,'public deliberately different confirmation',key,key,message])+'\n'
    with tempfile.TemporaryDirectory(prefix='goblin-private-installed-public-') as temporary:
        for mode in ['interpreted','compiled']:
            folder=Path(temporary)/mode;folder.mkdir();shutil.copy2(example,folder/'private_encryption.gbl')
            env=__import__('os').environ.copy();env['GOBLIN_SECRET_PROMPT_PROTOCOL']='hex-v1'
            env['GOBLIN_NATIVE_RESULT_PATH']=str(folder/'must-not-write-native-result')
            env['GOBLIN_NATIVE_DATA_PATH']=str(folder/'must-not-write-native-data')
            command=[str(binary),'run','private_encryption.gbl']+(['--compile']if mode=='compiled'else[])
            result=subprocess.run(command,cwd=folder,input=answers,text=True,capture_output=True,env=env)
            assert result.returncode==0,result.stdout+result.stderr
            assert key not in result.stdout+result.stderr
            assert result.stderr.count('GOBLIN_SECRET_PROMPT_V1\t')==5,result.stderr
            assert 'Passphrases differ; try again.'in result.stdout
            assert message in result.stdout
            assert 'RUN_STATUS=PASS'in result.stdout and 'NO_RUN_RECEIPT'in result.stdout
            assert 'RUN_DIR='not in result.stdout
            assert (folder/'private-restored.txt').read_text()==message
            assert {p.name for p in folder.iterdir()}=={'private_encryption.gbl','private-message.age','private-restored.txt'}
            print(f'Private {mode} installed-script check: PASS (public fixtures, no scientific receipt)')

if __name__=='__main__':main()
