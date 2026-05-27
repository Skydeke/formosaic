use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use imgui::*;
use crate::formosaic::UiState;
use crate::ui::state_machine::{UiInput, UiScreen};
use super::util::{Scale, self as util};

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let credits = UiNode::new("credits", move |ui, w, h, ctx| {
        let s = state.borrow();
        if s.screen != UiScreen::Credits || !s.is_solved { return; }
        let Some(level) = &s.current_level else { return; };
        let scale = Scale::from_screen(w, h, s.is_touch);
        let level_name = level.name.clone();
        let level_author = level.author.clone();
        let level_license = level.license.clone();
        let level_source = level.source_url.clone();
        drop(s);
        let mut open_link = false;
        let mut go_menu = false;

        let pw = (w * 0.60).clamp(scale.su(320.0), scale.su(480.0));
        let inner_w = pw - scale.pad_w() * 2.0;

        let _wp = ui.push_style_var(imgui::StyleVar::WindowPadding([scale.pad_w(), scale.pad_w()]));
        let _win_bg = ui.push_style_color(imgui::StyleColor::WindowBg, [0.03, 0.04, 0.06, 0.92]);
        ui.window("##credits")
            .flags(util::popup_flags())
            .position([w * 0.5, h * 0.5], Condition::Always)
            .position_pivot([0.5, 0.5])
            .build(|| {
                util::title(ui, "Level Complete");
                util::gap(ui, scale.gap_md());
                util::text_body(ui, &level_name);
                util::text_muted(ui, &format!("by {}", level_author));
                util::gap(ui, scale.gap_xxs());
                util::text_muted(ui, &level_license);
                util::gap(ui, scale.gap_md());
                util::separator_dim(ui);
                util::gap(ui, scale.gap_sm());
                scale.btn_full(ui, "Open Artist Link", inner_w, &mut open_link);
                util::gap(ui, scale.gap_sm());
                scale.btn_full(ui, "Back to Main Menu", inner_w, &mut go_menu);
            });
        drop(_win_bg);
        drop(_wp);
        if open_link { ctx.push_ui_action(UiInput::ArtistLinkPressed(level_source)); }
        if go_menu { ctx.push_ui_action(UiInput::BackToMenuPressed); }
    });
    scene.add_node(Rc::new(RefCell::new(credits)));
}
