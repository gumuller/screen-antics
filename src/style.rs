//! The three selectable presentations share the same animation interpreter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtStyle {
    Original,
    Remaster,
    Hd2d,
}
impl ArtStyle {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "original" | "retro" | "1" => Ok(Self::Original),
            "remaster" | "remastered" | "2" => Ok(Self::Remaster),
            "hd2d" | "hd-2d" | "3" => Ok(Self::Hd2d),
            _ => Err(format!(
                "Invalid style {value}; use original, remaster or hd2d"
            )),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Remaster => "Remaster",
            Self::Hd2d => "HD-2D Neo-Retro",
        }
    }
    pub fn cycle(&mut self) {
        *self = match self {
            Self::Original => Self::Remaster,
            Self::Remaster => Self::Hd2d,
            Self::Hd2d => Self::Original,
        };
    }
    pub fn pack_style(self) -> Self {
        if self == Self::Original {
            Self::Remaster
        } else {
            self
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn style_cycle_preserves_all_three_options() {
        let mut style = ArtStyle::Original;
        for expected in [ArtStyle::Remaster, ArtStyle::Hd2d, ArtStyle::Original] {
            style.cycle();
            assert_eq!(style, expected);
        }
        assert_eq!(ArtStyle::parse("3").unwrap(), ArtStyle::Hd2d);
        assert!(ArtStyle::parse("unknown").is_err());
    }
}
