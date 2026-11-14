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
}
