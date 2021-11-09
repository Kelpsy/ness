use super::Device;

pub struct Empty {}

impl Empty {
    pub fn new() -> Self {
        Empty {}
    }
}

impl Default for Empty {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for Empty {
    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn auto_read(&mut self) -> u16 {
        0
    }
}
