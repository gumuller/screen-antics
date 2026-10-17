//! Exhaustive indexed playback plus evenly-spaced true-color review captures.
#[path="../src/resources.rs"] mod resources;
#[path="../src/animation.rs"] mod animation;
#[path="../src/scheduler.rs"] mod scheduler;
#[path="../src/presentation.rs"] mod presentation;
#[path="../src/remaster.rs"] mod remaster;
#[path="../src/hd.rs"] mod hd;
use std::{fs,io::Write,path::Path};
fn main() -> Result<(),String> {
    let root=Path::new(env!("CARGO_MANIFEST_DIR"));
    let out=root.join("captures/review"); fs::create_dir_all(&out).map_err(|e|e.to_string())?;
    let assets=resources::AssetStore::load(root.join("original"))?;
    let art=remaster::RemasterAssets::load_with_painted_art(&root.join("assets/remastered"),&assets,false)?;
    let raw:Vec<_>=assets.raw.iter().filter(|(n,_)|n.ends_with(".TTM")).map(|(n,b)|animation::Clip::parse(n,b)).collect::<Result<_,_>>()?;
    let clips=scheduler::build_scene_clips(&assets,&raw)?;
    let mut log=String::from("scene\tframes\tsprite_draws\tcaptures\n");
    for clip in clips {
        let mut player=animation::Player::new();
        let mut renderer=hd::HdPresentation::new(960,540,&assets.palette);
        let checkpoints:Vec<_>=(0..8).map(|i|1+(clip.frames.len()-1)*i/7).collect();
        let mut draws=0;
        let mut captures=0;
        while player.step(&clip,&assets).is_some() {
            draws+=player.draws.len();
            if checkpoints.contains(&player.frame) {
                renderer.update(&player,&assets,&art,true,presentation::ColorMode::Classic);
                let path=out.join(format!("{}-{:04}.ppm",clip.name,player.frame));
                let mut f=fs::File::create(path).map_err(|e|e.to_string())?;
                write!(f,"P6\n960 540\n255\n").map_err(|e|e.to_string())?;
                let bytes:Vec<_>=renderer.pixels.iter().flat_map(|p|[(p>>16) as u8,(p>>8) as u8,*p as u8]).collect();
                f.write_all(&bytes).map_err(|e|e.to_string())?; captures+=1;
            }
        }
        let row=format!("{}\t{}\t{draws}\t{captures}\n",clip.name,player.frame);
        print!("{row}");log.push_str(&row);
    }
    fs::write(root.join("evidence/scene-review.tsv"),log).map_err(|e|e.to_string())?;
    Ok(())
}
