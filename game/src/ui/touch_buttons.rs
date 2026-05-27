use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use imgui::*;
use crate::formosaic::UiState;
use crate::ui::state_machine::{UiInput, UiScreen};
use super::util::Scale;

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let touch = UiNode::new("touch_buttons", move |ui, w, h, ctx| {
        let s = state.borrow();
        if s.screen != UiScreen::Game { return; }
        let scale = Scale::from_screen(w, h, s.is_touch);
        let btn_h  = (h * 0.08).max(scale.btn_h());
        let margin = scale.pad_w();
        let btn_w  = (w - margin * 3.0) * 0.5;
        let win_y  = h - btn_h - scale.su(80.0);

        let mut hint_clicked = false;
        let mut menu_clicked = false;
        ui.window("##touch_btns")
            .flags(WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE
                 | WindowFlags::NO_BACKGROUND | WindowFlags::NO_SAVED_SETTINGS)
            .position([0.0, win_y], Condition::Always)
            .size([w, btn_h], Condition::Always)
            .build(|| {
                ui.set_cursor_pos([margin, 0.0]);
                hint_clicked = ui.button_with_size("HINT", [btn_w, btn_h]);
                ui.same_line_with_spacing(0.0, margin);
                menu_clicked = ui.button_with_size("Menu", [btn_w, btn_h]);
            });
        drop(s);
        if hint_clicked { ctx.push_ui_action(UiInput::Hint); }
        if menu_clicked { ctx.push_ui_action(UiInput::MenuPressed); }
    });
    scene.add_node(Rc::new(RefCell::new(touch)));
}
