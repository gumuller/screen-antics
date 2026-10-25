//! Executes the original ADS rules into a deterministic, bounded native timeline.
//! The original Windows application chooses which ADS segment to start. For the
//! scene browser we select the segment that directly starts the requested TTM.
use crate::animation::{Clip, Command, Frame};
use crate::resources::{chunks, decompress, AssetStore};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
struct Op {
    code: u16,
    args: Vec<i16>,
}
#[derive(Clone, Debug)]
struct Script {
    name: String,
    envs: BTreeMap<i16, String>,
    segments: BTreeMap<i16, Vec<Op>>,
    segment_order: Vec<i16>,
}
#[derive(Clone, Default)]
struct Actor {
    played: bool,
    running: bool,
    paused: bool,
    finished: bool,
    start: usize,
    frame: usize,
    due: u32,
    repeats: u32,
    cut: Option<u32>,
    finish_next: bool,
    interval: u32,
    bmp: i16,
    getput: i16,
    brush: i16,
    clip: [i16; 4],
    colors: [i16; 2],
    drawing: Vec<Command>,
}
impl Actor {
    fn fresh(frame: usize) -> Self {
        Self {
            start: frame,
            frame,
            interval: 1,
            clip: [0, 0, 639, 479],
            colors: [15, 15],
            ..Self::default()
        }
    }
}

fn nargs(code: u16) -> usize {
    match code {
        0x1080 | 0x1090 | 0x1380 | 0x1390 | 0x13a0 | 0x13a1 | 0x13b0 | 0x13b1 | 0x13c0 | 0x13c1
        | 0x3020 | 0xf010 | 0xf200 | 0xf210 => 1,
        0x1010..=0x1070 | 0x1310..=0x1370 => 2,
        0x2010 | 0x2015 | 0x2020 | 0x4000 | 0x4010 => 3,
        0x2000 | 0x2005 => 4,
        _ => 0,
    }
}
fn word(data: &[u8], p: &mut usize) -> Result<i16, String> {
    let b = data.get(*p..*p + 2).ok_or("Truncated ADS operand")?;
    *p += 2;
    Ok(i16::from_le_bytes([b[0], b[1]]))
}
impl Script {
    fn parse(name: &str, data: &[u8]) -> Result<Self, String> {
        let mut envs = BTreeMap::new();
        let mut stream = None;
        for (id, d) in chunks(data)? {
            if &id == b"RES:" {
                let mut p = 0;
                let count = word(d, &mut p)?;
                for _ in 0..count {
                    let env = word(d, &mut p)?;
                    let end = d[p..]
                        .iter()
                        .position(|&b| b == 0)
                        .ok_or("Unterminated ADS resource")?
                        + p;
                    envs.insert(env, String::from_utf8_lossy(&d[p..end]).to_uppercase());
                    p = end + 1;
                }
            }
            if &id == b"SCR:" {
                stream = Some(decompress(d)?);
            }
        }
        let data = stream.ok_or("Missing ADS SCR")?;
        let mut p = 0;
        let mut segments = BTreeMap::new();
        let mut segment_order = Vec::new();
        while p < data.len() {
            let id = word(&data, &mut p)?;
            let mut ops = Vec::new();
            while p < data.len() {
                let code = word(&data, &mut p)? as u16;
                if code == 0xffff {
                    break;
                }
                let mut args = Vec::new();
                for _ in 0..nargs(code) {
                    args.push(word(&data, &mut p)?);
                }
                ops.push(Op { code, args });
            }
            segments.insert(id, ops);
            segment_order.push(id);
        }
        Ok(Self {
            name: name.into(),
            envs,
            segments,
            segment_order,
        })
    }
}

