exec(open('/tmp/ocenaudio-bench/common.py').read())
import re,sys,random,statistics

def temp():
 try:return int(pathlib.Path("/sys/class/thermal/thermal_zone7/temp").read_text())/1000
 except OSError:return None
base=pathlib.Path('/tmp/argand-67')
source=pathlib.Path('/home/xokc/dp/argand/tests/signals/m39-repeat-1GB.wav')
def q(v,p):return sorted(v)[int((len(v)-1)*p)] if v else None
def run(label,workers=8,affinity='none',mask=None,round=0):
 case=base/f'final-ui-{label}-r{round}';case.mkdir(exist_ok=True)
 for name in ['state/argand','config/argand']:(case/name).mkdir(parents=True,exist_ok=True)
 (case/'config/argand/argand.toml').write_text(f'theme = "dark"\n[analysis]\nworkers = {workers}\nbatch_frames = 1024\naffinity = "{affinity}"\n')
 if label=='baseline':(case/'config/argand/argand.toml').write_text('theme = "dark"\n')
 (case/'state/argand/session.toml').write_text('version = 3\nwindow_state = "normal"\n[geometry]\nx = 0.0\ny = 0.0\nwidth = 1280.0\nheight = 800.0\n')
 childenv=dict(env,XDG_STATE_HOME=str(case/'state'),XDG_CONFIG_HOME=str(case/'config'),RUST_LOG='argand=debug,argand::ui_latency=trace,argand_dsp=debug,gpui=error',NO_COLOR='1')
 if mask:childenv.update(LD_PRELOAD=str(base/'compute-affinity.so'),BENCH_COMPUTE_CPUS=mask)
 with source.open('rb') as f:
  while f.read(8<<20):pass
 binary=base/'baseline-target/release/argand' if label=='baseline' else base/'target/release/argand'
 with (case/'client.log').open('w') as log:
  started=time.monotonic();app=subprocess.Popen([str(binary),str(source)],env=childenv,stdout=log,stderr=log)
  masks={};lag=[];complete_at=None;tick=0;temps=[temp()];cpu_seconds=None;busy_probe=[]
  while time.monotonic()-started<90 and app.poll() is None:
   due=time.monotonic()+.02;time.sleep(.02);lag.append(max(0,time.monotonic()-due)*1000);tick+=1
   if tick%5:continue
   data=(case/'client.log').read_text()
   if not masks:
    try:
     for t in pathlib.Path(f'/proc/{app.pid}/task').iterdir():
      comm=(t/'comm').read_text().strip()
      if t.name==str(app.pid) or comm.startswith('argand-fft-'):
       masks[comm]=re.search(r'Cpus_allowed_list:\s*([^\n]+)',(t/'status').read_text())[1]
     if len(masks)<workers+1:masks={}
    except OSError:pass
   if tick%50==0:temps.append(temp())
   if 'analysis completed' in data and complete_at is None:
    complete_at=time.monotonic();busy_probe=lag.copy()
    try:
     stat=pathlib.Path(f'/proc/{app.pid}/stat').read_text().split(') ',1)[1].split()
     cpu_seconds=(int(stat[11])+int(stat[12]))/os.sysconf('SC_CLK_TCK')
    except OSError:pass
   if complete_at and time.monotonic()-complete_at>=3:break
  data=re.sub(r'\x1b\[[0-9;]*m','',(case/'client.log').read_text())
  (case/'clean.log').write_text(data)
  before,sep,after=data.partition('analysis completed')
  timer=lambda d:[int(x)/1000 for x in re.findall(r'late_us=(\d+)',d)]
  uploads=[int(x)/1000 for x in re.findall(r'texture prepared elapsed_us=(\d+)',before)]
  ages=[int(x)/1000 for x in re.findall(r'analysis delivery received age_us=(\d+)',before)]
  busy=timer(before);idle=timer(after)
  result=dict(label=label,round=round,temps=temps,cpu_seconds=cpu_seconds,busy_probe_p99_ms=q(busy_probe,.99),completed=bool(sep),masks=masks,busy_samples=len(busy),ui_p95_ms=q(busy,.95),ui_p99_ms=q(busy,.99),ui_max_ms=max(busy,default=0),idle_p99_ms=q(idle,.99),upload_p99_ms=q(uploads,.99),delivery_p99_ms=q(ages,.99),probe_p99_ms=q(lag,.99),events=[l for l in data.splitlines() if any(k in l for k in ['analysis completed','first picture painted','starting analysis pool','file opened'])])
  (case/'result.json').write_text(json.dumps(result,indent=2));print(json.dumps(result),flush=True)
  with (base/'native-final.jsonl').open('a') as f:f.write(json.dumps(result)+'\n')
  if round==0:subprocess.run(['grim',str(case/'final.png')],env=childenv,check=True)
  if app.poll() is None:
   for node in nodes(ipc('',4)):
    if node.get('app_id')=='io.github.o_kos.argand':ipc(f'[con_id={node["id"]}] kill')
   app.wait(timeout=10)
  if not sep:raise RuntimeError('analysis failed: '+str(case))
if __name__=='__main__':
 if len(sys.argv)>1 and sys.argv[1]=='smoke':
  run('optimized',round=-1);run('baseline',round=-1)
 else:
  configs=[('baseline',8,'none',None),('optimized',8,'none',None),('any4',4,'none',None),('any6',6,'none',None),('e4',4,'none','4,5,6,7'),('e6',6,'none','4,5,6,7,8,9'),('efficiency',8,'efficiency',None),('mixed10',10,'none','0,2,4,5,6,7,8,9,10,11')]
  for r in range(3):
   order=configs.copy();random.Random(710+r).shuffle(order)
   for label,w,a,m in order:run(label,w,a,m,r)
