import sys,random,importlib.util
sys.argv=['run-compact','compact']
spec=importlib.util.spec_from_file_location('matrix','/tmp/argand-67/run-matrix.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
configs=[('baseline256','baseline-bench',256,8),('compact256','compact-only-bench',256,8),('compact1024','compact-only-bench',1024,32),('compact2048','compact-only-bench',2048,64)]
for r in range(3):
 random.Random(700+r).shuffle(configs)
 for label,binary,b,g in configs:
  m.binary=m.root/binary;m.run(label,8,'any',batch=b,grain=g,round=r)
