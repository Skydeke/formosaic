pub struct GlUtils;

impl GlUtils {
    pub fn enable_depth_test() {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
        }
    }

    pub fn disable_depth_test() {
        unsafe {
            gl::Disable(gl::DEPTH_TEST);
        }
    }
}
