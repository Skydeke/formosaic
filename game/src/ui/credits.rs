use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use formosaic_engine::input::{Event, Key};
use imgui::*;
use crate::formosaic::UiState;

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let credits = UiNode::new("credits", move |ui, w, h, _ctx| {
        let s = state.borrow();
        if s.show_menu || !s.is_solved { return; }
        let Some(level) = &s.current_level else { return; };
        let level_name = level.name.clone();
        let level_author = level.author.clone();
        let level_license = level.license.clone();
        let level_source = level.source_url.clone();
        drop(s);
        let mut open_link = false;
        let mut go_menu = false;
        let _win_bg = ui.push_style_color(imgui::StyleColor::WindowBg, [0.03, 0.04, 0.06, 0.82]);
        ui.window("##credits")
            .flags(WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE | WindowFlags::NO_SAVED_SETTINGS)
            .position([w * 0.5, h * 0.5], Condition::Always)
            .position_pivot([0.5, 0.5])
            .size([
                (w - 48.0).clamp(320.0, 520.0),
                124.0,
            ], Condition::Always)
            .build(|| {
                ui.text_colored([0.85, 0.62, 0.18, 1.0], "Level Complete");
                ui.text_colored([0.88, 0.90, 0.96, 1.0], format!("{} by {}", level_name, level_author));
                ui.text_colored([0.50, 0.56, 0.68, 0.9], level_license);
                if ui.button("Open Artist Link") {
                    open_link = true;
                }
                ui.same_line_with_spacing(0.0, 12.0);
                if ui.button("Back to Main Menu") {
                    go_menu = true;
                }
            });
        drop(_win_bg);
        if open_link {
            state.borrow_mut().open_url = Some(level_source);
        }
        if go_menu {
            state.borrow_mut().queued_events.push(Event::KeyDown { key: Key::Escape });
        }
    });
    scene.add_node(Rc::new(RefCell::new(credits)));
}
