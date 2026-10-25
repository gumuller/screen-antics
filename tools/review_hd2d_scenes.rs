//! Exercise the actual standalone HD-2D renderer through every recovered scene.
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
use std::{fs,io::Write,path::Path};
fn main()->Result<(),String>{
 let root=Path::new(env!("CARGO_MANIFEST_DIR"));let out=root.join("captures/hd2d-story-final");fs::create_dir_all(&out).map_err(|e|e.to_string())?;
 let assets=resources::AssetStore::load(root.join("original"))?;
 let raw:Vec<_>=assets.raw.iter().filter(|(n,_)|n.ends_with(".TTM")).map(|(n,b)|animation::Clip::parse(n,b)).collect::<Result<_,_>>()?;
 let clips=scheduler::build_hd2d_clips(&assets,&raw)?;
 let mut scene=hd2d_scene::Scene::load(&root.join("assets/hd2d-v2"),960,405)?;
 let mut fx=hd2d::Hd2dEffects::new();let mut report=String::from("scene\tframes\tcaptures\n");
 for clip in clips {
 if std::env::args().nth(1).is_some_and(|name|!clip.name.starts_with(&name)){continue;}
  for file in fs::read_dir(&out).map_err(|e|e.to_string())? { let path=file.map_err(|e|e.to_string())?.path();if path.file_name().unwrap().to_string_lossy().starts_with(&format!("{}-",clip.name)){fs::remove_file(path).map_err(|e|e.to_string())?;} }
 let mut p=animation::Player::new();let mut count=0;let mut seen=std::collections::BTreeSet::new();
  while p.step(&clip,&assets).is_some(){
   scene.render(&p,&assets,&clip.name,remaster::AspectMode::Ratio43_18,p.frame as f64/60.)?;
   let names:std::collections::BTreeSet<_>=scene.mask.sources.iter().filter_map(|s|if let animation::PixelSource::Sprite(d)=s {Some(d.name.clone())}else{None}).collect();
   let beat=hd2d_story::frame(&clip.name,p.frame).map(|s|s.beat).unwrap_or("ending");
   let key=format!("{beat}: {}",names.into_iter().collect::<Vec<_>>().join(","));let new_pose=seen.insert(key.clone());
   if (new_pose && count<55) || p.frame==1 || p.frame==clip.frames.len() || p.frame%((clip.frames.len()/10).max(1))==0 {
    fs::write(out.join(format!("{}-{:05}.txt",clip.name,p.frame)),format!("visible {key}\ndraws {:#?}\naudit {:#?}",p.draws,scene.audit)).map_err(|e|e.to_string())?;
    let l=scene.layout;fx.set_world(hd2d::WorldLighting{horizon:l.horizon,reflection_x:if l.night{320.}else{200.},night:l.night,outdoors:matches!(l.kind,hd2d_layout::SceneKind::Island|hd2d_layout::SceneKind::Underwater)});
    let pixels=fx.render(&scene.canvas.pixels,960,405,scene.canvas.stage,&scene.mask,&scene.fx_art,p.frame as f64/60.);
    let mut f=fs::File::create(out.join(format!("{}-{:05}.ppm",clip.name,p.frame))).map_err(|e|e.to_string())?;write!(f,"P6\n960 405\n255\n").map_err(|e|e.to_string())?;
    f.write_all(&pixels.iter().flat_map(|p|[(p>>16) as u8,(p>>8)as u8,*p as u8]).collect::<Vec<_>>()).map_err(|e|e.to_string())?;count+=1;
   }
  }
  let row=format!("{}\t{}\t{count}\n",clip.name,p.frame);print!("{row}");report.push_str(&row);
 }

 let backgrounds:Vec<_>=scene.art.backgrounds.keys().cloned().collect();
 for background in backgrounds {
  for (mode,aspect) in [("core",remaster::AspectMode::Original),("16_9",remaster::AspectMode::Ratio16_9),("21_9",remaster::AspectMode::Ratio21_9),("43_18",remaster::AspectMode::Ratio43_18)] {
   let mut p=animation::Player::new();p.current_background=Some(background.clone());
   let layout=hd2d_layout::Layout::for_background(&background);
   let family=match layout.kind {hd2d_layout::SceneKind::Island=>Some(("MJREAD.BMP",5,120,250)),hd2d_layout::SceneKind::Underwater=>Some(("GJDIVE.BMP",0,290,200)),hd2d_layout::SceneKind::Office=>Some(("SJWORK.BMP",0,300,250)),hd2d_layout::SceneKind::Beach=>Some(("SSUZY1.BMP",0,230,285)),_=>None};
   if let Some((name,index,x,y))=family {let im=&assets.sprites[name][index];p.draws.push(animation::Draw{name:name.into(),frame:index,x,y,width:im.width,height:im.height,flip_x:false,flip_y:false,clip:[0,0,640,480],pose_signature:0});}
   scene.render(&p,&assets,&format!("layout-{background}"),aspect,5.)?;
   let mut f=fs::File::create(out.join(format!("layout-{background}-{mode}.ppm"))).map_err(|e|e.to_string())?;write!(f,"P6\n960 405\n255\n").map_err(|e|e.to_string())?;
   f.write_all(&scene.canvas.pixels.iter().flat_map(|p|[(p>>16)as u8,(p>>8)as u8,*p as u8]).collect::<Vec<_>>()).map_err(|e|e.to_string())?;
  }
 }
 fs::write(out.join(if std::env::args().nth(1).is_some(){"targeted-scene-review.tsv"}else{"scene-review.tsv"}),report).map_err(|e|e.to_string())?;
 let audit=scene.audit.iter().map(|(k,v)|format!("{k}\t{}\n",v.iter().cloned().collect::<Vec<_>>().join(","))).collect::<String>();fs::write(out.join(if std::env::args().nth(1).is_some(){"targeted-sprite-bindings.tsv"}else{"sprite-bindings.tsv"}),audit).map_err(|e|e.to_string())?;
 Ok(())
}