struct Machine<'a> {
    script: &'a Script,
    clips: &'a BTreeMap<String, Clip>,
    actors: BTreeMap<(i16, i16), Actor>,
    active: BTreeMap<i16, bool>,
    order: Vec<(i16, i16)>,
    resume: BTreeMap<i16, usize>,
    random: u64,
    tick: u32,
}
impl Machine<'_> {
    fn condition(&self, op: &Op) -> bool {
        let state = op
            .args
            .first()
            .zip(op.args.get(1))
            .and_then(|(&e, &s)| self.actors.get(&(e, s)));
        match op.code & 0xfff0 {
            0x1330 | 0x1030 => state.is_none_or(|a| !a.played),
            0x1340 | 0x1040 => state.is_some_and(|a| a.played),
            0x1350 | 0x1050 => state.is_some_and(|a| a.finished),
            0x1360 | 0x1060 => state.is_none_or(|a| !a.running && !a.paused),
            0x1370 | 0x1070 => state.is_some_and(|a| a.running),
            0x1310 | 0x1010 => state.is_some_and(|a| a.paused),
            0x1320 | 0x1020 => state.is_none_or(|a| !a.paused),
            0x1380 => op.args[0] >= 1,
            0x1390 => op.args[0] <= 1,
            _ => false,
        }
    }
    fn apply(&mut self, op: &Op, segment: i16) {
        let a = &op.args;
        match op.code {
            0x2000 | 0x2005 => {
                let key = (a[0], a[1]);
                if let Some(clip) = self.script.envs.get(&a[0]).and_then(|n| self.clips.get(n)) {
                    if let Some(&start) = clip.labels.get(&a[1]) {
                        let actor = self
                            .actors
                            .entry(key)
                            .or_insert_with(|| Actor::fresh(start));
                        if op.code == 0x2000 {
                            actor.frame = start;
                            actor.due = self.tick;
                            actor.finish_next = false;
                        }
                        actor.played = true;
                        actor.running = true;
                        actor.paused = false;
                        actor.finished = false;
                        actor.repeats = if a[2] > 0 { a[2] as u32 - 1 } else { 0 };
                        actor.cut = if a[2] < 0 {
                            Some(self.tick + (-(a[2] as i32)) as u32)
                        } else {
                            None
                        };
                    }
                }
            }
            0x2010 | 0x2015 => {
                if let Some(actor) = self.actors.get_mut(&(a[0], a[1])) {
                    actor.running = false;
                    actor.paused = op.code == 0x2015;
                }
            }
            0x4000 | 0x4010 => {
                let key = (a[0], a[1]);
                if let Some(at) = self.order.iter().position(|&k| k == key) {
                    self.order.remove(at);
                    if op.code == 0x4000 {
                        self.order.push(key);
                    } else {
                        self.order.insert(0, key);
                    }
                }
            }
            0x2020 => {
                if let Some(actor) = self.actors.get_mut(&(a[0], a[1])) {
                    let start = actor.start;
                    *actor = Actor::fresh(start);
                }
            }
            0xf000 => {
                self.active.insert(segment, false);
            }
            0xf010 => {
                let target = if a[0] < 0 { segment } else { a[0] };
                self.active.insert(target, false);
            }
            0xf200 | 0xf210 => {
                self.active.insert(a[0], true);
            }
            _ => {}
        }
    }
    fn eval(&mut self, ops: &[Op], segment: i16) {
        // Conditions in ADS are block structured. AND/OR chain the immediately
        // preceding tests rather than introducing nested blocks.
        let mut stack = Vec::<(bool, Option<usize>)>::new();
        let mut enabled = true;
        let mut p = self.resume.remove(&segment).unwrap_or(0);
        while p < ops.len() {
            let op = &ops[p];
            if (0x1300..0x1400).contains(&op.code) || (0x1010..=0x1070).contains(&op.code) {
                let condition_start = p;
                let mut test = self.condition(op);
                while p + 2 < ops.len() && matches!(ops[p + 1].code, 0x1420 | 0x1430) {
                    let other = self.condition(&ops[p + 2]);
                    test = if ops[p + 1].code == 0x1420 {
                        test && other
                    } else {
                        test || other
                    };
                    p += 2;
                }
                let waiting = (op.code < 0x1300 && enabled && test).then_some(condition_start);
                stack.push((enabled, waiting));
                enabled = enabled && test;
            } else if op.code == 0x1510 || op.code == 0x1520 {
                let (parent, waiting) = stack.pop().unwrap_or((true, None));
                if op.code == 0x1520 {
                    if let Some(at) = waiting {
                        self.resume.insert(segment, at);
                        return;
                    }
                }
                enabled = parent;
            } else if op.code == 0x1500 {
                enabled = stack.last().map_or(true, |&(parent, _)| parent) && !enabled;
            } else if op.code == 0x3010 {
                let start = p + 1;
                let mut end = start;
                while end < ops.len() && ops[end].code != 0x30ff {
                    end += 1;
                }
                if enabled {
                    let total: u32 = ops[start..end]
                        .iter()
                        .map(|o| o.args.last().copied().unwrap_or(0).max(0) as u32)
                        .sum();
                    if total > 0 {
                        self.random ^= self.random << 13;
                        self.random ^= self.random >> 7;
                        self.random ^= self.random << 17;
                        let mut pick = (self.random % total as u64) as i32;
                        for choice in &ops[start..end] {
                            pick -= choice.args.last().copied().unwrap_or(0).max(0) as i32;
                            if pick < 0 {
                                self.apply(choice, segment);
                                break;
                            }
                        }
                    }
                }
                p = end;
            } else if enabled {
                self.apply(op, segment);
                if op.code == 0xf000
                    || (op.code == 0xf010 && (op.args[0] < 0 || op.args[0] == segment))
                {
                    self.resume.remove(&segment);
                    return;
                }
            }
            p += 1;
        }
    }
}
fn command(opcode: u16, args: &[i16]) -> Command {
    Command {
        opcode,
        args: args.to_vec(),
        text: String::new(),
    }
}
fn finish_due(actor: &mut Actor, tick: u32) {
    if !actor.running {
        return;
    }
    if actor.cut.is_some_and(|cut| tick >= cut) {
        actor.running = false;
        actor.finished = true;
        actor.finish_next = false;
        actor.due = tick;
    } else if actor.finish_next && tick >= actor.due {
        // Time-limited actors repeat on PURGE until their cut, too.
        if actor.repeats > 0 || actor.cut.is_some() {
            actor.repeats = actor.repeats.saturating_sub(1);
            actor.frame = actor.start;
            actor.finish_next = false;
        } else {
            actor.running = false;
            actor.finished = true;
            actor.frame = actor.start;
            actor.finish_next = false;
            actor.due = tick;
            actor.interval = 1;
        }
    }
}

