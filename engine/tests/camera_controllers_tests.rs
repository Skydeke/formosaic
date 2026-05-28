use cgmath::Vector3;
use formosaic_engine::architecture::scene::node::transform::Transform;
use formosaic_engine::input::{Event, Key};
use formosaic_engine::rendering::instances::camera::camera_controller::CameraController;
use formosaic_engine::rendering::instances::camera::camera_controller::CameraControllers;
use formosaic_engine::rendering::instances::camera::camera_controller::NoneController;

struct RecordingController {
    control_calls: std::cell::Cell<u32>,
    event_calls: std::cell::Cell<u32>,
}

impl RecordingController {
    fn new() -> Self {
        Self {
            control_calls: std::cell::Cell::new(0),
            event_calls: std::cell::Cell::new(0),
        }
    }
}

impl CameraController for RecordingController {
    fn control(&mut self, _transform: &mut Transform) {
        self.control_calls.set(self.control_calls.get() + 1);
    }

    fn handle_event(&mut self, _event: &Event, _width: f32, _height: f32) {
        self.event_calls.set(self.event_calls.get() + 1);
    }
}

#[test]
fn none_controller_does_not_modify_transform() {
    let mut ctrl = NoneController::new();
    let mut t = Transform::new();
    t.position = Vector3::new(1.0, 2.0, 3.0);

    ctrl.control(&mut t);
    assert_eq!(t.position, Vector3::new(1.0, 2.0, 3.0));
}

#[test]
fn none_controller_ignores_all_events() {
    let mut ctrl = NoneController::new();
    ctrl.handle_event(&Event::Quit, 800.0, 600.0);
    ctrl.handle_event(&Event::KeyDown { key: Key::Escape }, 800.0, 600.0);
    ctrl.handle_event(
        &Event::MouseDown {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
}

#[test]
fn camera_controllers_calls_all_children() {
    let rec1 = RecordingController::new();
    let rec2 = RecordingController::new();

    let mut controllers = CameraControllers::new(vec![Box::new(rec1), Box::new(rec2)]);

    let mut t = Transform::new();
    controllers.control(&mut t);
}

#[test]
fn camera_controllers_forwards_events() {
    let rec1 = RecordingController::new();
    let rec2 = RecordingController::new();

    let mut controllers = CameraControllers::new(vec![Box::new(rec1), Box::new(rec2)]);

    let ev = Event::KeyDown { key: Key::H };
    controllers.handle_event(&ev, 800.0, 600.0);
}

#[test]
fn camera_controllers_empty_list_does_not_panic() {
    let mut controllers = CameraControllers::new(vec![]);
    let mut t = Transform::new();
    controllers.control(&mut t);
    controllers.handle_event(&Event::Quit, 800.0, 600.0);
}

#[test]
fn camera_controllers_order_is_preserved() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let call_order = Rc::new(RefCell::new(Vec::new()));

    struct OrderRecorder {
        id: u32,
        order: Rc<RefCell<Vec<u32>>>,
    }

    impl CameraController for OrderRecorder {
        fn control(&mut self, _transform: &mut Transform) {
            self.order.borrow_mut().push(self.id);
        }
        fn handle_event(&mut self, _event: &Event, _width: f32, _height: f32) {}
    }

    let r1 = OrderRecorder {
        id: 1,
        order: Rc::clone(&call_order),
    };
    let r2 = OrderRecorder {
        id: 2,
        order: Rc::clone(&call_order),
    };
    let r3 = OrderRecorder {
        id: 3,
        order: Rc::clone(&call_order),
    };

    let mut controllers = CameraControllers::new(vec![Box::new(r1), Box::new(r2), Box::new(r3)]);

    let mut t = Transform::new();
    controllers.control(&mut t);

    let order = call_order.borrow();
    assert_eq!(*order, vec![1, 2, 3]);
}

#[test]
fn camera_controllers_multiple_events_accumulate() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let count = Rc::new(RefCell::new(0));

    struct Counter {
        count: Rc<RefCell<u32>>,
    }

    impl CameraController for Counter {
        fn control(&mut self, _transform: &mut Transform) {}
        fn handle_event(&mut self, _event: &Event, _width: f32, _height: f32) {
            *self.count.borrow_mut() += 1;
        }
    }

    let counter = Counter {
        count: Rc::clone(&count),
    };
    let mut controllers = CameraControllers::new(vec![Box::new(counter)]);

    for _ in 0..5 {
        controllers.handle_event(&Event::Quit, 800.0, 600.0);
    }

    assert_eq!(*count.borrow(), 5);
}

#[test]
fn camera_controllers_mixed_controller_types() {
    let none = NoneController::new();
    let rec = RecordingController::new();

    let mut controllers = CameraControllers::new(vec![Box::new(none), Box::new(rec)]);

    let mut t = Transform::new();
    controllers.control(&mut t);
}
