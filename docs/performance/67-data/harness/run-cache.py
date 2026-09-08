import sys,os,importlib.util,shutil
from pathlib import Path
sys.argv=['run-cache','cache']
spec=importlib.util.spec_from_file_location('matrix','/tmp/argand-67/run-matrix.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
m.binary=m.root/'baseline-bench'
with m.source.open('rb') as src,(m.root/'cold-copy.wav').open('wb') as dst:
 shutil.copyfileobj(src,dst,8<<20);dst.flush();os.fsync(dst.fileno())
for r in range(3):
 pairs=[('warm','mapped'),('cold','mapped'),('warm','memory')]
 pairs=pairs[r:]+pairs[:r]
 for cache,kind in pairs:m.run(f'{kind}-{cache}',8,'any',mode=kind,cache=cache,round=r)
