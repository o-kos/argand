exec(open('/tmp/ocenaudio-bench/common.py').read())
import sys,re,statistics,ctypes,mmap
base=pathlib.Path('/tmp/argand-74')
source=pathlib.Path('/home/xokc/dp/argand/tests/signals/m39-repeat-1GB.wav')
ansi=re.compile(r'\x1b\[[0-9;]*m')
def clean(p):return ansi.sub('',p.read_text())
def quant(v,p):return sorted(v)[int((len(v)-1)*p)] if v else None
def temp():
 try:return int(pathlib.Path('/sys/class/thermal/thermal_zone7/temp').read_text())/1000
 except OSError:return None
def proc(app):
 stat=pathlib.Path(f'/proc/{app.pid}/stat').read_text().split(') ',1)[1].split()
 status=pathlib.Path(f'/proc/{app.pid}/status').read_text()
 return dict(cpu_s=(int(stat[11])+int(stat[12]))/os.sysconf('SC_CLK_TCK'),rss_kib=int(re.search(r'VmRSS:\s*(\d+)',status)[1]),peak_kib=int(re.search(r'VmHWM:\s*(\d+)',status)[1]))
def durations(data,tag):
 return [float(n)*({'s':1,'ms':.001,'µs':.000001,'us':.000001,'ns':1e-9}[unit]) for n,unit in re.findall(re.escape(tag)+r' elapsed=([0-9.]+)(ms|µs|us|ns|s)',data)]
def residency(path):
 with path.open('rb') as f:
  m=mmap.mmap(f.fileno(),0,access=mmap.ACCESS_COPY)
  n=(len(m)+4095)//4096;v=(ctypes.c_ubyte*n)()
  libc=ctypes.CDLL(None,use_errno=True)
  rc=libc.mincore(ctypes.c_void_p(ctypes.addressof(ctypes.c_char.from_buffer(m))),ctypes.c_size_t(len(m)),v)
  if rc:raise OSError(ctypes.get_errno(),'mincore')
  resident=sum(x&1 for x in v);m.close()
 return dict(pages=n,resident=resident)
def run(label,round,mode='max',during=False,cold=False):
 case=base/f'{label}-{mode}-r{round}{"-during" if during else ""}{"-cold" if cold else ""}'
 case.mkdir(exist_ok=True)
 for name in ['state/argand','config/argand']:(case/name).mkdir(parents=True,exist_ok=True)
 (case/'config/argand/argand.toml').write_text(f'theme="dark"\naggregation="{mode}"\n[analysis]\nworkers=8\nbatch_frames=1024\naffinity="none"\n')
 (case/'state/argand/session.toml').write_text('version=3\nwindow_state="normal"\n[geometry]\nx=0.0\ny=0.0\nwidth=1280.0\nheight=800.0\n')
 path=base/'cold-copy.wav' if cold else source
 if cold:
  with path.open('rb') as f:os.posix_fadvise(f.fileno(),0,0,os.POSIX_FADV_DONTNEED)
 else:
  with path.open('rb') as f:
   while f.read(8<<20):pass
 cache_before=residency(path)
 childenv=dict(env,XDG_CONFIG_HOME=str(case/'config'),XDG_STATE_HOME=str(case/'state'),RUST_LOG='argand=debug,argand::ui_latency=trace,argand_dsp=debug,gpui=error',NO_COLOR='1')
 binary=base/'baseline-argand' if label=='baseline' else pathlib.Path('/home/xokc/dp/argand/target/release/argand')
 logpath=case/'app.log';app=subprocess.Popen([str(binary),str(path)],env=childenv,stdout=logpath.open('w'),stderr=subprocess.STDOUT)
 start=time.monotonic();temps=[temp()];rss=[];wid=None
 def wait(predicate,timeout=90):
  until=time.monotonic()+timeout
  while time.monotonic()<until:
   if app.poll() is not None:raise RuntimeError('app exited: '+str(case))
   d=clean(logpath)
   if predicate(d):return d
   time.sleep(.005)
  raise RuntimeError('timeout: '+str(case))
 try:
  if during:
   wait(lambda d:'analysis snapshot' in d)
   wid=next(n['id'] for n in nodes(ipc('',4)) if n.get('pid')==app.pid)
   for w,h in [(1180,760),(1080,700),(1400,900),(1280,800)]*4:
    ipc(f'[con_id={wid}] resize set width {w} px height {h} px');time.sleep(.035)
  initial=wait(lambda d:'analysis completed' in d)
  initial_seen=time.monotonic()-start
  metrics=proc(app);temps.append(temp());rss.append(metrics['rss_kib'])
  wid=next(n['id'] for n in nodes(ipc('',4)) if n.get('pid')==app.pid)
  time.sleep(.25)
  results=[]
  if not during and not cold:
   for name,w,h in [('height',1280,700),('width',1080,700),('return',1280,800),('larger',1680,950),('restore',1280,800)]:
    before=clean(logpath);metrics_before=proc(app);begin=time.monotonic()
    tag='analysis completed' if label=='baseline' else 'cached view prepared'
    count=before.count(tag)
    ipc(f'[con_id={wid}] resize set width {w} px height {h} px')
    data=wait(lambda d:d.count(tag)>count and 'texture prepared' in d[d.rfind(tag):])
    result=dict(name=name,requested=[w,h],observed_upload_s=time.monotonic()-begin,cpu_s=proc(app)['cpu_s']-metrics_before['cpu_s'],cache_s=durations(data[len(before):],'cached view prepared'),analysis_s=durations(data[len(before):],'analysis completed'))
    results.append(result);rss.append(proc(app)['rss_kib']);temps.append(temp());time.sleep(.25)
  data=clean(logpath)
  busy=data.split('analysis completed')[0]
  lag=[int(x)/1000 for x in re.findall(r'late_us=(\d+)',busy)]
  after=data[len(initial):]
  resize_lag=[int(x)/1000 for x in re.findall(r'late_us=(\d+)',after)]
  result=dict(label=label,round=round,mode=mode,during=during,cold=cold,cache_before=cache_before,initial_seen_s=initial_seen,initial_analysis_s=durations(initial,'analysis completed'),initial=metrics,temps=temps,rss_kib=rss,ui_initial_p99_ms=quant(lag,.99),ui_resize_p99_ms=quant(resize_lag,.99),analysis_passes=data.count('analysis completed'),resizes=results)
  (case/'result.json').write_text(json.dumps(result,indent=2));(case/'clean.log').write_text(data)
  with (base/'native.jsonl').open('a') as f:f.write(json.dumps(result)+'\n')
  if round==0:subprocess.run(['grim',str(case/'final.png')],env=childenv,check=True)
  print(json.dumps(result),flush=True)
 finally:
  if app.poll() is None:
   if wid:ipc(f'[con_id={wid}] kill')
   else:app.terminate()
   app.wait(timeout=15)
if __name__=='__main__':
 ipc('output HEADLESS-1 mode 1920x1080')
 ipc('for_window [app_id="io.github.o_kos.argand"] floating enable')
 if sys.argv[1]=='smoke':run('current',-1)
 elif sys.argv[1]=='matrix':
  for r in range(3):
   for label in (['baseline','current'] if r%2==0 else ['current','baseline']):run(label,r)
  run('current',0,'mean-power');run('current',0,during=True)
 elif sys.argv[1]=='cold':
  for r in range(3):
   for label in (['baseline','current'] if r%2==0 else ['current','baseline']):run(label,r,cold=True)
