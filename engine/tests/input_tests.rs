use formosaic_engine::input::{Event, Key};

#[test]
fn key_enum_variants_exist() {
    let keys = [Key::Escape, Key::H, Key::R, Key::N, Key::K, Key::L, Key::Space, Key::Other];
    for key in keys {
        let _ = format!("{:?}", key);
    }
}

#[test]
fn event_mouse_down_contains_fields() {
    let ev = Event::MouseDown {
        x: 100.0,
        y: 200.0,
        width: 800.0,
        height: 600.0,
    };
    match ev {
        Event::MouseDown { x, y, width, height } => {
            assert_eq!(x, 100.0);
            assert_eq!(y, 200.0);
            assert_eq!(width, 800.0);
            assert_eq!(height, 600.0);
        }
        _ => panic!("expected MouseDown"),
    }
}

#[test]
fn event_mouse_up_contains_fields() {
    let ev = Event::MouseUp {
        x: 100.0,
        y: 200.0,
        width: 800.0,
        height: 600.0,
    };
    match ev {
        Event::MouseUp { x, y: _, width: _, height: _ } => {
            assert_eq!(x, 100.0);
        }
        _ => panic!("expected MouseUp"),
    }
}

#[test]
fn event_mouse_move_contains_fields() {
    let ev = Event::MouseMove {
        x: 150.0,
        y: 250.0,
        width: 800.0,
        height: 600.0,
    };
    match ev {
        Event::MouseMove { x, y: _, width: _, height: _ } => {
            assert_eq!(x, 150.0);
        }
        _ => panic!("expected MouseMove"),
    }
}

#[test]
fn event_touch_down_contains_id() {
    let ev = Event::TouchDown {
        id: 42,
        x: 300.0,
        y: 200.0,
        width: 800.0,
        height: 600.0,
    };
    match ev {
        Event::TouchDown { id, x, y: _, width: _, height: _ } => {
            assert_eq!(id, 42);
            assert_eq!(x, 300.0);
        }
        _ => panic!("expected TouchDown"),
    }
}

#[test]
fn event_touch_move_contains_id() {
    let ev = Event::TouchMove {
        id: 1,
        x: 310.0,
        y: 210.0,
        width: 800.0,
        height: 600.0,
    };
    match ev {
        Event::TouchMove { id, .. } => {
            assert_eq!(id, 1);
        }
        _ => panic!("expected TouchMove"),
    }
}

#[test]
fn event_touch_up_contains_id() {
    let ev = Event::TouchUp { id: 1 };
    match ev {
        Event::TouchUp { id } => {
            assert_eq!(id, 1);
        }
        _ => panic!("expected TouchUp"),
    }
}

#[test]
fn event_key_down_contains_key() {
    let ev = Event::KeyDown { key: Key::Escape };
    match ev {
        Event::KeyDown { key } => {
            assert!(matches!(key, Key::Escape));
        }
        _ => panic!("expected KeyDown"),
    }
}

#[test]
fn event_quit_is_variant() {
    let ev = Event::Quit;
    assert!(matches!(ev, Event::Quit));
}

#[test]
fn event_clone_and_copy_work() {
    let ev = Event::MouseDown {
        x: 100.0,
        y: 200.0,
        width: 800.0,
        height: 600.0,
    };
    let ev2 = ev;
    let ev3 = ev;
    assert!(matches!(ev2, Event::MouseDown { .. }));
    assert!(matches!(ev3, Event::MouseDown { .. }));
}

#[test]
fn event_debug_repr_contains_type() {
    let ev = Event::KeyDown { key: Key::H };
    let repr = format!("{:?}", ev);
    assert!(repr.contains("KeyDown"));
    assert!(repr.contains("H"));
}

#[test]
fn key_debug_repr() {
    assert!(format!("{:?}", Key::Space).contains("Space"));
    assert!(format!("{:?}", Key::Other).contains("Other"));
}
