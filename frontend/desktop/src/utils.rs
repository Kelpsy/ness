macro_rules! error {
    (yes_no, $title: expr, $($desc: tt)*) => {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title($title)
            .set_description(&format!($($desc)*))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
    };
    ($title: expr, $($desc: tt)*) => {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title($title)
            .set_description(&format!($($desc)*))
            .set_buttons(rfd::MessageButtons::Ok)
            .show()
    };
}

macro_rules! config_error {
    (yes_no, $($desc: tt)*) => {
        error!(yes_no, "Configuration error", $($desc)*)
    };
    ($($desc: tt)*) => {
        error!("Configuration error", $($desc)*)
    };
}

pub fn scale_to_fit(aspect_ratio: f32, frame_size: [f32; 2]) -> ([f32; 2], [f32; 2]) {
    let width = (frame_size[1] * aspect_ratio).min(frame_size[0]);
    let height = width / aspect_ratio;
    (
        [
            (frame_size[0] - width) * 0.5,
            (frame_size[1] - height) * 0.5,
        ],
        [width, height],
    )
}
