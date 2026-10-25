//! Standalone HD-2D scene interpretation. The recovered VM supplies chronology
//! and semantic contacts; every visible sprite is drawn from the new clip set.
use crate::{
    animation::{Draw, PixelSource, Player},
    hd2d_art::Art,
    hd2d_canvas::Canvas,
    hd2d_layout::{Layout, Point, SceneKind},
    hd2d_roles::{self, Binding, Group, Role, Shape, Support},
    remaster::{AspectMode, RemasterAssets},
    resources::AssetStore,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
#[derive(Clone)]
struct Candidate {
    draw: Draw,
    shape: Shape,
    binding: Binding,
    old: Point,
    current: bool,
}
pub struct Scene {
    pub art: Art,
    pub canvas: Canvas,
    pub mask: Player,
    pub fx_art: RemasterAssets,
    pub layout: Layout,
    stats: BTreeMap<(String, usize), Shape>,
    motion: crate::hd2d_motion::Motion,
    pub audit: BTreeMap<String, BTreeSet<String>>,
    pub motion_samples: Vec<crate::hd2d_motion::Sample>,
    last_scene: String,
    last_frame: usize,
    dive_takeoff: Option<usize>,
}
impl Scene {
    pub fn load(root: &Path, width: usize, height: usize) -> Result<Self, String> {
        let art = Art::load(root)?;
        let fire = art
            .clips
            .get("campfire")
            .ok_or("New campfire clip missing")?;
        let fx_art = RemasterAssets {
            sprites: BTreeMap::from([(
                "FIRE1.BMP".into(),
                fire.frames.iter().map(|f| f.image.clone()).collect(),
            )]),
            backgrounds: BTreeMap::new(),
            widescreen: BTreeMap::new(),
            remade_frames: fire.frames.len(),
        };
        Ok(Self {
            art,
            canvas: Canvas::new(width, height),
            mask: Player::new(),
            fx_art,
            layout: Layout::for_background("ISLETEMP.SCR"),
            stats: BTreeMap::new(),
            motion: Default::default(),
            audit: BTreeMap::new(),
            motion_samples: vec![],
            last_scene: String::new(),
            last_frame: 0,
            dive_takeoff: None,
        })
    }
    pub fn resize(&mut self, width: usize, height: usize) {
        self.canvas.resize(width, height);
    }
    fn candidates(
        &mut self,
        player: &Player,
        assets: &AssetStore,
    ) -> Result<(Vec<Candidate>, Vec<(Role, Point)>), String> {
        let mut visible = vec![false; player.sources.len() + 1];
        for &id in player.origins.iter().chain(player.margin_origins.values()) {
            visible[id as usize] = true;
        }
        for (i, s) in player.sources.iter().enumerate().rev() {
            if visible[i + 1] {
                if let PixelSource::Layer { top, under } = s {
                    visible[*top as usize] = true;
                    visible[*under as usize] = true;
                }
            }
        }
        let mut draws: Vec<(Draw, bool)> =
            player.draws.iter().cloned().map(|d| (d, true)).collect();
        for (i, s) in player.sources.iter().enumerate().rev() {
            if visible[i + 1] {
                if let PixelSource::Sprite(d) = s {
                    if !draws.iter().any(|(p, _)| p == d) {
                        draws.push((d.clone(), false));
                    }
                }
            }
        }
        let mut candidates = vec![];
        let mut effects = vec![];
        for (draw, current) in draws {
            let shape = *self
                .stats
                .entry((draw.name.clone(), draw.frame))
                .or_insert_with(|| {
                    assets
                        .sprites
                        .get(&draw.name)
                        .and_then(|frames| frames.get(draw.frame))
                        .map(|im| Shape::analyze(im, &assets.palette))
                        .unwrap_or_default()
                });
            let image = assets
                .sprites
                .get(&draw.name)
                .and_then(|f| f.get(draw.frame))
                .ok_or("Unknown semantic source")?;
            let sx = draw.width as f32 / image.width.max(1) as f32;
            let sy = draw.height as f32 / image.height.max(1) as f32;
            let middle = (shape.bbox[0] + shape.bbox[2]) as f32 * 0.5;
            let mut old = Point::new(
                draw.x as f32
                    + if draw.flip_x {
                        draw.width as f32 - middle * sx
                    } else {
                        middle * sx
                    },
                draw.y as f32 + shape.bbox[3] as f32 * sy,
            );
            let role = hd2d_roles::bind(&draw.name, draw.frame, shape);
            if let Role::Sprite(binding) = role {
                if matches!(binding.group, Group::Hero | Group::Suzy | Group::Mermaid)
                    && matches!(binding.support, Support::Ground | Support::Tree)
                {
                    old.x = draw.x as f32
                        + if draw.flip_x {
                            draw.width as f32 - shape.foot_x * sx
                        } else {
                            shape.foot_x * sx
                        };
                }
            }
            let label = match role {
                Role::Sprite(b) => b.clip.to_string(),
                Role::Unknown => {
                    return Err(format!(
                        "No new HD-2D role for {}:{}",
                        draw.name, draw.frame
                    ))
                }
                other => format!("{other:?}"),
            };
            self.audit
                .entry(draw.name.clone())
                .or_default()
                .insert(label);
            match role {
                Role::Sprite(binding) => candidates.push(Candidate {
                    draw,
                    shape,
                    binding,
                    old,
                    current,
                }),
                Role::WaterEffect
                | Role::Smoke
                | Role::Spark
                | Role::Projectile
                | Role::Score(_)
                | Role::Idea
                | Role::Question
                | Role::SleepText => effects.push((role, old)),
                _ => {}
            }
        }
        Ok((candidates, effects))
    }
    fn animated_pose(
        &mut self,
        key: &str,
        point: Point,
        clip: &str,
        time: f64,
        flip: bool,
    ) -> crate::hd2d_motion::Pose {
        let art = &self.art.clips[clip];
        let pose = self
            .motion
            .sample(key, clip, point, time, art.frames.len(), art.fps, flip);
        self.motion_samples.push(crate::hd2d_motion::Sample {
            key: key.into(),
            clip: clip.into(),
            point: pose.point,
            phase: pose.phase,
            flip: pose.flip,
        });
        pose
    }
    fn source_scale(&self, c: &Candidate) -> f32 {
        let clip = &self.art.clips[c.binding.clip];
        if c.binding.group == Group::Crew {
            let height = clip
                .frames
                .iter()
                .map(|f| f.image.height)
                .max()
                .unwrap_or(1) as f32
                / clip.ppu;
            return (c.shape.height() as f32 * 0.55 / height).clamp(0.12, 1.5);
        }
        let maxw = clip.frames.iter().map(|f| f.image.width).max().unwrap_or(1) as f32 / clip.ppu;
        match c.binding.group {
            Group::Hero | Group::Suzy | Group::Mermaid | Group::Pair => 1.,
            Group::Vehicle => {
                if c.draw.name == "GJPROW.BMP" {
                    2.
                } else {
                    (c.shape.width() as f32 / maxw).clamp(0.12, 4.)
                }
            }
            Group::Animal => {
                if c.binding.clip == "gull" {
                    0.75
                } else {
                    1.
                }
            }
            Group::Prop => {
                if c.binding.clip == "bottle" {
                    0.75
                } else {
                    1.
                }
            }
            _ => 1.,
        }
    }
    fn point(&self, c: &Candidate) -> Point {
        match c.binding.support {
            Support::Ground | Support::Pair => {
                if c.binding.group == Group::Crew && c.old.y < 275. {
                    return self.layout.ship_waterline(c.old);
                }
                if c.binding.clip == "campfire"
                    || (c.binding.clip == "driftwood" && c.draw.name == "FIRE1.BMP")
                {
                    return Point::new(350., 261.);
                }
                if ["johnny-ignite", "johnny-tend"].contains(&c.binding.clip)
                    && self.layout.kind == SceneKind::Island
                {
                    return Point::new(390., 267.);
                }
                if c.binding.clip == "johnny-firecraft" {
                    return Point::new(350., 261.);
                }
                if c.binding.clip == "johnny-hot-meal" || c.binding.clip == "johnny-roast" {
                    return Point::new(405., 267.);
                }
                if c.binding.clip.starts_with("johnny-fish")
                    && self.layout.kind == SceneKind::Island
                {
                    return Point::new(
                        if c.old.x < self.layout.original_palm.x {
                            136.
                        } else {
                            532.
                        },
                        258.,
                    );
                }
                if self.last_scene == "MJFIRE.TTM"
                    && c.binding.group == Group::Hero
                    && self.layout.kind == SceneKind::Island
                {
                    // The new fire is at 350, whereas the old one is at 313.
                    // Register the complete approach/departure to that contact.
                    let x = 350. + (c.old.x - 313.) * (415. - 350.) / (448. - 313.);
                    return Point::new(x, self.layout.sand_front(x));
                }
                self.layout.grounded(c.old)
            }
            Support::Tree => self.layout.climbing(c.old),
            Support::Sky => self.layout.sky(c.old),
            Support::Water => {
                if c.binding.clip.starts_with("johnny-dive")
                    && self.layout.kind == SceneKind::Island
                {
                    let t = ((self.layout.original_palm.x - c.old.x) / 250.).clamp(0., 1.);
                    return Point::new(
                        self.layout.palm_crown.x - 370. * t,
                        self.layout.palm_crown.y + 240. * t * t,
                    );
                }
                if matches!(c.binding.group, Group::Vehicle | Group::Crew) {
                    if c.draw.name == "GJPROW.BMP" {
                        return Point::new(c.old.x, 340.);
                    }
                    let mut old = c.old;
                    if c.draw.name == "SHIPS.BMP" && c.draw.frame == 7 {
                        old.y += 34.;
                    }
                    self.layout.ship_waterline(old)
                } else {
                    self.layout.swimming(c.old)
                }
            }
            Support::Contact => self.layout.grounded(c.old),
        }
    }
    fn draw_sprite(
        &mut self,
        clip: &str,
        phase: usize,
        point: Point,
        scale: f32,
        flip: bool,
        ground_shadow: bool,
    ) -> Result<[f32; 4], String> {
        if ground_shadow {
            self.canvas.shadow(point, 10. * scale, 0.17);
        }
        let rect = self.art.draw(
            clip,
            phase,
            &mut self.canvas.pixels,
            [self.canvas.width, self.canvas.height],
            self.canvas.stage,
            [point.x, point.y],
            scale,
            flip,
        )?;
        self.mask_sprite(clip, phase, point, scale, flip, rect)?;
        Ok(rect)
    }
    fn mask_sprite(
        &mut self,
        clip: &str,
        phase: usize,
        point: Point,
        scale: f32,
        flip: bool,
        rect: [f32; 4],
    ) -> Result<(), String> {
        let (frame, ppu) = self.art.frame(clip, phase)?;
        let image = &frame.image;
        let factor = scale / ppu;
        let factor_y = rect[3] / image.height as f32;
        let flip = flip ^ frame.flip_x;
        let draw = Draw {
            name: if clip == "campfire" {
                "FIRE1.BMP".into()
            } else {
                clip.into()
            },
            frame: phase % self.art.clips[clip].frames.len(),
            x: rect[0].floor() as i32,
            y: rect[1].floor() as i32,
            width: rect[2].ceil() as u32,
            height: rect[3].ceil() as u32,
            flip_x: flip,
            flip_y: false,
            clip: [-640, 0, 1280, self.layout.height as i32],
            pose_signature: 0,
        };
        let id = self.mask.sources.len() as u32 + 1;
        self.mask.sources.push(PixelSource::Sprite(draw));
        let [left, top, w, h] = rect;
        // Provenance is needed only where the current viewport can display it.
        // A foreground ship can span several stages: inserting its invisible
        // wings into the sparse margin map dominated the frame's render time.
        let [s, ox, _] = self.canvas.stage;
        let visible_left = ((-ox / s).floor() as i32).max(-640);
        let visible_right = (((self.canvas.width as f64 - ox) / s).ceil() as i32).min(1280);
        for y in
            (top.floor() as i32).max(0)..((top + h).ceil() as i32).min(self.layout.height as i32)
        {
            let py = (((y as f32 + 0.5 - top) / factor_y) as usize).min(image.height - 1);
            for x in (left.floor() as i32).max(visible_left)
                ..((left + w).ceil() as i32).min(visible_right)
            {
                let mut px = (((x as f32 + 0.5 - left) / factor) as usize).min(image.width - 1);
                if flip {
                    px = image.width - 1 - px;
                }
                if image.pixels[py * image.width + px] >> 24 < 24 {
                    continue;
                }
                if (0..640).contains(&x) {
                    self.mask.origins[y as usize * 640 + x as usize] = id;
                } else {
                    self.mask.margin_origins.insert((y, x), id);
                }
            }
        }
        let _ = point;
        Ok(())
    }
    fn terrain_foreground(&mut self) {
        if self.layout.kind != SceneKind::Island {
            return;
        }
        let [s, ox, oy] = self.canvas.stage;
        let y0 = (oy + (self.layout.sand_center.y - self.layout.sand_radius.y) as f64 * s)
            .floor()
            .max(0.) as usize;
        let y1 = (oy + (self.layout.sand_center.y + self.layout.sand_radius.y) as f64 * s)
            .ceil()
            .min(self.canvas.height as f64) as usize;
        for y in y0..y1 {
            let wy = ((y as f64 + 0.5 - oy) / s) as f32;
            for x in 0..self.canvas.width {
                let wx = ((x as f64 + 0.5 - ox) / s) as f32;
                if !self.layout.sand_contains(Point::new(wx, wy)) {
                    continue;
                }
                let at = y * self.canvas.width + x;
                let c = self.canvas.base[at];
                let r = (c >> 16) & 255;
                let g = (c >> 8) & 255;
                let b = c & 255;
                if r * 10 >= g * 9 && g * 10 > b * 11 && r > 100 {
                    self.canvas.pixels[at] = c;
                }
            }
        }
    }
    pub fn render(
        &mut self,
        player: &Player,
        assets: &AssetStore,
        scene_name: &str,
        aspect: AspectMode,
        time: f64,
    ) -> Result<(), String> {
        if self.last_scene != scene_name || player.frame < self.last_frame {
            self.motion.clear();
            self.dive_takeoff = None;
            self.last_scene = scene_name.into();
        }
        self.last_frame = player.frame;
        self.motion.begin_frame();
        self.motion_samples.clear();
        let background = player
            .current_background
            .as_deref()
            .unwrap_or("ISLETEMP.SCR");
        self.layout = Layout::for_background(background);
        self.canvas.background(&self.art, background, aspect)?;
        self.mask.current_background = Some(background.into());
        self.mask.width = 640;
        self.mask.height = self.layout.height as u32;
        self.mask.sources.clear();
        self.mask
            .sources
            .push(PixelSource::Background(background.into()));
        self.mask.origins.fill(1);
        self.mask.margin_origins.clear();
        if scene_name == "THEEND.TTM" && background == "THEEND.SCR" {
            let ending = crate::hd2d_ending::frame(player.frame.saturating_sub(50) as f32 / 60.);
            self.canvas
                .stage_overlay(&self.art.ending_clean, ending.clean_opacity);
            for layer in ending.layers {
                let rect = self.art.draw_transformed(
                    layer.clip,
                    layer.phase,
                    &mut self.canvas.pixels,
                    [self.canvas.width, self.canvas.height],
                    self.canvas.stage,
                    [layer.point.x, layer.point.y],
                    layer.scale,
                    layer.flip,
                    Some(0x322d35),
                    layer.opacity,
                )?;
                self.mask_sprite(
                    layer.clip,
                    layer.phase,
                    layer.point,
                    layer.scale[0],
                    layer.flip,
                    rect,
                )?;
                self.motion_samples.push(crate::hd2d_motion::Sample {
                    key: layer.clip.into(),
                    clip: layer.clip.into(),
                    point: layer.point,
                    phase: layer.phase,
                    flip: layer.flip,
                });
            }
            self.canvas.preserve_frame();
            return Ok(());
        }
        if self.layout.kind == SceneKind::Island {
            let terrain = Draw {
                name: "scene-terrain".into(),
                frame: 0,
                x: 0,
                y: 0,
                width: 640,
                height: 350,
                flip_x: false,
                flip_y: false,
                clip: [0, 0, 640, 350],
                pose_signature: 0,
            };
            let id = self.mask.sources.len() as u32 + 1;
            self.mask.sources.push(PixelSource::Sprite(terrain));
            for y in 0..350 {
                for x in 0..640 {
                    if self
                        .layout
                        .sand_contains(Point::new(x as f32 + 0.5, y as f32 + 0.5))
                    {
                        self.mask.origins[y * 640 + x] = id;
                    }
                }
            }
        }
        if let Some(story) = crate::hd2d_story::frame(scene_name, player.frame) {
            if story.black_background {
                self.canvas.pixels.fill(0);
                self.mask.current_background = Some("JOFFICE.SCR".into());
                self.mask.origins.fill(0);
                self.mask.margin_origins.clear();
            }
            let vessel = |clip: &str| {
                matches!(
                    clip,
                    "galleon"
                        | "tiny-rowboat"
                        | "tiny-rowboat-empty"
                        | "motorboat"
                        | "clown-boat"
                        | "party-yacht"
                )
            };
            let mut ordered: Vec<_> = story.layers.iter().collect();
            ordered.sort_by(|a, b| {
                (!vessel(a.clip)).cmp(&(!vessel(b.clip))).then_with(|| {
                    a.depth
                        .unwrap_or(a.point.y)
                        .total_cmp(&b.depth.unwrap_or(b.point.y))
                })
            });
            for layer in ordered {
                let rect = self.draw_sprite(
                    layer.clip,
                    layer.phase,
                    layer.point,
                    layer.scale,
                    layer.flip,
                    layer.shadow,
                )?;
                if let Some(waterline) = layer.waterline {
                    self.canvas.water_cover(rect, waterline);
                }
                if layer.clip == "johnny-climb"
                    || (layer.clip == "johnny-run" && layer.point.y < 240.)
                    || (layer.clip == "rescue-biplane" && layer.depth == Some(100.))
                {
                    self.canvas
                        .palm_foreground(self.layout, rect, layer.clip != "johnny-climb");
                }
                if vessel(layer.clip) {
                    self.terrain_foreground();
                    self.canvas.palm_foreground(self.layout, rect, true);
                }
                if layer.clip == "mermaid-swim" && layer.point.y < 220. {
                    self.terrain_foreground();
                    self.canvas.palm_foreground(self.layout, rect, true);
                }
                self.motion_samples.push(crate::hd2d_motion::Sample {
                    key: format!("story:{}", story.beat),
                    clip: layer.clip.into(),
                    point: layer.point,
                    phase: layer.phase,
                    flip: layer.flip,
                });
            }
            for (index, text) in &story.scores {
                let layer = &story.layers[*index];
                if let Some(at) = self.art.contact(
                    layer.clip,
                    layer.phase,
                    "card_center",
                    [layer.point.x, layer.point.y],
                    layer.scale,
                    layer.flip,
                ) {
                    self.canvas.score_digits(Point::new(at[0], at[1]), text);
                }
            }
            if let Some(line) = &story.line {
                let actor = &story.layers[line.actor];
                if let Some(tip) = self.art.contact(
                    actor.clip,
                    actor.phase,
                    "rod_tip",
                    [actor.point.x, actor.point.y],
                    actor.scale,
                    actor.flip,
                ) {
                    self.canvas
                        .fishing_line(Point::new(tip[0], tip[1]), line.end, line.taut, time);
                    if line.in_water {
                        self.canvas.ripple(line.end, 4., time as f32 * 2.);
                    }
                }
            }
            if let Some((point, age)) = story.splash {
                self.canvas.ripple(point, 3. + age * 14., time as f32 * 2.);
                if age < 0.3 {
                    self.draw_sprite("splash", 0, point, 0.4, false, false)?;
                }
            }
            if let Some(point) = story.smoke {
                self.canvas.smoke(point, time);
            }
            if let Some((from, to)) = story.lightning {
                self.canvas.lightning(from, to);
            }
            for point in story.projectiles {
                self.canvas.projectile(point);
            }
            for (from, to) in story.ropes {
                self.canvas.fishing_line(from, to, true, time);
            }
            self.canvas.preserve_frame();
            return Ok(());
        }
        let (mut candidates, effects) = self.candidates(player, assets)?;
        // One physical animal/prop can be assembled from several recovered
        // layers. Choosing a complete new drawing for every part duplicates it.
        let mut resolved: BTreeMap<String, Candidate> = BTreeMap::new();
        candidates.retain(|c| {
            if c.binding.group == Group::Animal
                || ["sandcastle", "raft-building", "dinner-table", "gramophone"]
                    .contains(&c.binding.clip)
            {
                let key = format!("{}:{:?}", c.binding.clip, c.binding.support);
                let replace = resolved.get(&key).is_none_or(|old| {
                    (c.current, c.shape.opaque) > (old.current, old.shape.opaque)
                });
                if replace {
                    resolved.insert(key, c.clone());
                }
                false
            } else {
                true
            }
        });
        candidates.extend(resolved.into_values());
        // Close interactions are one authored new unit. Old body/arm/head and
        // chair companions must not create several complete people underneath it.
        let pair = candidates
            .iter()
            .filter(|c| c.binding.group == Group::Pair)
            .max_by_key(|c| (c.current, c.binding.priority, c.shape.opaque))
            .cloned();
        let selected_hero = candidates
            .iter()
            .filter(|c| c.binding.group == Group::Hero)
            .max_by_key(|c| (c.current, c.binding.priority, c.shape.opaque))
            .cloned();
        let body = candidates
            .iter()
            .filter(|c| c.binding.group == Group::Hero && c.shape.height() >= 30)
            .max_by_key(|c| (c.current, c.shape.opaque))
            .cloned();
        let mut hero = selected_hero;
        if let (Some(hero), Some(body)) = (&mut hero, body) {
            if hero.shape.height() < 30 {
                hero.old = body.old;
            }
        }
        let npc_suzy = candidates
            .iter()
            .filter(|c| c.binding.group == Group::Suzy)
            .max_by_key(|c| (c.current, c.shape.opaque))
            .cloned();
        let npc_mermaid = candidates
            .iter()
            .filter(|c| c.binding.group == Group::Mermaid)
            .max_by_key(|c| (c.current, c.shape.opaque))
            .cloned();
        candidates.retain(|c| {
            !matches!(
                c.binding.group,
                Group::Hero | Group::Suzy | Group::Mermaid | Group::Pair
            )
        });
        let mut front_water = vec![];
        let mut foreground = vec![];
        candidates.sort_by_key(|c| c.binding.group == Group::Crew);
        let mut vessels: Vec<(Candidate, Point)> = vec![];
        for mut c in candidates {
            if c.binding.clip == "octopus"
                && self.layout.kind == SceneKind::Island
                && c.old.y >= 285.
            {
                c.binding.support = Support::Ground;
            }
            if c.binding.support == Support::Water || c.binding.support == Support::Sky {
                if c.draw.name == "GJPROW.BMP" {
                    front_water.push(c);
                    continue;
                }
                let mut point = self.point(&c);
                let mut attached = false;
                if c.binding.group == Group::Crew {
                    if let Some((vessel, at)) = vessels
                        .iter()
                        .filter(|(v, _)| {
                            c.old.x >= v.draw.x as f32 - 8.
                                && c.old.x <= (v.draw.x + v.draw.width as i32) as f32 + 8.
                                && c.old.y <= v.old.y + 8.
                        })
                        .min_by(|(a, _), (b, _)| {
                            (a.old.x - c.old.x)
                                .abs()
                                .total_cmp(&(b.old.x - c.old.x).abs())
                        })
                    {
                        point = Point::new(
                            at.x + c.old.x - vessel.old.x,
                            at.y + c.old.y - vessel.old.y,
                        );
                        attached = true;
                    }
                }
                let key = self.motion.entity_key(
                    &format!("{}:{}", c.draw.name, c.binding.clip),
                    point,
                    time,
                );
                let mut pose = self.animated_pose(&key, point, c.binding.clip, time, c.draw.flip_x);
                if attached {
                    // The hull already has a continuous root. Smoothing the
                    // passenger a second time makes them trail outside the boat.
                    pose.point = point;
                    if let Some(sample) = self.motion_samples.last_mut() {
                        sample.point = point;
                    }
                }
                let scale = self.motion.scale(&key, self.source_scale(&c), time);
                let rect = self.draw_sprite(
                    c.binding.clip,
                    pose.phase,
                    pose.point,
                    scale,
                    pose.flip,
                    false,
                )?;
                if c.binding.support == Support::Water && c.binding.group == Group::Vehicle {
                    self.canvas.water_cover(rect, pose.point.y - 1.3);
                    self.canvas
                        .ripple(pose.point, rect[2] * 0.52, (time * 2.) as f32);
                    vessels.push((c.clone(), pose.point));
                }
            } else {
                foreground.push(c);
            }
        }
        self.terrain_foreground();
        let has_raft = foreground
            .iter()
            .any(|c| c.binding.clip.starts_with("raft"));
        let mut actors = vec![];
        if pair.is_none() {
            for c in [hero.as_ref(), npc_suzy.as_ref(), npc_mermaid.as_ref()]
                .into_iter()
                .flatten()
            {
                let mut clip = c.binding.clip;
                let mut support = c.binding.support;
                if self.layout.kind == SceneKind::Underwater && c.binding.group == Group::Hero {
                    clip = "johnny-swim";
                    support = Support::Water;
                }
                let mut point = if support == Support::Water && !clip.starts_with("johnny-dive") {
                    self.layout.swimming(c.old)
                } else {
                    self.point(&c)
                };
                let pose = self.animated_pose(
                    &format!("{:?}", c.binding.group),
                    point,
                    clip,
                    time,
                    c.draw.flip_x,
                );
                point = pose.point;
                let (mut phase, mut flip) = (pose.phase, pose.flip);
                if let Some(authored) = hd2d_roles::authored_phase(&c.draw.name, c.draw.frame) {
                    phase = authored;
                }
                if clip == "johnny-firecraft" {
                    point = Point::new(350., 261.);
                    flip = true;
                }
                if clip == "johnny-hot-meal" || clip == "johnny-roast" {
                    point = Point::new(405., 267.);
                    flip = true;
                }
                if clip.starts_with("johnny-fish") && self.layout.kind == SceneKind::Island {
                    flip = c.old.x < self.layout.original_palm.x;
                }
                let flight = c.binding.group == Group::Hero
                    && matches!(c.draw.name.as_str(), "MJDIVE.BMP" | "GJDIVE.BMP")
                    && matches!(clip, "johnny-dive" | "johnny-dive-down")
                    && self.layout.kind == SceneKind::Island;
                if flight {
                    // Authored scene clocks also support selecting HD-2D midway
                    // through a dive, without restarting it at the palm crown.
                    let start = match scene_name {
                        "MJDIVE.TTM" => 505,
                        "GJDIVE.TTM" => 510 + player.frame.saturating_sub(510) / 1150 * 1150,
                        _ => *self.dive_takeoff.get_or_insert(player.frame),
                    };
                    let duration = if scene_name == "GJDIVE.TTM" { 70. } else { 60. };
                    let t = (player.frame.saturating_sub(start) as f32 / duration).clamp(0., 1.);
                    point = Point::new(465. - 365. * t, 46. + 256. * t * t);
                    if let Some(sample) = self.motion_samples.last_mut() {
                        sample.point = point;
                    }
                } else if c.binding.group == Group::Hero {
                    self.dive_takeoff = None;
                }
                if clip.starts_with("johnny-dive") {
                    flip = true;
                    if !flight
                        && matches!(clip, "johnny-dive" | "johnny-dive-down")
                        && self.layout.kind == SceneKind::Island
                    {
                        // Flight poses anchor at the body centre; the takeoff pose
                        // anchors at its feet. Blend that offset out toward impact.
                        point.y -= 24. * ((point.x - 100.) / 365.).clamp(0., 1.);
                    }
                }
                if clip == "johnny-bath" {
                    phase = match c.draw.frame {
                        0..=2 => 1,
                        3..=8 => 2,
                        14..=16 => 3,
                        _ => 0,
                    };
                }
                if ["johnny-ignite", "johnny-tend"].contains(&clip) {
                    flip = point.x > 350.;
                }
                let mut scale = 1.;
                if self.layout.kind == SceneKind::Office && c.binding.group == Group::Hero {
                    clip = "johnny-office";
                    phase = match c.draw.frame {
                        0..=4 => (time * 3.) as usize % 2,
                        5..=8 => 2,
                        _ => 3,
                    };
                    point = Point::new(350., 223.);
                    scale = 0.75;
                    flip = false;
                }
                if c.binding.group == Group::Mermaid && self.layout.kind != SceneKind::Underwater {
                    scale = 0.9;
                }
                if let Some(sample) = self.motion_samples.last_mut() {
                    sample.clip = clip.into();
                    sample.phase = phase;
                    sample.point = point;
                    sample.flip = flip;
                }
                actors.push((c.clone(), clip, support, point, phase, flip, scale));
            }
        }
        if scene_name == "SUZYCITY.TTM" {
            if let Some(suzy) = actors
                .iter()
                .find(|a| a.0.binding.group == Group::Suzy)
                .map(|a| a.3)
            {
                for actor in &mut actors {
                    if actor.0.binding.group == Group::Hero && actor.0.draw.name == "SSUZY3.BMP" {
                        actor.3 = Point::new(suzy.x + 35., suzy.y - 89.);
                        actor.6 = 0.35;
                    }
                }
            }
        }
        let main_actor = if scene_name == "SUZYCITY.TTM" {
            npc_suzy.as_ref()
        } else {
            hero.as_ref().or(npc_suzy.as_ref()).or(npc_mermaid.as_ref())
        };
        let main_point = if scene_name == "SUZYCITY.TTM" {
            actors
                .iter()
                .find(|a| a.0.binding.group == Group::Suzy)
                .map(|a| a.3)
        } else {
            actors.first().map(|a| a.3)
        };
        let main_old = main_actor.map(|h| h.old);
        if self.layout.kind == SceneKind::Island
            && !has_raft
            && hero
                .as_ref()
                .is_some_and(|h| h.binding.clip == "johnny-build")
        {
            if let Some(p) = main_point {
                self.draw_sprite(
                    "raft-building",
                    0,
                    Point::new(p.x + 28., p.y + 3.),
                    0.8,
                    false,
                    true,
                )?;
            }
        }
        let mut placed: BTreeSet<(String, i32, i32)> = BTreeSet::new();
        for c in foreground {
            if c.binding.clip == "fish-tropical"
                && c.draw.name == "MJFISH3.BMP"
                && actors
                    .iter()
                    .any(|a| a.1 == "johnny-fishing-gag" && a.4 < 7)
            {
                continue;
            }
            if c.binding.clip == "driftwood"
                && c.draw.name == "FIRE1.BMP"
                && hero
                    .as_ref()
                    .is_some_and(|h| h.binding.clip == "johnny-firecraft")
            {
                continue;
            }
            if pair.is_some() && matches!(c.binding.clip, "dinner-table" | "flowers" | "gift") {
                continue;
            }
            if hero
                .as_ref()
                .is_some_and(|h| h.binding.clip == "johnny-read")
                && c.binding.clip == "book"
            {
                continue;
            }
            if hero
                .as_ref()
                .is_some_and(|h| h.binding.clip == "johnny-fish")
                && c.binding.clip == "fishing-rod"
            {
                continue;
            }
            let mut point = self.point(&c);
            if c.binding.clip.starts_with("thought-") {
                point =
                    main_point.map_or(Point::new(350., 180.), |p| Point::new(p.x + 35., p.y - 80.));
            }
            if c.binding.support == Support::Contact {
                if let (Some(new), Some(old)) = (main_point, main_old) {
                    if (c.old.x - old.x).abs() < 100. && (c.old.y - old.y).abs() < 110. {
                        point = Point::new(
                            new.x + (c.old.x - old.x) * 0.85,
                            new.y + (c.old.y - old.y) * 0.85,
                        );
                    }
                }
            }
            if !placed.insert((
                c.binding.clip.into(),
                (point.x * 2.) as i32,
                (point.y * 2.) as i32,
            )) {
                continue;
            }
            let phase = (time * self.art.clips[c.binding.clip].fps as f64) as usize;
            self.draw_sprite(
                c.binding.clip,
                phase,
                point,
                self.source_scale(&c),
                c.draw.flip_x,
                c.binding.support == Support::Ground,
            )?;
        }
        if let Some(pair) = pair {
            let pose = self.animated_pose(
                "Pair",
                self.point(&pair),
                pair.binding.clip,
                time,
                pair.draw.flip_x,
            );
            let point = pose.point;
            self.draw_sprite(
                pair.binding.clip,
                0,
                point,
                1.,
                pair.draw.flip_x,
                pair.binding.support != Support::Water,
            )?;
        } else {
            for (c, clip, support, point, phase, flip, scale) in actors {
                let rect = self.draw_sprite(
                    clip,
                    phase,
                    point,
                    scale,
                    flip,
                    support == Support::Ground
                        && self.layout.kind != SceneKind::Office
                        && clip != "johnny-firecraft",
                )?;
                if clip == "johnny-fishing-cast" {
                    if let Some(tip) =
                        self.art
                            .contact(clip, phase, "rod_tip", [point.x, point.y], scale, flip)
                    {
                        let direction = if flip { -1. } else { 1. };
                        let float = Point::new(point.x + direction * 125., 300.);
                        self.canvas.fishing_line(
                            Point::new(tip[0], tip[1]),
                            float,
                            phase >= 6,
                            time,
                        );
                    }
                }
                if support == Support::Water
                    && c.binding.group == Group::Hero
                    && self.layout.kind != SceneKind::Underwater
                    && !clip.starts_with("johnny-dive")
                {
                    self.canvas.water_cover(rect, point.y);
                }
                if self.layout.kind == SceneKind::Island
                    && matches!(clip, "johnny-dive" | "johnny-dive-down")
                    && point.y > 260.
                {
                    self.canvas.water_cover(rect, 302.);
                }
                if support == Support::Ground && self.layout.kind == SceneKind::Island {
                    self.canvas.palm_foreground(self.layout, rect, false);
                }
            }
        }
        for c in front_water {
            let point = self.point(&c);
            self.draw_sprite(
                c.binding.clip,
                0,
                point,
                self.source_scale(&c),
                c.draw.flip_x,
                false,
            )?;
        }
        // Effects are authored in new scene space; no old splash/shore bitmaps.
        for (role, old) in effects {
            match role {
                Role::Idea | Role::Question => {
                    if let Some(point) = main_point {
                        let clip = if matches!(role, Role::Idea) {
                            "idea-bulb"
                        } else {
                            "thought-question"
                        };
                        self.draw_sprite(
                            clip,
                            0,
                            Point::new(point.x + 25., point.y - 82.),
                            0.55,
                            false,
                            false,
                        )?;
                    }
                }
                Role::SleepText => {
                    if let Some(point) = main_point {
                        self.canvas.sleep_text(point, time);
                    }
                }
                Role::Smoke => {
                    let point = if scene_name.contains("FIRE") {
                        Point::new(350., 261.)
                    } else {
                        self.layout.grounded(old)
                    };
                    self.canvas.smoke(point, time);
                }
                Role::Spark => {
                    self.canvas
                        .ripple(self.layout.ship_waterline(old), 2., time as f32 * 15.);
                }
                Role::Projectile => {
                    self.canvas.projectile(self.layout.ship_waterline(old));
                }
                Role::Score(text) => {
                    let p = self.layout.grounded(old);
                    self.canvas.scorecard(Point::new(p.x, p.y - 18.), text);
                }
                Role::WaterEffect => {
                    if self.layout.kind != SceneKind::Underwater {
                        let point = if scene_name == "MJDIVE.TTM" {
                            Point::new(100., 302.)
                        } else {
                            self.layout.swimming(old)
                        };
                        self.canvas.ripple(
                            point,
                            8. + (time as f32 * 3.).sin() * 2.,
                            time as f32 * 4.,
                        );
                    }
                }
                _ => {}
            }
        }
        if scene_name == "FIRE.TTM" {
            self.draw_sprite(
                "campfire",
                (time * 10.) as usize,
                Point::new(350., 261.),
                1.,
                false,
                false,
            )?;
            self.draw_sprite("johnny-tend", 0, Point::new(390., 267.), 1., true, true)?;
        }
        self.canvas.preserve_frame();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_ship_only_masks_the_visible_panorama_margins() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut scene = Scene::load(&root.join("assets/hd2d-v2"), 1920, 1080).unwrap();
        scene.canvas.stage = [3., 0., 15.];
        scene
            .draw_sprite("tanker", 0, Point::new(320., 500.), 8., false, false)
            .unwrap();
        assert!(scene.mask.margin_origins.is_empty());
        assert!(scene.mask.origins.iter().any(|&id| id != 0));
        scene.resize(3440, 1440);
        scene.canvas.stage = [4., 440., 20.];
        scene
            .draw_sprite("tanker", 0, Point::new(320., 500.), 8., false, false)
            .unwrap();
        assert!(!scene.mask.margin_origins.is_empty());
        assert!(scene
            .mask
            .margin_origins
            .keys()
            .all(|&(y, x)| (0..350).contains(&y)
                && (-110..750).contains(&x)
                && !(0..640).contains(&x)));
    }
    #[test]
    fn held_scene_repaints_without_accumulating_sprites_or_effects() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let assets = AssetStore::load(root.join("original")).unwrap();
        let raw: Vec<_> = assets
            .raw
            .iter()
            .filter(|(n, _)| n.ends_with(".TTM"))
            .map(|(n, b)| crate::animation::Clip::parse(n, b).unwrap())
            .collect();
        let clips = crate::scheduler::build_scene_clips(&assets, &raw).unwrap();
        let clip = clips.iter().find(|c| c.name == "MJREAD.TTM").unwrap();
        let mut p = Player::new();
        for _ in 0..600 {
            p.step(clip, &assets);
        }
        let mut scene = Scene::load(&root.join("assets/hd2d-v2"), 960, 405).unwrap();
        scene
            .render(&p, &assets, &clip.name, AspectMode::Ratio43_18, 10.)
            .unwrap();
        let first = scene.canvas.pixels.clone();
        let sources = scene.mask.sources.len();
        scene
            .render(&p, &assets, &clip.name, AspectMode::Ratio43_18, 10.)
            .unwrap();
        assert_eq!(first, scene.canvas.pixels);
        assert_eq!(sources, scene.mask.sources.len());
        assert!(scene.mask.sources.iter().all(|s|!matches!(s,PixelSource::Sprite(d) if d.name.ends_with(".BMP") && d.name!="FIRE1.BMP")));
        assert!(scene
            .mask
            .sources
            .iter()
            .any(|s| matches!(s,PixelSource::Sprite(d) if d.name=="johnny-read")));
    }
}
