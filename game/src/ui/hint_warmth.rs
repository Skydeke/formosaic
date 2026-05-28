use super::util::{self as util, Scale};
use crate::formosaic::UiState;
use crate::puzzle::hints::HintTier;
use crate::ui::state_machine::UiScreen;
use formosaic_engine::architecture::scene::node::{scenegraph::Scenegraph, ui_node::UiNode};
use imgui::*;
use std::{cell::RefCell, rc::Rc};

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let hints_node = UiNode::new("hint_warmth", move |ui, w, h, _ctx| {
        let s = state.borrow();
        if s.screen != UiScreen::Game {
            return;
        }
        if s.hint_tier == HintTier::None {
            return;
        }
        let scale = Scale::from_screen(w, h, s.is_touch);

        ui.window("##hints")
            .flags(util::hud_flags())
            .position(
                [scale.pad_w(), h - scale.su(60.0) - scale.pad_w()],
                Condition::Always,
            )
            .size([scale.su(180.0), scale.su(60.0)], Condition::Always)
            .build(|| {
                let w = s.hint_warmth;
                let cold = [0.2_f32, 0.4, 0.9, 1.0];
                let hot = [0.9_f32, 0.2, 0.1, 1.0];
                let col = [
                    cold[0] + (hot[0] - cold[0]) * w,
                    cold[1] + (hot[1] - cold[1]) * w,
                    cold[2] + (hot[2] - cold[2]) * w,
                    1.0,
                ];
                let label = if w > 0.8 {
                    "HOT"
                } else if w > 0.5 {
                    "WARM"
                } else {
                    "COLD"
                };
                ui.text_colored(col, format!(">> {} <<", label));
                util::text_muted(ui, &format!("Hint: {}", util::hint_tier_label(s.hint_tier)));
            });
    });
    scene.add_node(Rc::new(RefCell::new(hints_node)));
}
