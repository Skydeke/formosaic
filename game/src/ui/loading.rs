use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use imgui::*;
use crate::formosaic::UiState;
use super::util::{Scale, self as util};

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let loading = UiNode::new("loading", move |ui, w, h, _ctx| {
        let s = state.borrow();
        if !s.is_loading { return; }
        let scale = Scale::from_screen(w, h, s.is_touch);

        let pw = (w * 0.50).clamp(scale.su(280.0), scale.su(380.0));
        let inner_w = pw - scale.pad_w() * 2.0;
        let bar_w = inner_w;

        let _wp = ui.push_style_var(imgui::StyleVar::WindowPadding([scale.pad_w(), scale.pad_w()]));
        let _win_bg = ui.push_style_color(imgui::StyleColor::WindowBg, [0.03, 0.04, 0.06, 0.92]);
        let _border = ui.push_style_color(imgui::StyleColor::Border, [0.68, 0.48, 0.12, 0.35]);
        ui.window("##loading")
            .flags(util::popup_flags() | WindowFlags::NO_NAV | WindowFlags::NO_INPUTS)
            .position([w * 0.5, h * 0.5], Condition::Always)
            .position_pivot([0.5, 0.5])
            .focused(true)
            .build(|| {
                util::title(ui, "Loading level...");
                util::gap(ui, scale.gap_sm());
                if let Some(p) = s.download_progress {
                    let _pb = ui.push_style_color(imgui::StyleColor::PlotHistogram, [0.68, 0.48, 0.12, 0.85]);
                    imgui::ProgressBar::new(p)
                        .size([bar_w, scale.bar_h()])
                        .overlay_text(&format!("{:.0}%", p * 100.0))
                        .build(ui);
                    drop(_pb);
                } else {
                    util::text_muted(ui, "Preparing model...");
                }
            });
        drop(_border);
        drop(_win_bg);
        drop(_wp);
    });
    scene.add_node(Rc::new(RefCell::new(loading)));
}