fn actor_context(actor: &Actor, env: i16) -> Vec<Command> {
    vec![
        command(0xffe0, &[env]),
        command(0x4000, &actor.clip),
        command(0x2000, &actor.colors),
        command(0x1050, &[actor.bmp + env * 64]),
        command(0x1120, &[actor.getput + env * 64]),
        command(0x1030, &[actor.brush]),
    ]
}
fn drawing(op: u16) -> bool {
    // Native A050/A060 capture/restore and the presentation commands are one-shot.
    // Held actors repaint their drawing only, without recapturing scene buffers.
    ((0xa000..=0xa530).contains(&op) && !(0xa010..=0xa090).contains(&op))
        || matches!(op, 0x2000 | 0x4000 | 0x1030 | 0x1050)
}
fn remap(c: &Command, env: i16, actor: &mut Actor) -> Command {
    let mut c = c.clone();
    match c.opcode {
        0x1050 => {
            actor.bmp = c.args[0];
            c.args[0] += env * 64;
        }
        0x1120 => {
            actor.getput = c.args[0];
            c.args[0] += env * 64;
        }
        0xa600 => c.args[0] += env * 64,
        0xa500..=0xa530 => {
            if c.args.len() >= 4 {
                c.args[3] += env * 64;
            }
        }
        0x1030 => actor.brush = c.args[0],
        0x4000 => actor.clip.copy_from_slice(&c.args[..4]),
        0x2000 => actor.colors.copy_from_slice(&c.args[..2]),
        _ => {}
    }
    c
}
fn selected_segment(script: &Script, name: &str) -> Option<i16> {
    // Browser policy: prefer coverage of distinct authored states. Counting raw
    // start instructions biases selection toward repeated idle/random branches,
    // e.g. Gulliver segment 3, hiding the longer story in segment 4.

    script
        .segments
        .iter()
        .filter_map(|(&id, ops)| {
            let score = ops
                .iter()
                .filter(|op| {
                    matches!(op.code, 0x2000 | 0x2005)
                        && script.envs.get(&op.args[0]).is_some_and(|n| n == name)
                })
                .map(|op| op.args[1])
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            (score > 0).then_some((score, id))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, id)| id)
}

/// Replace raw storage-order clips with ADS-directed timelines. Each generated
/// frame is one original scheduler tick; the frontend chooses wall-clock speed.
pub fn build_scene_clips(assets: &AssetStore, raw: &[Clip]) -> Result<Vec<Clip>, String> {
    build_scene_clips_with_limit(assets, raw, 2400)
}

/// HD-2D story choreography uses a uniform 60 Hz clock, including standalone
/// helper scripts whose local frames hold for multiple ticks. Expand holds
/// without repeating draw, sound, buffer or transition commands.
pub fn build_hd2d_clips(assets: &AssetStore, raw: &[Clip]) -> Result<Vec<Clip>, String> {
    let mut clips = build_scene_clips_with_limit(assets, raw, 14400)?;
    for clip in &mut clips {
        let mut expanded = Vec::new();
        for mut frame in std::mem::take(&mut clip.frames) {
            let delay = frame.delay_ticks.max(1);
            frame.delay_ticks = 1;
            expanded.push(frame);
            for _ in 1..delay {
                expanded.push(Frame {
                    commands: vec![command(crate::animation::PRESENTATION_HOLD, &[])],
                    delay_ticks: 1,
                });
            }
        }
        clip.frames = expanded;
        clip.labels.clear();
    }
    Ok(clips)
}

