use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use formosaic_engine::input::{Event, Key};
use imgui::*;
use crate::formosaic::UiState;

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let touch = UiNode::new("touch_buttons", move |ui, w, h, _ctx| {
        let s = state.borrow();
        if s.show_menu { return; }
        let btn_h  = (h * 0.08).max(52.0);
        let margin = (w * 0.02).max(8.0);
        let btn_w  = (w - margin * 3.0) * 0.5;
        let win_y  = h - btn_h - 80.0;
        let win_w  = w;
        let win_h  = btn_h;

        let mut hint_clicked = false;
        let mut menu_clicked = false;
        ui.window("##touch_btns")
            .flags(WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE
                 | WindowFlags::NO_BACKGROUND | WindowFlags::NO_SAVED_SETTINGS)
            .position([0.0, win_y], Condition::Always)
            .size([win_w, win_h], Condition::Always)
            .build(|| {
                ui.set_cursor_pos([margin, 0.0]);
                hint_clicked = ui.button_with_size("HINT", [btn_w, btn_h]);
                ui.same_line_with_spacing(0.0, margin);
                menu_clicked = ui.button_with_size("Menu", [btn_w, btn_h]);
            });
        drop(s);
        if hint_clicked { state.borrow_mut().queued_events.push(Event::KeyDown { key: Key::H }); }
        if menu_clicked { state.borrow_mut().queued_events.push(Event::KeyDown { key: Key::Escape }); }
    });
    scene.add_node(Rc::new(RefCell::new(touch)));
}
