use ropey::Rope;

pub struct Buffer {
    #[allow(dead_code)]
    rope: Rope,
}

impl Buffer {
    pub fn new() -> Self {
        Self { rope: Rope::new() }
    }

    pub fn with_text(text: &str) -> Self {
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