/// Number of source frames to replay to display a scheduler tick. A long local
/// hold and its expanded HD-2D presentation refer to the same point in time.
pub fn frames_at_tick(clip: &Clip, tick: u32) -> usize {
    let mut end = 0u32;
    for (i, frame) in clip.frames.iter().enumerate() {
        end = end.saturating_add(frame.delay_ticks.max(1));
        if tick < end {
            return i + 1;
        }
    }
    clip.frames.len()
}

/// Review complete stories beyond the legacy browser's forty-second cap.
/// A finite ceiling remains necessary for intentional ambient loops.
pub fn build_scene_clips_with_limit(
    assets: &AssetStore,
    raw: &[Clip],
    max_ticks: u32,
) -> Result<Vec<Clip>, String> {
    let clips: BTreeMap<_, _> = raw.iter().map(|c| (c.name.clone(), c.clone())).collect();
    let mut scripts = Vec::new();
    for (name, bytes) in &assets.raw {
        if name.ends_with(".ADS") {
            scripts.push(Script::parse(name, bytes).map_err(|e| format!("{name}: {e}"))?);
        }
    }
    let mut result = Vec::new();
    for raw in raw {
        let found = scripts
            .iter()
            .find_map(|s| selected_segment(s, &raw.name).map(|segment| (s, segment)));
        if let Some((script, segment)) = found {
            result.push(compile_with_limit(
                script, segment, &raw.name, &clips, max_ticks,
            ));
        } else {
            result.push(compile_local(raw));
        }
    }
    Ok(result)
}
#[cfg(test)]
fn compile(script: &Script, segment: i16, name: &str, clips: &BTreeMap<String, Clip>) -> Clip {
    compile_with_limit(script, segment, name, clips, 2400)
}
fn compile_with_limit(
    script: &Script,
    segment: i16,
    name: &str,
    clips: &BTreeMap<String, Clip>,
    max_ticks: u32,
) -> Clip {
    let mut order = Vec::new();
    for (&env, name) in &script.envs {
        if let Some(clip) = clips.get(name) {
            let mut labels: Vec<_> = clip
                .labels
                .iter()
                .map(|(&seq, &start)| (seq, start))
                .collect();
            labels.sort_by_key(|(_, start)| *start);
            order.extend(labels.into_iter().map(|(seq, _)| (env, seq)));
        }
    }
    let mut machine = Machine {
        script,
        clips,
        actors: BTreeMap::new(),
        active: BTreeMap::from([(segment, true)]),
        order,
        resume: BTreeMap::new(),
        random: 0x736372616e746963,
        tick: 0,
    };
    let mut frames = Vec::new();
    let mut initial = Vec::new();
    for (&env, n) in &script.envs {
        if let Some(c) = clips.get(n) {
            let first = c.labels.values().copied().min().unwrap_or(0);
            let mut context = Actor::fresh(0);
            initial.push(command(0x1050, &[env * 64]));
            initial.push(command(0x1120, &[env * 64]));
            for f in &c.frames[..first] {
                for cmd in &f.commands {
                    initial.push(remap(cmd, env, &mut context));
                }
            }
        }
    }
    // Keep the requested scene's own setup last if another environment supplied
    // a different screen during archive initialization.
    if let Some(c) = clips.get(name) {
        for f in c
            .frames
            .iter()
            .take(c.labels.values().copied().min().unwrap_or(0))
        {
            for cmd in &f.commands {
                if cmd.opcode == 0xf010 {
                    initial.push(cmd.clone());
                }
            }
        }
    }
    for tick in 0..max_ticks {
        machine.tick = tick;
        // FINISHED is visible to this tick's ADS tests. A normal PURGE
        // completion rewinds; a timed cut preserves the sequence position.
        for actor in machine.actors.values_mut() {
            finish_due(actor, tick);
        }
        for &id in &script.segment_order {
            if machine.active.get(&id) == Some(&true) {
                if let Some(ops) = script.segments.get(&id) {
                    machine.eval(ops, id);
                }
            }
        }
        let mut commands = if tick == 0 {
            std::mem::take(&mut initial)
        } else {
            Vec::new()
        };
        let mut any = false;
        for (env, seq) in machine.order.clone() {
            let Some(actor) = machine.actors.get_mut(&(env, seq)) else {
                continue;
            };
            if !actor.running && !actor.paused {
                continue;
            }
            any = true;
            let Some(clip) = script.envs.get(&env).and_then(|n| clips.get(n)) else {
                continue;
            };
            let context = actor_context(actor, env);
            commands.extend(context.clone());
            if !actor.paused && tick >= actor.due {
                if let Some(frame) = clip.frames.get(actor.frame) {
                    // A held repaint must start with the state at entry to the
                    // source frame, not state mutations after its earlier draws.
                    actor.drawing = context;
                    let mut next = actor.frame + 1;
                    let mut purge = false;
                    for cmd in &frame.commands {
                        if cmd.opcode == 0x0110 {
                            purge = true;
                            continue;
                        }
                        if cmd.opcode == 0x1200 {
                            if let Some(&target) = clip.labels.get(&cmd.args[0]) {
                                next = target;
                            }
                            continue;
                        }
                        if cmd.opcode == 0x1020 {
                            actor.interval = cmd.args[0].max(1) as u32;
                        }
                        if cmd.opcode == 0x2020 {
                            actor.interval =
                                ((cmd.args[0] as i32 + cmd.args[1] as i32) / 2).max(1) as u32;
                        }
                        let mapped = remap(cmd, env, actor);
                        if drawing(mapped.opcode) {
                            actor.drawing.push(mapped.clone());
                        }
                        commands.push(mapped);
                    }
                    actor.due = tick + actor.interval;
                    actor.frame = next;
                    actor.finish_next = purge || clip.frames.get(next).is_none();
                } else {
                    actor.running = false;
                    actor.finished = true;
                }
            } else {
                commands.extend(actor.drawing.clone());
            }
        }
        frames.push(Frame {
            commands,
            delay_ticks: 1,
        });
        // FINISHED is a one-tick event. NOT_PLAYED persists for the lifetime of a segment.
        for actor in machine.actors.values_mut() {
            if actor.finished && !actor.running {
                actor.finished = false;
            }
        }
        if tick > 1 && !any && machine.active.values().all(|&a| !a) {
            break;
        }
    }
    let _source = &script.name;
    Clip {
        name: name.into(),
        frames,
        labels: BTreeMap::new(),
    }
}

