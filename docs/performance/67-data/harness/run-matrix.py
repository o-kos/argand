import os,time,json,subprocess,resource,statistics,random,sys,mmap,ctypes,math
from pathlib import Path
root=Path('/tmp/argand-67');binary=root/'bench-workspace/target/release/fft-scheduling-bench'
source=Path('/home/xokc/dp/argand/tests/signals/m39-repeat-1GB.wav')
libc=ctypes.CDLL(None);libc.mincore.argtypes=[ctypes.c_void_p,ctypes.c_size_t,ctypes.c_void_p]
def residency(path):
 with path.open('rb') as f:
  m=mmap.mmap(f.fileno(),0,access=mmap.ACCESS_COPY);n=(len(m)+4095)//4096;v=(ctypes.c_ubyte*n)();c=ctypes.c_char.from_buffer(m);rc=libc.mincore(ctypes.addressof(c),len(m),v);del c;m.close()
  return sum(x&1 for x in v)/n if rc==0 else None
def temp():
 try:return int(Path('/sys/class/thermal/thermal_zone7/temp').read_text())/1000
 except OSError:return None
def quantile(v,p):return sorted(v)[min(len(v)-1,int((len(v)-1)*p))] if v else None
def run(label,workers,mask,batch=256,grain=8,mode='mapped',cache='warm',round=0):
 path=source if cache!='cold' else root/'cold-copy.wav'
 if mode=='mapped':
  with path.open('rb') as f:
   if cache=='cold':os.posix_fadvise(f.fileno(),0,0,os.POSIX_FADV_DONTNEED)
   else:
    while f.read(8<<20):pass
 before_cache=residency(path) if mode=='mapped' else None
 before=resource.getrusage(resource.RUSAGE_CHILDREN);t0=time.monotonic();temps=[temp()];lags=[];rss=0
 env=dict(os.environ,BENCH_BATCH=str(batch),BENCH_GRAIN=str(grain),BENCH_FILE=str(path))
 output=root/f'{label}-r{round}.out'
 with output.open('w') as out:
  p=subprocess.Popen([str(binary),str(workers),mask,mode],stdout=out,stderr=out,env=env)
  next_temp=t0+1
  while p.poll() is None:
   due=time.monotonic()+.02;time.sleep(.02);lags.append(max(0,time.monotonic()-due)*1000)
   if time.monotonic()>next_temp:
    temps.append(temp());next_temp+=1
    try:
     lines=Path(f'/proc/{p.pid}/status').read_text().splitlines();rss=max(rss,int(next(l.split()[1] for l in lines if l.startswith('VmRSS:'))))
    except (OSError,StopIteration):pass
 after=resource.getrusage(resource.RUSAGE_CHILDREN)
 if p.returncode:raise RuntimeError(output.read_text())
 lines=output.read_text().splitlines();record=json.loads(next(l for l in lines if l.startswith('{')))
 stage=next((l for l in lines if l.startswith('STAGES ')),None)
 record.update(label=label,round=round,batch=batch,grain=grain,cache=cache,resident_before=before_cache,temp_before=temps[0],temp_max=max(t for t in temps if t is not None),temp_after=temp(),cpu_seconds=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime,system_seconds=after.ru_stime-before.ru_stime,minor_faults=after.ru_minflt-before.ru_minflt,major_faults=after.ru_majflt-before.ru_majflt,voluntary_switches=after.ru_nvcsw-before.ru_nvcsw,involuntary_switches=after.ru_nivcsw-before.ru_nivcsw,peak_rss_kib=rss,probe_p95_ms=quantile(lags,.95),probe_p99_ms=quantile(lags,.99),probe_max_ms=max(lags),stages=stage)
 with (root/(sys.argv[1]+'.jsonl')).open('a') as f:f.write(json.dumps(record)+'\n')
 print(json.dumps({k:record[k] for k in ['label','round','seconds','cpu_seconds','temp_before','temp_max','probe_p99_ms','db_hash','affinity_errors']}),flush=True)
 return record
if __name__=='__main__':
 mode=sys.argv[1]
 if mode=='placement':
  configs=[('any4',4,'any'),('any6',6,'any'),('any8',8,'any'),('any10',10,'any'),('any12',12,'any'),('e4',4,'4,5,6,7'),('e6',6,'4,5,6,7,8,9'),('e8',8,'4,5,6,7,8,9,10,11'),('mix6',6,'0,2,4,5,6,7'),('mix10',10,'0,2,4,5,6,7,8,9,10,11')]
  for i in range(2):run('warmup',8,'any',round=i)
  for r in range(3):
   order=configs.copy();random.Random(670+r).shuffle(order)
   for label,w,m in order:run(label,w,m,round=r)
 elif mode=='batches':
  configs=[(256,8),(512,8),(1024,8),(2048,8),(1024,32),(1024,128),(1024,256),(1024,1024)]
  for r in range(3):
   random.Random(680+r).shuffle(configs)
   for b,g in configs:run(f'b{b}-g{g}',8,'any',batch=b,grain=g,round=r)
 elif mode=='cache':
  for r in range(3):
   for cache,kind in [('warm','mapped'),('cold','mapped'),('warm','memory')]:run(f'{kind}-{cache}',8,'any',mode=kind,cache=cache,round=r)
