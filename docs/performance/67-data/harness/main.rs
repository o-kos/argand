use std::time::Instant;
use argand_core::{AccessPattern, Colormap, SampleRange, SampleSource, SignalMeta, SourceError};
use argand_dsp::{AnalysisRequest, DynamicRange, Flow, Reduce, StftConfig, Window, analyze_progressive};
struct Repeated { meta: SignalMeta, data: Vec<f32>, pos: u64 }
impl SampleSource for Repeated {
 fn meta(&self)->&SignalMeta { &self.meta }
 fn seek(&mut self,n:u64)->Result<(),SourceError>{self.pos=n;Ok(())}
 fn access_pattern(&mut self,_:AccessPattern){}
 fn read(&mut self,out:&mut[f32])->Result<usize,SourceError>{
  let n=out.len().min((self.meta.len_samples-self.pos) as usize);let mut done=0;
  while done<n {let start=self.pos as usize%self.data.len();let take=(n-done).min(self.data.len()-start);out[done..done+take].copy_from_slice(&self.data[start..start+take]);done+=take;self.pos+=take as u64;}
  Ok(n)
 }
}
fn main()->anyhow::Result<()> {
 let args:Vec<String>=std::env::args().collect();let workers:usize=args[1].parse()?;
 let cpus:Vec<usize>=if args[2]=="any"{vec![]}else{args[2].split(',').map(str::parse).collect::<Result<_,_>>()?};
 let affinity=std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));let errors=affinity.clone();
 let pool=rayon::ThreadPoolBuilder::new().num_threads(workers).start_handler(move |_|{
  if !cpus.is_empty(){unsafe {let mut mask=std::mem::zeroed::<libc::cpu_set_t>();libc::CPU_ZERO(&mut mask);for &cpu in &cpus{libc::CPU_SET(cpu,&mut mask);}if libc::sched_setaffinity(0,std::mem::size_of_val(&mask),&mask)!=0{errors.fetch_add(1,std::sync::atomic::Ordering::Relaxed);}}}
 }).build()?;
 let mut hints=argand_io::OpenHints::default();hints.level_scan_bytes=Some(64<<20);
 let path=std::env::var("BENCH_FILE").unwrap_or_else(|_|"/home/xokc/dp/argand/tests/signals/m39-repeat-1GB.wav".to_owned());
 let mut source=argand_io::open(std::path::Path::new(&path),&hints)?;
 if args[3]=="memory" {
  let mut original=argand_io::open(std::path::Path::new("/home/xokc/dp/argand/tests/signals/m39.wav"),&hints)?;
  let mut data=vec![0.;original.meta().len_samples as usize];let mut done=0;while done<data.len(){let n=original.read(&mut data[done..])?;anyhow::ensure!(n>0,"short source");done+=n;}
  source=Box::new(Repeated{meta:source.meta().clone(),data,pos:0});
 }
 let request=AnalysisRequest{cfg:StftConfig::new(2048,Window::Hann),range:SampleRange::new(0,source.meta().len_samples),width:1214,height:662,reduce:Reduce::Max,colormap:Colormap::Oceanic,dynamic_range:DynamicRange::Default,waveform_columns:Some(1214)};
 let start=Instant::now();let mut first=None;let mut updates=0;
 let result=pool.install(||analyze_progressive(source.as_mut(),&request,&||Flow::Continue,&mut|_,_|{first.get_or_insert(start.elapsed().as_secs_f64());updates+=1;Flow::Continue}))?;
 let seconds=start.elapsed().as_secs_f64();
 let mut hash=0xcbf29ce484222325u64;for v in &result.db.values{for b in v.to_bits().to_le_bytes(){hash=(hash^u64::from(b)).wrapping_mul(0x100000001b3);}}
 println!("{}",serde_json::json!({"workers":workers,"placement":args[2],"mode":args[3],"seconds":seconds,"first":first,"updates":updates,"frames":result.frames,"db_hash":format!("{hash:016x}"),"affinity_errors":affinity.load(std::sync::atomic::Ordering::Relaxed)}));
 Ok(())
}
