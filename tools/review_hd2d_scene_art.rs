//! Actual Rust sprite/background composition for new-scene registration review.
#[path="../src/resources.rs"] mod resources;
#[path="../src/remaster.rs"] mod remaster;
#[path="../src/hd2d_art.rs"] mod hd2d_art;
#[path="../src/hd2d_layout.rs"] mod hd2d_layout;
use std::{fs,io::Write,path::Path};
fn main()->Result<(),String>{
    let root=Path::new(env!("CARGO_MANIFEST_DIR"));let art=hd2d_art::Art::load(&root.join("assets/hd2d-v2"))?;
    let bg=remaster::load_image(root.join("assets/hd2d/backgrounds/ISLETEMP.SCR.rgba"))?;
    let layout=hd2d_layout::Layout::for_background("ISLETEMP.SCR");let (w,h)=(1280,700);
    let mut base=vec![0;w*h];for y in 0..h{for x in 0..w{base[y*w+x]=bg.pixels[y*bg.height/h*bg.width+x*bg.width/w];}}
    let out=root.join("captures/hd2d-new-layout");fs::create_dir_all(&out).map_err(|e|e.to_string())?;
    for (name,clip) in &art.clips {
        for phase in 0..clip.frames.len() {
            let mut pixels=base.clone();let foot=layout.grounded(hd2d_layout::Point::new(410.,318.));
            let point=if ["galleon","galleon-furled","liner","tanker","rowboat","motorboat"].contains(&name.as_str()) {
                let p=layout.ship_waterline(hd2d_layout::Point::new(250.,260.));[p.x,p.y]
            }else{[foot.x,foot.y]};
            art.draw(name,phase,&mut pixels,[w,h],[2.,0.,0.],point,1.,false)?;
            let mut f=fs::File::create(out.join(format!("{name}-{phase:03}.ppm"))).map_err(|e|e.to_string())?;
            write!(f,"P6\n{w} {h}\n255\n").map_err(|e|e.to_string())?;
            let bytes:Vec<_>=pixels.iter().flat_map(|p|[(p>>16) as u8,(p>>8) as u8,*p as u8]).collect();
            f.write_all(&bytes).map_err(|e|e.to_string())?;
        }
    }
    println!("Reviewed {} clips against measured painted-island support",art.clips.len());Ok(())
}
