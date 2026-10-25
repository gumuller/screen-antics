//! Continuous 60-Hz comparison recording and per-frame actor telemetry.
#[path="../src/animation.rs"] mod animation;
#[path="../src/resources.rs"] mod resources;
#[path="../src/remaster.rs"] mod remaster;
#[path="../src/scheduler.rs"] mod scheduler;
#[path="../src/hd2d_art.rs"] mod hd2d_art;
#[path="../src/hd2d_layout.rs"] mod hd2d_layout;
#[path="../src/hd2d_roles.rs"] mod hd2d_roles;
#[path="../src/hd2d_canvas.rs"] mod hd2d_canvas;
#[path="../src/hd2d_scene.rs"] mod hd2d_scene;
#[path="../src/hd2d.rs"] mod hd2d;
#[path="../src/hd2d_motion.rs"] mod hd2d_motion;
#[path="../src/hd2d_ending.rs"] mod hd2d_ending;
#[path="../src/hd2d_story.rs"] mod hd2d_story;
use std::{fs,io::{Write,BufWriter},path::Path,process::{Command,Stdio}};
fn main()->Result<(),String>{
 let root=Path::new(env!("CARGO_MANIFEST_DIR"));let args:Vec<_>=std::env::args().skip(1).collect();
 let out=root.join(args.first().map(String::as_str).unwrap_or("captures/motion-baseline"));fs::create_dir_all(&out).map_err(|e|e.to_string())?;
 let assets=resources::AssetStore::load(root.join("original"))?;
 let raw:Vec<_>=assets.raw.iter().filter(|(n,_)|n.ends_with(".TTM")).map(|(n,b)|animation::Clip::parse(n,b)).collect::<Result<_,_>>()?;
 let limit=args.iter().find_map(|a|a.strip_prefix("--limit=")).map(|s|s.parse::<u32>()).transpose().map_err(|e|e.to_string())?.unwrap_or(2400);
 let clips=if limit>=14400 {scheduler::build_hd2d_clips(&assets,&raw)?}else{scheduler::build_scene_clips_with_limit(&assets,&raw,limit)?};
 let video=!args.iter().any(|a|a=="--no-video");let effects=args.iter().any(|a|a=="--effects");
 let (w,h)=if video&&!args.iter().any(|a|a=="--small"){(800,450)}else{(480,270)};
 let mut renderer=hd2d_scene::Scene::load(&root.join("assets/hd2d-v2"),w,h)?;let mut fx=hd2d::Hd2dEffects::new();
 let mut summary=BufWriter::new(fs::File::create(out.join("summary.tsv")).map_err(|e|e.to_string())?);writeln!(summary,"scene\tticks\tsource_frames").unwrap();
 for clip in clips.iter().filter(|c|args.len()<2 || args[1..].iter().any(|a|c.name==*a||c.name==format!("{a}.TTM")) || args.iter().any(|a|a=="--all")) {
  let mut child=if video{Some(Command::new("ffmpeg").args(["-hide_banner","-loglevel","error","-y","-f","rawvideo","-pixel_format","rgb24","-video_size",&format!("{}x{h}",w*2),"-framerate","60","-i","-","-an","-c:v","libx264","-preset","fast","-crf","20","-pix_fmt","yuv420p"]).arg(out.join(format!("{}.mp4",clip.name))).stdin(Stdio::piped()).spawn().map_err(|e|e.to_string())?)}else{None};
  let mut input=child.as_mut().map(|c|BufWriter::new(c.stdin.take().unwrap()));
  let mut trace=BufWriter::new(fs::File::create(out.join(format!("{}.tsv",clip.name))).map_err(|e|e.to_string())?);
  writeln!(trace,"tick\tlayer\tbackground\tclip\tpose\tx\ty\twidth\theight\tflip").unwrap();
  let mut p=animation::Player::new();let mut pixels=vec![0u8;w*2*h*3];
  let mut tick=0;
  while let Some(delay)=p.step(clip,&assets){
   if args.iter().any(|a|a=="--audio-cues") && !p.sounds.is_empty() {println!("AUDIO {} tick={} sounds={:?} draws={:?}",clip.name,tick+1,p.sounds,p.draws.iter().map(|d|(&d.name,d.frame)).collect::<Vec<_>>());}

   for _ in 0..delay.max(1) {
   tick+=1;
   renderer.render(&p,&assets,&clip.name,remaster::AspectMode::Ratio16_9,tick as f64/60.)?;
   let bg=p.current_background.as_deref().unwrap_or("NONE");
   writeln!(trace,"{}\tstate\t{bg}\t-\t0\t0\t0\t0\t0\t0",tick).unwrap();
   for (layer,draws) in [("source",p.draws.clone()),("new",renderer.mask.sources.iter().filter_map(|s|if let animation::PixelSource::Sprite(d)=s{Some(d.clone())}else{None}).collect())] {
    for d in draws {writeln!(trace,"{}\t{layer}\t{bg}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",tick,d.name,d.frame,d.x,d.y,d.width,d.height,if d.flip_x{1}else{0}).unwrap();}
   }
   for sample in &renderer.motion_samples {writeln!(trace,"{}\troot-{}\t{bg}\t{}\t{}\t{:.4}\t{:.4}\t0\t0\t{}",tick,sample.key,sample.clip,sample.phase,sample.point.x,sample.point.y,if sample.flip{1}else{0}).unwrap();}
   if let Some(input)=&mut input {
    let l=renderer.layout;fx.set_world(hd2d::WorldLighting{horizon:l.horizon,reflection_x:if l.night{320.}else{200.},night:l.night,outdoors:matches!(l.kind,hd2d_layout::SceneKind::Island|hd2d_layout::SceneKind::Underwater|hd2d_layout::SceneKind::Beach)});
    let new=if effects{fx.render(&renderer.canvas.pixels,w,h,renderer.canvas.stage,&renderer.mask,&renderer.fx_art,tick as f64/60.)}else{&renderer.canvas.pixels};
    let scale=(w as f64/640.).min(h as f64/p.height as f64);let ox=(w as f64-640.*scale)*0.5;let oy=(h as f64-p.height as f64*scale)*0.5;
    pixels.fill(0);
    for y in 0..h {for x in 0..w {
     let sx=(x as f64+0.5-ox)/scale;let sy=(y as f64+0.5-oy)/scale;
     if sx>=0.&&sx<640.&&sy>=0.&&sy<p.height as f64 {let c=assets.palette[p.pixels[sy as usize*640+sx as usize]as usize];let i=(y*w*2+x)*3;pixels[i..i+3].copy_from_slice(&c);}
     let c=new[y*w+x];let i=(y*w*2+w+x)*3;pixels[i..i+3].copy_from_slice(&[(c>>16)as u8,(c>>8)as u8,c as u8]);
    }}
    input.write_all(&pixels).map_err(|e|e.to_string())?;
   }
  }
  }
  drop(input);if let Some(mut child)=child {if !child.wait().map_err(|e|e.to_string())?.success(){return Err("Video encoder failed".into());}}
  writeln!(summary,"{}\t{}\t{}",clip.name,tick,p.frame).unwrap();println!("{}: {} simulated ticks ({} source frames)",clip.name,tick,p.frame);
 }
 Ok(())
}
