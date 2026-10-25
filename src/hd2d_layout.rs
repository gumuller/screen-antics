//! Scene geometry for the independent HD-2D presentation. Coordinates here refer
//! to painted content (640x350 logical units), not the old TTM framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    Island,
    Underwater,
    Beach,
    Office,
    Plate,
}
#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub kind: SceneKind,
    pub height: f32,
    pub horizon: f32,
    pub original_palm: Point,
    pub palm_root: Point,
    pub palm_crown: Point,
    pub sand_center: Point,
    pub sand_radius: Point,
    pub night: bool,
}
impl Layout {
    pub fn for_background(name: &str) -> Self {
        let shift = if name == "ISLETEMP.SCR" { 272. } else { 0. };
        let mut layout = Self {
            kind: SceneKind::Island,
            height: 350.,
            horizon: 120.,
            original_palm: Point::new(176. + shift, 289.),
            palm_root: Point::new(415., 225.),
            palm_crown: Point::new(469., 62.),
            sand_center: Point::new(335., 247.),
            sand_radius: Point::new(230., 35.),
            night: name == "NIGHT.SCR",
        };
        match name {
            "ISLAND2.SCR" | "ISLETEMP.SCR" | "NIGHT.SCR" => {}
            "OCEAN00.SCR" | "OCEAN01.SCR" | "OCEAN02.SCR" => {
                layout.kind = SceneKind::Underwater;
                layout.horizon = 0.;
            }
            "SUZBEACH.SCR" => {
                layout.kind = SceneKind::Beach;
                layout.height = 480.;
                layout.horizon = 190.;
            }
            "JOFFICE.SCR" => {
                layout.kind = SceneKind::Office;
                layout.horizon = 0.;
            }
            _ => {
                layout.kind = SceneKind::Plate;
                layout.horizon = 0.;
                layout.height = if name == "INTRO.SCR" { 480. } else { 350. };
            }
        }
        layout
    }
    /// Piecewise registration preserves the palm as an interaction anchor while
    /// independently registering the broader painted left/right shoreline.
    pub fn land_x(&self, x: f32) -> f32 {
        if self.kind != SceneKind::Island {
            return x;
        }
        let old = self.original_palm.x;
        if x <= old {
            110. + (x - (old - 156.)) * (self.palm_root.x - 110.) / 156.
        } else {
            self.palm_root.x + (x - old) * (560. - self.palm_root.x) / 137.
        }
    }
    pub fn sand_front(&self, x: f32) -> f32 {
        let t = ((x - self.sand_center.x) / self.sand_radius.x).clamp(-1., 1.);
        self.sand_center.y + self.sand_radius.y * 0.7 * (1. - t * t).sqrt()
    }
    pub fn sand_contains(&self, p: Point) -> bool {
        ((p.x - self.sand_center.x) / self.sand_radius.x).powi(2)
            + ((p.y - self.sand_center.y) / self.sand_radius.y).powi(2)
            <= 1.
    }
    /// Source foot height relative to its original support curve preserves jumps
    /// and crouches. A different crop height never silently scales a character.
    pub fn grounded(&self, foot: Point) -> Point {
        match self.kind {
            SceneKind::Island => {
                let x = self.land_x(foot.x);
                let old_center = self.original_palm.x - 16.;
                let q = ((foot.x - old_center) / 145.).clamp(-1., 1.);
                let old_floor = 304. + 14. * (1. - q * q).sqrt();
                Point::new(x, self.sand_front(x) + (foot.y - old_floor) * 0.85)
            }
            SceneKind::Beach => {
                let x = foot.x * 0.78;
                Point::new(x, 310. + x * 0.38 + (foot.y - 310.) * 0.65)
            }
            SceneKind::Office => Point::new(foot.x, foot.y),
            _ => foot,
        }
    }
    /// Contact on the actual curved palm, rather than the original vertical red
    /// trunk. Feet/hands can share this curve with per-animation offsets.
    pub fn palm_at(&self, t: f32) -> Point {
        let t = t.clamp(0., 1.);
        let u = 1. - t;
        let a = self.palm_root;
        let b = Point::new(428., 186.);
        let c = Point::new(461., 124.);
        let d = self.palm_crown;
        Point::new(
            u * u * u * a.x + 3. * u * u * t * b.x + 3. * u * t * t * c.x + t * t * t * d.x,
            u * u * u * a.y + 3. * u * u * t * b.y + 3. * u * t * t * c.y + t * t * t * d.y,
        )
    }
    pub fn climbing(&self, old_foot: Point) -> Point {
        self.palm_at((self.original_palm.y - old_foot.y) / 166.)
    }
    pub fn sky(&self, p: Point) -> Point {
        Point::new(p.x, p.y * self.horizon.max(80.) / 185.)
    }
    /// Ship waterlines stay in the open strip behind the painted island. Scene
    /// terrain is subsequently composited in front, before land actors.
    pub fn ship_waterline(&self, p: Point) -> Point {
        Point::new(
            p.x,
            self.horizon + ((p.y - 150.) / 160.).clamp(0.4, 1.) * 78.,
        )
    }
    pub fn swimming(&self, p: Point) -> Point {
        if self.kind == SceneKind::Underwater {
            Point::new(
                80. + p.x * 0.75,
                70. + ((p.y - 160.) / 190.).clamp(0., 1.) * 220.,
            )
        } else {
            Point::new(p.x, 290. + ((p.y - 240.) / 110.).clamp(0., 1.) * 50.)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn differently_placed_original_islands_register_to_the_same_painted_palm() {
        let a = Layout::for_background("ISLAND2.SCR");
        let b = Layout::for_background("ISLETEMP.SCR");
        assert!((a.land_x(176.) - 415.).abs() < 0.001);
        assert!((b.land_x(448.) - 415.).abs() < 0.001);
        let left = a.grounded(Point::new(160., 318.));
        let right = b.grounded(Point::new(432., 318.));
        assert!((left.x - right.x).abs() < 0.001 && (left.y - right.y).abs() < 0.001);
        assert!(a.sand_contains(left));
    }
    #[test]
    fn climbing_contacts_follow_the_painted_curve_and_ships_do_not_land_on_sand() {
        let layout = Layout::for_background("ISLETEMP.SCR");
        let bottom = layout.climbing(Point::new(448., 289.));
        let top = layout.climbing(Point::new(448., 123.));
        assert!((bottom.x - 415.).abs() < 0.001 && (bottom.y - 225.).abs() < 0.001);
        assert!((top.x - 469.).abs() < 0.001 && (top.y - 62.).abs() < 0.001);
        let ship = layout.ship_waterline(Point::new(320., 260.));
        assert!(ship.y < layout.sand_center.y - layout.sand_radius.y);
    }
}
