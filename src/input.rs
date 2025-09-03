#[derive(Clone, Copy, Debug)]
pub enum Key {
    Escape,
    R,
    Space,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    MouseDown {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    MouseUp {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    MouseMove {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    TouchDown {
        id: u64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    TouchMove {
        id: u64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    TouchUp {
        id: u64,
    },
    KeyDown {
        key: Key,
    },
    Quit,
}
