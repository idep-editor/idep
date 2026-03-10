use ropey::Rope;

pub struct Buffer {
    rope: Rope,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
        }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}
