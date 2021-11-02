pub const VIEW_WIDTH: usize = 256;

pub const VIEW_HEIGHT_NTSC: usize = 224;
pub const VIEW_HEIGHT_PAL: usize = 239;

pub const FB_WIDTH: usize = VIEW_WIDTH << 1;
pub const FB_HEIGHT: usize = VIEW_HEIGHT_PAL << 1;

#[repr(C, align(64))]
#[derive(Clone)]
pub struct Framebuffer(pub [u32; FB_WIDTH * FB_HEIGHT]);

unsafe impl utils::Zero for Framebuffer {}
