use argand_core::{SampleRange, Colormap};
use argand_dsp::*;
use std::{time::Instant, hint::black_box};
fn main() {
 for workers in [1,8] {
 let pool=rayon::ThreadPoolBuilder::new().num_threads(workers).build().unwrap();
 pool.install(|| {
 for path in ["m39.wav", "iq_i16-hfdl.iqw"] {
 let path=std::path::Path::new("/home/xokc/dp/argand/tests/signals").join(path);
 let mut source=argand_io::open(&path,&Default::default()).unwrap();
 for frames in [1,16,128,512,2048] {
 let n=2048+512*(frames-1);
 if n>source.meta().len_samples {continue;}
 let request=AnalysisRequest{cfg:StftConfig::new(2048,Window::Hann),range:SampleRange::new(0,n),width:1500,height:700,reduce:Reduce::Max,colormap:Colormap::Oceanic,dynamic_range:DynamicRange::Default,waveform_columns:Some(1500)};
 for publish in [false,true] {
 let mut times=vec![]; let mut renders=0;
 for _ in 0..5 {
 let start=Instant::now();
 let mut state=analyze_overview(source.as_mut(),&request,ProgressiveOptions::default().final_only(),&||Flow::Continue,&mut |s,_|{if publish {black_box(s.render(1500,700,Some(1500)).unwrap());renders+=1;} Flow::Continue}).unwrap();
 black_box(state.render(if frames == 1 {1} else {1500},700,Some(1500)).unwrap());times.push(start.elapsed().as_secs_f64()*1000.);
 }
 times.sort_by(f64::total_cmp);
 println!("{} workers={workers} frames={frames} publish={publish} median_ms={:.3} snapshots={renders}",path.file_name().unwrap().to_string_lossy(),times[2]);
 }
 }
 }
 });
 }
}
