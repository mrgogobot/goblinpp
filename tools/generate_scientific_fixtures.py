#!/usr/bin/env python3
"""Development-only upstream fixtures; Python is never used by native Goblin++ readers.

Run each group in an environment with its named upstream libraries installed.
The output folder must not exist. Fixtures are synthetic, not research data.
"""
import argparse,hashlib,json,platform,struct
from pathlib import Path
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--group',choices=['numpy-scipy','astropy'],required=True)
p.add_argument('--output',required=True,type=Path)
a=p.parse_args(); a.output.mkdir(parents=True,exist_ok=False)
import numpy as np
versions={'python':platform.python_version(),'numpy':np.__version__}
expect={}
if a.group=='numpy-scipy':
 import scipy
 import scipy.sparse as sp
 versions['scipy']=scipy.__version__
 matrix=np.array([[1.,2.,3.],[4.,5.,6.]],dtype=np.float64)
 arrays={'numeric_c':matrix,'numeric_f':np.asfortranarray(matrix),'big_endian':np.array([0.1,-0.,1e-30],dtype='>f4'),'ids':np.array([9007199254740993,18446744073709551615],dtype=np.uint64),'booleans':np.array([True,False]),'unicode':np.array(['Ada π','科学','']),'bytes':np.array([b'A',b'\xcf\x80'],dtype='S2'),'scalar':np.array(3.5),'empty':np.zeros((0,3),dtype=np.float32)}
 for name,arr in arrays.items():
  np.save(a.output/f'{name}.npy',arr,allow_pickle=False)
  flat=arr.ravel(order='C'); values=[str(x) for x in flat] if arr.dtype.kind=='u' else [x.decode('utf8') for x in flat.tolist()] if arr.dtype.kind=='S' else flat.tolist()
  expect[name]={'shape':list(arr.shape),'dtype':arr.dtype.str,'values':values}
  if arr.dtype.kind=='f': expect[name]['f64_bits']=[struct.pack('>d',float(x)).hex() for x in flat]
 for version in [(2,0),(3,0)]:
  with (a.output/f'version{version[0]}.npy').open('xb') as f:np.lib.format.write_array(f,matrix,version=version,allow_pickle=False)
 np.save(a.output/'objects.npy',np.array([{'benign':'pickle fixture, never execute'}],dtype=object))
 np.save(a.output/'nonfinite.npy',np.array([1.,np.nan,np.inf]))
 np.save(a.output/'invalid_utf8.npy',np.array([b'\xff'],dtype='S1'),allow_pickle=False)
 for name,writer in [('stored',np.savez),('compressed',np.savez_compressed)]:writer(a.output/f'{name}.npz',matrix=arrays['numeric_f'],ids=arrays['ids'],labels=arrays['unicode'])
 dense=np.array([[0.,1.,0.,2.],[3.,0.,0.,0.],[0.,0.,4.,0.]])
 for name,maker in [('csr',sp.csr_matrix),('csc',sp.csc_matrix),('coo',sp.coo_matrix)]:sp.save_npz(a.output/f'{name}.npz',maker(dense))
 sp.save_npz(a.output/'csr_array.npz',sp.csr_array(dense))
 sp.save_npz(a.output/'duplicates.npz',sp.coo_matrix(([1.,2.],([0,0],[1,1])),shape=(1,3)))
 expect['sparse']={'shape':list(dense.shape),'values':dense.ravel(order='C').tolist(),'nnz':int(np.count_nonzero(dense))}
else:
 import astropy
 from astropy.table import Table,MaskedColumn
 from astropy.time import Time
 versions['astropy']=astropy.__version__
 t=Table({'distance':np.array([1.25,2.5],dtype=np.float32),'velocity':np.array([3.,4.]),'source_id':np.array([9007199254740993,18446744073709551615],dtype=np.uint64),'accepted':[True,False],'label':['Ada π','研究']})
 t['distance'].unit='km';t['velocity'].unit='m / s'
 for name,delimiter in [('space',' '),('comma',',')]:t.write(a.output/f'{name}.ecsv',format='ascii.ecsv',delimiter=delimiter)
 unknown=Table({'flux':[1.,2.]});unknown['flux'].unit='Jy';unknown.write(a.output/'unsupported_unit.ecsv',format='ascii.ecsv')
 masked=Table();masked['sample']=MaskedColumn([1.,2.],mask=[False,True]);masked.write(a.output/'masked.ecsv',format='ascii.ecsv')
 mixed=Table({'time':Time(['2026-01-01','2026-01-02'])});mixed.write(a.output/'mixin.ecsv',format='ascii.ecsv')
 expect={'distance_si':[1250.,2500.],'velocity_si':[3.,4.],'source_id':['9007199254740993','18446744073709551615'],'accepted':[True,False],'label':['Ada π','研究']}
manifest={'schema':'goblin.upstream-reader-fixtures.v1','scope':'SYNTHETIC_FORMAT_INTEROPERABILITY_NOT_SCIENTIFIC_VALIDATION','producer_versions':versions,'expectations':expect,'files':{f.name:hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted(a.output.iterdir()) if f.is_file()}}
with (a.output/'REFERENCE.json').open('x') as f:json.dump(manifest,f,indent=2,ensure_ascii=False,allow_nan=False);f.write('\n')
print(json.dumps({'output':str(a.output),'producer_versions':versions,'fixtures':len(manifest['files'])}))
