exec(open('/tmp/ocenaudio-bench/common.py').read())
import re
base=pathlib.Path('/tmp/argand-74')
def pointer(commands):
 data=''
 for cmd,x,y in commands:
  if cmd=='M':x,y=round(x*1600/1920),round(y*1000/1080)
  data+=f'{cmd} {x} {y}\n'
 subprocess.run(['/tmp/argand-start-page/pointer'],input=data,text=True,env=env,check=True,stdout=subprocess.DEVNULL)
def run(name,path,mode):
 case=base/f'visual-{name}-{mode}';case.mkdir(exist_ok=True)
 for d in ['config/argand','state/argand']:(case/d).mkdir(parents=True,exist_ok=True)
 (case/'config/argand/argand.toml').write_text(f'theme="dark"\naggregation="{mode}"\n[analysis]\nworkers=8\n')
 (case/'state/argand/session.toml').write_text('version=3\nwindow_state="normal"\n[geometry]\nx=0.0\ny=0.0\nwidth=1280.0\nheight=800.0\n')
 childenv=dict(env,XDG_CONFIG_HOME=str(case/'config'),XDG_STATE_HOME=str(case/'state'),RUST_LOG='argand=debug,argand_dsp=debug,gpui=error',NO_COLOR='1')
 logpath=case/'app.log'
 app=subprocess.Popen(['/home/xokc/dp/argand/target/release/argand',path],env=childenv,stdout=logpath.open('w'),stderr=subprocess.STDOUT)
 wid=None
 try:
  start=time.monotonic()
  while 'analysis completed' not in logpath.read_text():
   if app.poll() is not None or time.monotonic()-start>90:raise RuntimeError('open failed')
   time.sleep(.01)
  time.sleep(.2)
  win=next(n for n in nodes(ipc('',4)) if n.get('pid')==app.pid);wid=win['id'];rect=win['rect']
  gpu=sorted(set(os.readlink(p) for p in pathlib.Path(f'/proc/{app.pid}/fd').iterdir() if p.exists() and '/dev/dri/' in os.readlink(p)))
  subprocess.run(['grim',str(case/'before.png')],env=childenv,check=True)
  x=rect['x']+640;y=rect['y']
  before=logpath.read_text().count('cached view prepared')
  pointer([('M',x,y+82),('B',1,0),('S',100,0),('M',x,y+150),('S',100,0),('M',x,y+230),('S',100,0),('B',0,0),('S',300,0)])
  data=logpath.read_text()
  result=dict(name=name,mode=mode,gpu=gpu,analysis_passes=data.count('analysis completed'),new_cached_views=data.count('cached view prepared')-before)
  subprocess.run(['grim',str(case/'after.png')],env=childenv,check=True)
  (case/'result.json').write_text(json.dumps(result,indent=2));print(json.dumps(result),flush=True)
  assert result['analysis_passes']==1 and result['new_cached_views']>=1
 finally:
  if app.poll() is None:
   if wid:ipc(f'[con_id={wid}] kill')
   else:app.terminate()
   app.wait(timeout=15)
for mode in ['max','mean-power']:
 run('real','/home/xokc/dp/argand/tests/signals/m39.wav',mode)
 run('iq','/home/xokc/dp/argand/tests/signals/iq_i16-hfdl.iqw',mode)
