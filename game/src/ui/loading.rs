use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use imgui::*;
use crate::formosaic::UiState;

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let loading = UiNode::new("loading", move |ui, w, h, _ctx| {
        let s = state.borrow();
        if !s.is_loading { return; }
        let _win_bg = ui.push_style_color(imgui::StyleColor::WindowBg, [0.03, 0.04, 0.06, 0.82]);
        ui.window("##loading")
            .flags(WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE | WindowFlags::NO_SAVED_SETTINGS | WindowFlags::NO_NAV | WindowFlags::NO_INPUTS)
            .position([w * 0.5, h * 0.5], Condition::Always)
            .position_pivot([0.5, 0.5])
            .size([360.0, 120.0], Condition::Always)
            .focused(true)
            .build(|| {
                ui.text_colored([0.85, 0.62, 0.18, 1.0], "Loading level...");
                ui.set_cursor_pos([18.0, 42.0]);
                let p = s.download_progress.unwrap_or(0.0);
                let label = if p > 0.0 { format!("{:.0}%", p * 100.0) } else { "Loading...".to_string() };
                imgui::ProgressBar::new(p).size([320.0, 18.0]).overlay_text(&label).build(ui);
            });
        drop(_win_bg);
    });
    scene.add_node(Rc::new(RefCell::new(loading)));
}
