#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl WebColor {
    pub const fn parse(text: &str) -> Self {
        let bytes = text.as_bytes();
        if (bytes.len() != 7 && bytes.len() != 9) || bytes[0] != b'#' {
            panic!("web color must be #rrggbb or #rrggbbaa");
        }
        let red = hex_byte(bytes[1], bytes[2]);
        let green = hex_byte(bytes[3], bytes[4]);
        let blue = hex_byte(bytes[5], bytes[6]);
        let alpha = if bytes.len() == 9 {
            hex_byte(bytes[7], bytes[8])
        } else {
            255
        };
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub const fn rgba(self) -> (u8, u8, u8, u8) {
        (self.red, self.green, self.blue, self.alpha)
    }

    pub fn css(self) -> String {
        if self.alpha == 255 {
            format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
        } else {
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                self.red, self.green, self.blue, self.alpha
            )
        }
    }

    pub fn over(self, background: Self) -> Self {
        if self.alpha == 255 {
            return Self { alpha: 255, ..self };
        }
        let alpha = f64::from(self.alpha) / 255.0;
        let mix = |front: u8, back: u8| -> u8 {
            (f64::from(front) * alpha + f64::from(back) * (1.0 - alpha)).round() as u8
        };
        Self {
            red: mix(self.red, background.red),
            green: mix(self.green, background.green),
            blue: mix(self.blue, background.blue),
            alpha: 255,
        }
    }

    pub fn luminance(self) -> f64 {
        fn channel(value: u8) -> f64 {
            let unit = f64::from(value) / 255.0;
            if unit <= 0.04045 {
                unit / 12.92
            } else {
                ((unit + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.red) + 0.7152 * channel(self.green) + 0.0722 * channel(self.blue)
    }

    pub fn contrast_with(self, other: Self) -> f64 {
        let left = self.luminance();
        let right = other.luminance();
        let (lighter, darker) = if left >= right {
            (left, right)
        } else {
            (right, left)
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    pub fn ensure_contrast(self, background: Self) -> Self {
        const MINIMUM: f64 = 4.5;
        let flat = self.over(background);
        if flat.contrast_with(background) >= MINIMUM {
            return flat;
        }
        let target = if background.luminance() >= 0.4 {
            Self {
                red: 0,
                green: 0,
                blue: 0,
                alpha: 255,
            }
        } else {
            Self {
                red: 255,
                green: 255,
                blue: 255,
                alpha: 255,
            }
        };
        let mut low = 0.0;
        let mut high = 1.0;
        let mut best = target;
        for _ in 0..16 {
            let amount = (low + high) / 2.0;
            let mixed = flat.mix(target, amount);
            if mixed.contrast_with(background) >= MINIMUM {
                best = mixed;
                high = amount;
            } else {
                low = amount;
            }
        }
        if best.contrast_with(background) >= MINIMUM {
            best
        } else {
            target
        }
    }

    fn mix(self, other: Self, amount: f64) -> Self {
        let channel = |start: u8, end: u8| -> u8 {
            (f64::from(start) + (f64::from(end) - f64::from(start)) * amount)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Self {
            red: channel(self.red, other.red),
            green: channel(self.green, other.green),
            blue: channel(self.blue, other.blue),
            alpha: 255,
        }
    }
}

impl PartialEq<&str> for WebColor {
    fn eq(&self, other: &&str) -> bool {
        self.css() == *other
    }
}

const fn hex_byte(hi: u8, lo: u8) -> u8 {
    hex_value(hi) * 16 + hex_value(lo)
}

const fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
        _ => panic!("web color has a non-hex digit"),
    }
}

#[macro_export]
macro_rules! webcolor {
    ($hex:literal) => {{ $crate::color::WebColor::parse($hex) }};
}

#[cfg(test)]
mod tests {
    #[test]
    fn webcolor_keeps_hex_and_rgba() {
        let color = webcolor!("#1e1e1e");
        assert_eq!(color.rgba(), (30, 30, 30, 255));
        assert_eq!(color, "#1e1e1e");
        let ink = webcolor!("#fcf8ecbd");
        assert_eq!(ink.rgba(), (0xfc, 0xf8, 0xec, 0xbd));
        assert_eq!(ink.css(), "#fcf8ecbd");
        assert_eq!(webcolor!("#f4f6f7").rgba(), (244, 246, 247, 255));
    }

    #[test]
    fn ensure_contrast_keeps_a_readable_color_and_lifts_a_faint_one() {
        let paper = webcolor!("#ffffff");
        assert_eq!(webcolor!("#ad3da4").ensure_contrast(paper), "#ad3da4");
        let editor = webcolor!("#fcfcfc");
        let operator = webcolor!("#f2a191").ensure_contrast(editor);
        assert!(operator.contrast_with(editor) >= 4.5);
        assert_ne!(operator.css(), "#f2a191");
        let comment = webcolor!("#5c6370").ensure_contrast(webcolor!("#282c34"));
        assert!(comment.contrast_with(webcolor!("#282c34")) >= 4.5);
    }
}