fn compile_local(raw: &Clip) -> Clip {
    // A few sequences are launched directly by SCRANTIC.EXE rather than ADS.
    // Keep each local actor's explicit GOTO and PURGE boundaries, with bounded
    // looping; this is a deterministic scene browser policy, not recovered EXE logic.
    let mut frames = Vec::new();
    let mut visits = BTreeMap::new();
    let mut p = 0;
    while p < raw.frames.len() && frames.len() < 2400 {
        let mut f = raw.frames[p].clone();
        let mut next = p + 1;
        for c in &f.commands {
            if c.opcode == 0x1200 {
                if let Some(&target) = raw.labels.get(&c.args[0]) {
                    let count = visits.entry(target).or_insert(0);
                    *count += 1;
                    if *count <= 3 {
                        next = target;
                    }
                }
            }
        }
        f.commands.retain(|c| c.opcode != 0x1200);
        frames.push(f);
        p = next;
    }
    Clip {
        name: raw.name.clone(),
        frames,
        labels: raw.labels.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hd2d_local_holds_preserve_pixels_and_do_not_repeat_audio() {
        let assets =
            AssetStore::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("original"))
                .unwrap();
        let raw: Vec<_> = assets
            .raw
            .iter()
            .filter(|(n, _)| n.ends_with(".TTM"))
            .map(|(n, b)| Clip::parse(n, b).unwrap())
            .collect();
        let local = build_scene_clips_with_limit(&assets, &raw, 14400).unwrap();
        let hd = build_hd2d_clips(&assets, &raw).unwrap();
        for name in ["FIRE.TTM", "MJCOCO1.TTM"] {
            let a = local.iter().find(|c| c.name == name).unwrap();
            let b = hd.iter().find(|c| c.name == name).unwrap();
            let mut original = crate::animation::Player::new();
            let mut expanded = crate::animation::Player::new();
            while let Some(delay) = original.step(a, &assets) {
                for held in 0..delay.max(1) {
                    assert_eq!(expanded.step(b, &assets), Some(1));
                    assert!(
                        original.pixels == expanded.pixels,
                        "{name}: frame {} hold {held}",
                        original.frame
                    );
                    assert_eq!(original.current_background, expanded.current_background);
                    if held == 0 {
                        assert_eq!(original.sounds, expanded.sounds);
                    } else {
                        assert!(expanded.sounds.is_empty());
                    }
                }
            }
            assert!(expanded.step(b, &assets).is_none());
        }
    }
    #[test]
    fn extended_story_timelines_reach_endings_and_preserve_legacy_prefixes() {
        let assets =
            AssetStore::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("original"))
                .unwrap();
        let raw: Vec<_> = assets
            .raw
            .iter()
            .filter(|(name, _)| name.ends_with(".TTM"))
            .map(|(name, bytes)| Clip::parse(name, bytes).unwrap())
            .collect();
        let legacy = build_scene_clips(&assets, &raw).unwrap();
        let stories = build_scene_clips_with_limit(&assets, &raw, 14400).unwrap();
        for (old, full) in legacy.iter().zip(&stories) {
            assert_eq!(old.name, full.name);
            assert!(full.frames.len() < 14400, "Unfinished story {}", full.name);
            for (a, b) in old.frames.iter().zip(&full.frames) {
                assert_eq!(a.delay_ticks, b.delay_ticks);
                assert_eq!(a.commands.len(), b.commands.len());
                for (x, y) in a.commands.iter().zip(&b.commands) {
                    assert_eq!((x.opcode, &x.args, &x.text), (y.opcode, &y.args, &y.text));
                }
            }
        }
        for (name, ticks) in [
            ("MJFIRE.TTM", 3297),
            ("GJDIVE.TTM", 3172),
            ("SASKDATE.TTM", 2670),
            ("GJGULIVR.TTM", 2421),
        ] {
            assert_eq!(
                stories
                    .iter()
                    .find(|c| c.name == name)
                    .unwrap()
                    .frames
                    .len(),
                ticks
            );
        }
    }
    fn fixture(looping: bool) -> (Script, BTreeMap<String, Clip>) {
        let first = Frame {
            commands: vec![
                command(0x1110, &[1]),
                command(0xa000, &[1, 1]),
                command(
                    if looping { 0x1200 } else { 0x0110 },
                    if looping { &[1] } else { &[] },
                ),
            ],
            delay_ticks: 1,
        };
        let second = Frame {
            commands: vec![
                command(0x1110, &[2]),
                command(0xa000, &[2, 2]),
                command(0x0110, &[]),
            ],
            delay_ticks: 1,
        };
        let clip = Clip {
            name: "A.TTM".into(),
            frames: vec![first, second],
            labels: BTreeMap::from([(1, 0), (2, 1)]),
        };
        let ops = vec![
            Op {
                code: 0x1330,
                args: vec![1, 1],
            },
            Op {
                code: 0x2005,
                args: vec![1, 1, if looping { -6 } else { 3 }, 1],
            },
            Op {
                code: 0x1510,
                args: vec![],
            },
            Op {
                code: 0x1350,
                args: vec![1, 1],
            },
            Op {
                code: 0x2005,
                args: vec![1, 2, 0, 1],
            },
            Op {
                code: 0xf010,
                args: vec![-1],
            },
            Op {
                code: 0x1510,
                args: vec![],
            },
        ];
        (
            Script {
                name: "TEST.ADS".into(),
                envs: BTreeMap::from([(1, "A.TTM".into())]),
                segments: BTreeMap::from([(1, ops)]),
                segment_order: vec![1],
            },
            BTreeMap::from([("A.TTM".into(), clip)]),
        )
    }
    #[test]
    fn purge_repeats_exactly_requested_then_finished_branch_runs() {
        let (script, clips) = fixture(false);
        let clip = compile(&script, 1, "A.TTM", &clips);
        let xs: Vec<_> = clip
            .frames
            .iter()
            .flat_map(|f| &f.commands)
            .filter(|c| c.opcode == 0xa000)
            .map(|c| c.args[0])
            .collect();
        assert_eq!(xs, vec![1, 1, 1, 2]);
    }
    #[test]
    fn goto_loops_until_timed_cut_then_runs_successor() {
        let (script, clips) = fixture(true);
        let clip = compile(&script, 1, "A.TTM", &clips);
        let xs: Vec<_> = clip
            .frames
            .iter()
            .flat_map(|f| &f.commands)
            .filter(|c| c.opcode == 0xa000)
            .map(|c| c.args[0])
            .collect();
        assert_eq!(xs, vec![1, 1, 1, 1, 1, 1, 2]);
    }
    fn machine<'a>(script: &'a Script, clips: &'a BTreeMap<String, Clip>) -> Machine<'a> {
        Machine {
            script,
            clips,
            actors: BTreeMap::new(),
            active: BTreeMap::new(),
            order: vec![(1, 1), (1, 2)],
            resume: BTreeMap::new(),
            random: 1,
            tick: 0,
        }
    }
    #[test]
    fn timed_cut_repeats_on_purge_until_deadline_and_resume_keeps_position() {
        let (script, clips) = fixture(false);
        let mut actor = Actor::fresh(0);
        actor.running = true;
        actor.frame = 1;
        actor.finish_next = true;
        actor.cut = Some(10);
        actor.due = 2;
        finish_due(&mut actor, 2);
        assert!(actor.running && !actor.finished);
        assert_eq!(actor.frame, 0, "PURGE must repeat before timed cutoff");
        actor.frame = 1;
        finish_due(&mut actor, 10);
        assert!(actor.finished && !actor.running);
        assert_eq!(actor.frame, 1, "timed cut preserves source position");
        let mut m = machine(&script, &clips);
        m.actors.insert((1, 1), actor);
        m.apply(
            &Op {
                code: 0x2005,
                args: vec![1, 1, 0, 1],
            },
            1,
        );
        assert_eq!(m.actors[&(1, 1)].frame, 1, "ADD-without-reset must resume");
    }
    #[test]
    fn normal_completion_rewinds_even_after_finished_event_is_consumed() {
        let (script, clips) = fixture(false);
        let mut actor = Actor::fresh(0);
        actor.running = true;
        actor.frame = 1;
        actor.finish_next = true;
        actor.due = 2;
        finish_due(&mut actor, 2);
        actor.finished = false;
        let mut m = machine(&script, &clips);
        m.actors.insert((1, 1), actor);
        m.apply(
            &Op {
                code: 0x2005,
                args: vec![1, 1, 0, 1],
            },
            1,
        );
        assert_eq!(m.actors[&(1, 1)].frame, 0);
        assert!(!m.actors[&(1, 1)].finish_next);
    }
    #[test]
    fn held_replay_preserves_state_at_frame_entry() {
        let (mut script, mut clips) = fixture(false);
        script.segments.insert(
            1,
            vec![
                Op {
                    code: 0x1330,
                    args: vec![1, 1],
                },
                Op {
                    code: 0x2005,
                    args: vec![1, 1, 0, 1],
                },
                Op {
                    code: 0x1510,
                    args: vec![],
                },
            ],
        );
        clips.get_mut("A.TTM").unwrap().frames[0].commands = vec![
            command(0x1110, &[1]),
            command(0x2000, &[1, 1]),
            command(0xa000, &[3, 4]),
            command(0x2000, &[2, 2]),
            command(0x4000, &[0, 0, 1, 1]),
            command(0x1020, &[3]),
            command(0x0110, &[]),
        ];
        let compiled = compile(&script, 1, "A.TTM", &clips);
        let assets = AssetStore {
            palette: [[0; 3]; 256],
            backgrounds: BTreeMap::new(),
            sprites: BTreeMap::new(),
            raw: BTreeMap::new(),
        };
        let mut player = crate::animation::Player::new();
        for _ in 0..3 {
            player.step(&compiled, &assets);
            assert_eq!(
                player.pixels[4 * 640 + 3],
                1,
                "held repaint inherited late clip/color mutation"
            );
        }
    }
    #[test]
    fn held_actor_redraws_sprite_without_reemitting_wipe_event() {
        let (mut script, mut clips) = fixture(false);
        script.segments.insert(
            1,
            vec![
                Op {
                    code: 0x1330,
                    args: vec![1, 1],
                },
                Op {
                    code: 0x2005,
                    args: vec![1, 1, 0, 1],
                },
                Op {
                    code: 0x1510,
                    args: vec![],
                },
            ],
        );
        let mut load = command(0xf020, &[]);
        load.text = "SPRITE.BMP".into();
        clips.get_mut("A.TTM").unwrap().frames[0].commands = vec![
            command(0x1110, &[1]),
            load,
            command(0xa500, &[3, 4, 0, 0]),
            command(0xa050, &[0, 0, 8, 8]),
            command(0x1020, &[3]),
        ];
        clips.get_mut("A.TTM").unwrap().frames[1].commands = vec![
            command(0x1100, &[2]),
            command(0xa500, &[3, 4, 0, 0]),
            command(0xa050, &[0, 0, 8, 8]),
            command(0x0110, &[]),
        ];
        let compiled = compile(&script, 1, "A.TTM", &clips);
        let assets = AssetStore {
            palette: [[0; 3]; 256],
            backgrounds: BTreeMap::new(),
            sprites: BTreeMap::from([(
                "SPRITE.BMP".into(),
                vec![crate::resources::Image {
                    width: 1,
                    height: 1,
                    pixels: vec![7],
                }],
            )]),
            raw: BTreeMap::new(),
        };
        let mut player = crate::animation::Player::new();
        for i in 0..6 {
            let wipes = compiled.frames[i]
                .commands
                .iter()
                .filter(|c| (0xa010..=0xa090).contains(&c.opcode))
                .count();
            assert_eq!(
                wipes,
                usize::from(i % 3 == 0),
                "held repaint emitted a second timed transition"
            );
            player.step(&compiled, &assets);
            assert_eq!(
                player.pixels[4 * 640 + 3],
                7,
                "held sprite disappeared on scheduler tick {i} after suppressing wipe replay"
            );
            assert_eq!(player.draws.len(), 1);
        }
    }

    #[test]
    fn while_waits_and_resumes_after_running_actor_stops() {
        let (script, clips) = fixture(false);
        let mut m = machine(&script, &clips);
        let mut actor = Actor::fresh(0);
        actor.running = true;
        m.actors.insert((1, 1), actor);
        let ops = vec![
            Op {
                code: 0x1070,
                args: vec![1, 1],
            },
            Op {
                code: 0x1520,
                args: vec![],
            },
            Op {
                code: 0x2005,
                args: vec![1, 2, 0, 1],
            },
        ];
        m.eval(&ops, 1);
        assert!(
            !m.actors.contains_key(&(1, 2)),
            "WHILE incorrectly fell through in same tick"
        );
        assert_eq!(m.resume[&1], 0);
        m.actors.get_mut(&(1, 1)).unwrap().running = false;
        m.eval(&ops, 1);
        assert!(m.actors[&(1, 2)].running);
        assert!(!m.resume.contains_key(&1));
    }
    #[test]
    fn layer_operations_match_reference_draw_order_and_self_stop_ends_segment() {
        let (script, clips) = fixture(false);
        let mut m = machine(&script, &clips);
        m.apply(
            &Op {
                code: 0x4000,
                args: vec![1, 1, 1],
            },
            1,
        );
        assert_eq!(m.order, vec![(1, 2), (1, 1)]);
        m.apply(
            &Op {
                code: 0x4010,
                args: vec![1, 1, 1],
            },
            1,
        );
        assert_eq!(m.order, vec![(1, 1), (1, 2)]);
        m.eval(
            &[
                Op {
                    code: 0xf010,
                    args: vec![-1],
                },
                Op {
                    code: 0x2005,
                    args: vec![1, 1, 0, 1],
                },
            ],
            1,
        );
        assert!(
            !m.actors.contains_key(&(1, 1)),
            "self-stop must end rule evaluation immediately"
        );
    }

    #[test]
    fn real_archive_ads_all_scenes_have_drawable_scheduled_frames() {
        let assets = AssetStore::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("original")
                .as_path(),
        )
        .unwrap();
        let activity = Script::parse("ACTIVITY.ADS", &assets.raw["ACTIVITY.ADS"]).unwrap();
        assert_eq!(&activity.segment_order[..3], &[1, 12, 11]);
        let raw: Vec<_> = assets
            .raw
            .iter()
            .filter(|(n, _)| n.ends_with(".TTM"))
            .map(|(n, b)| Clip::parse(n, b).unwrap())
            .collect();
        let compiled = build_scene_clips(&assets, &raw).unwrap();
        assert_eq!(compiled.len(), 41);
        for clip in &compiled {
            assert!(!clip.frames.is_empty());
            assert!(
                clip.frames
                    .iter()
                    .flat_map(|f| &f.commands)
                    .any(|c| matches!(c.opcode, 0xa500 | 0xa520)),
                "{}",
                clip.name
            );
        }
        let building = Script::parse("BUILDING.ADS", &assets.raw["BUILDING.ADS"]).unwrap();
        assert_eq!(selected_segment(&building, "GJGULIVR.TTM"), Some(4));
        let gulliver = compiled.iter().find(|c| c.name == "GJGULIVR.TTM").unwrap();
        let wipes: Vec<_> = gulliver
            .frames
            .iter()
            .flat_map(|f| &f.commands)
            .filter(|c| (0xa010..=0xa090).contains(&c.opcode))
            .map(|c| c.opcode)
            .collect();
        assert_eq!(
            wipes,
            vec![0xa050, 0xa060],
            "scene browser hid the authored Gulliver story transitions"
        );
        let jog = compiled.iter().find(|c| c.name == "MJJOG.TTM").unwrap();
        // ADS repeats the jogging and dressing sequences, expanding beyond the
        // raw storage order and following explicit completion dependencies.
        assert!(jog.frames.len() > 1000);
    }
}
