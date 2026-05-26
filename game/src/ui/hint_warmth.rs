use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use imgui::*;
use crate::formosaic::UiState;
use crate::puzzle::hints::HintTier;
use super::util;

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let hints_node = UiNode::new("hint_warmth", move |ui, _w, h, _ctx| {
        let s = state.borrow();
        if s.show_menu { return; }
        if s.hint_tier == HintTier::None { return; }
        ui.window("##hints")
            .flags(util::hud_flags())
            .position([10.0, h - 70.0], Condition::Always)
            .size([180.0, 60.0], Condition::Always)
            .build(|| {
                let w = s.hint_warmth;
                let cold = [0.2_f32, 0.4, 0.9, 1.0];
                let hot  = [0.9_f32, 0.2, 0.1, 1.0];
                let col  = [cold[0]+(hot[0]-cold[0])*w, cold[1]+(hot[1]-cold[1])*w,
                            cold[2]+(hot[2]-cold[2])*w, 1.0];
                let label = if w > 0.8 { "HOT" } else if w > 0.5 { "WARM" } else { "COLD" };
                ui.text_colored(col, format!(">> {} <<", label));
                ui.text_colored([0.5, 0.5, 0.5, 0.7], format!("Hint: {:?}", s.hint_tier));
            });
    });
    scene.add_node(Rc::new(RefCell::new(hints_node)));
}
