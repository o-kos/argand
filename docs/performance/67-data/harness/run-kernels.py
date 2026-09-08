import sys,random
sys.argv=['run-kernels','kernels']
import importlib.util
spec=importlib.util.spec_from_file_location("matrix", "/tmp/argand-67/run-matrix.py")
m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
configs=[('baseline','baseline-bench'),('ranges','ranges-bench'),('compressed','compressed-bench')]
for r in range(3):
 random.Random(690+r).shuffle(configs)
 for label,binary in configs:
  m.binary=m.root/binary
  m.run(label,8,'any',round=r)
