use super::util::{self as util, Scale};
use crate::formosaic::UiState;
use crate::puzzle::entropy::difficulty_label;
use crate::ui::state_machine::UiScreen;
use formosaic_engine::architecture::scene::node::{scenegraph::Scenegraph, ui_node::UiNode};
use imgui::*;
use std::{cell::RefCell, rc::Rc};

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let hud = UiNode::new("hud", move |ui, w, h, _ctx| {
        let s = state.borrow();
        if s.screen != UiScreen::Game {
            return;
        }
        let scale = Scale::from_screen(w, h, s.is_touch);

        ui.window("##hud")
            .flags(util::hud_flags())
            .position(
                [w - scale.su(210.0) - scale.pad_w(), scale.pad_w()],
                Condition::Always,
            )
            .size([scale.su(210.0), scale.su(100.0)], Condition::Always)
            .build(|| {
                if let Some(diff) = s.difficulty {
                    util::text_body(
                        ui,
                        &format!("{:.1}s  |  {}", s.elapsed_secs, difficulty_label(diff)),
                    );
                }
                if s.hint_count > 0 {
                    let label = format!(
                        "{} {}",
                        s.hint_count,
                        if s.hint_count == 1 { "hint" } else { "hints" }
                    );
                    ui.text_colored([0.9, 0.6, 0.2, 0.8], label);
                }
                if s.is_solved {
                    ui.text_colored([0.2, 0.9, 0.5, 1.0], "SOLVED!");
                }
            });

        if !s.is_touch {
            ui.window("##keyinfo")
                .flags(util::hud_flags())
                .position([w * 0.5, h - scale.pad_w()], Condition::Always)
                .position_pivot([0.5, 1.0])
                .build(|| {
                    util::text_muted(ui, "ESC  Menu    H  Hint");
                });
        }
    });
    scene.add_node(Rc::new(RefCell::new(hud)));
}
