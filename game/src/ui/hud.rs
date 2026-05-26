use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use imgui::*;
use crate::formosaic::UiState;
use crate::puzzle::entropy::difficulty_label;
use super::util;

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let hud = UiNode::new("hud", move |ui, w, _h, _ctx| {
        let s = state.borrow();
        if s.show_menu { return; }
        ui.window("##hud")
            .flags(util::hud_flags())
            .position([w - 220.0, 10.0], Condition::Always)
            .size([210.0, 100.0], Condition::Always)
            .build(|| {
                if let Some(diff) = s.difficulty {
                    ui.text_colored([0.8, 0.85, 0.95, 0.9],
                        format!("{:.1}s  |  {}", s.elapsed_secs, difficulty_label(diff)));
                }
                if s.hint_count > 0 {
                    ui.text_colored([0.9, 0.6, 0.2, 0.8], format!("{} hint(s)", s.hint_count));
                }
                if s.is_solved      { ui.text_colored([0.2, 0.9, 0.5, 1.0], "SOLVED!"); }
                if s.is_downloading { ui.text_colored([0.4, 0.7, 1.0, 0.8], "Fetching..."); }
            });
    });
    scene.add_node(Rc::new(RefCell::new(hud)));
}
