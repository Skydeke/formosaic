use super::util::{self as util, Scale};
use crate::formosaic::UiState;
use crate::ui::state_machine::{UiInput, UiScreen};
use formosaic_engine::architecture::scene::node::{scenegraph::Scenegraph, ui_node::UiNode};
use imgui::*;
use std::{cell::RefCell, rc::Rc};

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let menu = UiNode::new("menu", move |ui, w, h, ctx| {
        let s = state.borrow();
        if s.screen != UiScreen::MainMenu {
            return;
        }
        let scale = Scale::from_screen(w, h, s.is_touch);
        let is_touch = s.is_touch;
        let levels = s.levels.clone();
        let _is_dl = s.is_downloading;
        let _is_loading = s.is_loading;

        let menu_flags = WindowFlags::NO_DECORATION
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_SAVED_SETTINGS
            | WindowFlags::NO_BACKGROUND
            | WindowFlags::NO_SCROLL_WITH_MOUSE;
        let pad = (w * 0.02).max(scale.gap_sm());

        let _wp = ui.push_style_var(imgui::StyleVar::WindowPadding([0.0, 0.0]));
        let _ip = ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0, 0.0]));
        ui.window("##menu")
            .flags(menu_flags)
            .position([0.0, 0.0], Condition::Always)
            .size([w, h], Condition::Always)
            .build(|| {
                let _ip2 = ui.push_style_var(imgui::StyleVar::ItemSpacing([
                    scale.gap_sm(),
                    scale.gap_xs(),
                ]));

                if is_touch {
                    let btn_h = (h * 0.08).max(scale.btn_h());
                    let title_h = (h * 0.05).max(scale.su(32.0));
                    let btns_y = h - btn_h - scale.su(80.0);
                    let list_h = (btns_y - title_h - pad).max(0.0);
                    let row_h = (h * 0.12).max(scale.su(80.0));
                    let m = pad;
                    let row_w = w - m * 2.0;

                    let _tok =
                        ui.push_style_color(imgui::StyleColor::ChildBg, [0.03, 0.04, 0.06, 0.95]);
                    ui.child_window("##hdr")
                        .size([w, title_h])
                        .scroll_bar(false)
                        .border(false)
                        .build(|| {
                            ui.set_cursor_pos([pad, (title_h - 16.0) * 0.5]);
                            ui.text_colored([0.85, 0.62, 0.18, 1.0], "FORMOSAIC");
                        });
                    drop(_tok);

                    let _tok2 =
                        ui.push_style_color(imgui::StyleColor::ChildBg, [0.0, 0.0, 0.0, 0.0]);
                    ui.child_window("##levels")
                        .size([w, list_h])
                        .scroll_bar(false)
                        .border(false)
                        .build(|| {
                            if levels.is_empty() {
                                ui.dummy([1.0, pad]);
                                ui.set_cursor_pos([pad, ui.cursor_pos()[1]]);
                                util::text_body(ui, "No saved levels yet.");
                                ui.set_cursor_pos([pad, ui.cursor_pos()[1] + scale.gap_xs()]);
                                util::text_dim(ui, "Use the buttons below to fetch one.");
                            } else {
                                for level in &levels {
                                    let dc = util::diff_color(level.difficulty);
                                    let _t3 = ui.push_style_color(
                                        imgui::StyleColor::ChildBg,
                                        [0.04, 0.06, 0.09, 0.72],
                                    );
                                    let _t4 = ui.push_style_color(imgui::StyleColor::Border, dc);
                                    ui.set_cursor_pos([m, ui.cursor_pos()[1]]);
                                    ui.child_window(format!("##r_{}", level.id))
                                        .size([row_w, row_h])
                                        .scroll_bar(false)
                                        .border(true)
                                        .build(|| {
                                            let ip = pad * 0.8;
                                            let btn_w = scale.su(100.0);
                                            let btn_h = (row_h - ip * 2.0).max(scale.btn_h());
                                            ui.set_cursor_pos([ip, ip * 0.5]);
                                            util::text_body(ui, &util::truncate(&level.name, 18));
                                            ui.same_line_with_spacing(0.0, ip * 0.5);
                                            ui.text_colored(
                                                dc,
                                                util::diff_label_str(level.difficulty),
                                            );
                                            if let Some(t) = level.best_time_secs {
                                                ui.same_line_with_spacing(0.0, ip * 0.5);
                                                ui.text_colored(
                                                    [0.5, 0.55, 0.65, 0.8],
                                                    format!("{:.1}s", t),
                                                );
                                            }
                                            ui.set_cursor_pos([ip, ip + scale.su(22.0)]);
                                            util::text_dim(ui, &util::truncate(&level.author, 22));
                                            ui.set_cursor_pos([
                                                row_w - btn_w - ip,
                                                (row_h - btn_h) * 0.5,
                                            ]);
                                            if ui.button_with_size("Play", [btn_w, btn_h]) {
                                                ctx.push_ui_action(UiInput::PlayLevel(
                                                    level.id.clone(),
                                                ));
                                            }
                                        });
                                    drop(_t3);
                                    drop(_t4);
                                    ui.dummy([1.0, pad * 0.3]);
                                }
                            }
                        });
                    drop(_tok2);

                    let half = (w - pad * 3.0) * 0.5;
                    ui.set_cursor_pos([pad, btns_y]);
                    let busy = _is_dl || _is_loading;
                    let _dis = ui.begin_disabled(busy);
                    if ui.button_with_size("+ Fetch Online", [half, btn_h]) {
                        ctx.push_ui_action(UiInput::FetchOnline);
                    }
                    ui.same_line_with_spacing(0.0, pad);
                    if ui.button_with_size("Random", [half, btn_h]) {
                        ctx.push_ui_action(UiInput::RandomSaved);
                    }
                    drop(_dis);
                    ui.dummy([0.0, pad]);
                } else {
                    let bar_h = scale.su(28.0);
                    let hdr_h = scale.su(20.0);
                    let row_h = scale.su(24.0);
                    let footer_h = scale.su(16.0);

                    let cx_name = pad;
                    let cx_auth = w * 0.30;
                    let cx_diff = w * 0.52;
                    let cx_best = w * 0.64;
                    let cx_play = w * 0.75;
                    let play_w = w - cx_play - pad;

                    let _tok =
                        ui.push_style_color(imgui::StyleColor::ChildBg, [0.03, 0.04, 0.06, 0.95]);
                    ui.child_window("##bar")
                        .size([w, bar_h])
                        .scroll_bar(false)
                        .border(false)
                        .build(|| {
                            let btn_h_bar = bar_h - scale.gap_xs();
                            let n_w = scale.su(150.0);
                            let r_w = scale.su(100.0);
                            let gap = scale.gap_xs();
                            let ver_w = scale.su(32.0);

                            let r_x = w - ver_w - pad - gap - r_w;
                            let n_x = r_x - gap - n_w;

                            ui.set_cursor_pos([pad, (bar_h - 14.0) * 0.5]);
                            ui.text_colored([0.85, 0.62, 0.18, 1.0], "FORMOSAIC");
                            ui.set_cursor_pos([w - ver_w - pad, (bar_h - 13.0) * 0.5]);
                            ui.text_colored([0.28, 0.34, 0.46, 0.6], "v0.1");
                            ui.set_cursor_pos([r_x, scale.su(2.0)]);
                            let _dis2 = ui.begin_disabled(_is_dl || _is_loading);
                            if ui.button_with_size("[R] Random", [r_w, btn_h_bar]) {
                                ctx.push_ui_action(UiInput::RandomSaved);
                            }
                            ui.set_cursor_pos([n_x, scale.su(2.0)]);
                            if ui.button_with_size("[N] Fetch Online", [n_w, btn_h_bar]) {
                                ctx.push_ui_action(UiInput::FetchOnline);
                            }
                            drop(_dis2);
                        });
                    drop(_tok);

                    let _tok_hdr =
                        ui.push_style_color(imgui::StyleColor::ChildBg, [0.04, 0.05, 0.08, 0.95]);
                    ui.child_window("##colhdr")
                        .size([w, hdr_h])
                        .scroll_bar(false)
                        .border(false)
                        .build(|| {
                            let vy = (hdr_h - 13.0) * 0.5;
                            ui.set_cursor_pos([cx_name, vy]);
                            util::text_dim(ui, "Name");
                            ui.set_cursor_pos([cx_auth, vy]);
                            util::text_dim(ui, "Author");
                            ui.set_cursor_pos([cx_diff, vy]);
                            util::text_dim(ui, "Difficulty");
                            ui.set_cursor_pos([cx_best, vy]);
                            util::text_dim(ui, "Best");
                        });
                    drop(_tok_hdr);

                    let list_h = h - bar_h - hdr_h - footer_h - scale.su(2.0);
                    let _tok2 =
                        ui.push_style_color(imgui::StyleColor::ChildBg, [0.0, 0.0, 0.0, 0.0]);
                    ui.child_window("##levels")
                        .size([w, list_h])
                        .scroll_bar(false)
                        .border(false)
                        .build(|| {
                            if levels.is_empty() {
                                ui.dummy([1.0, pad]);
                                ui.set_cursor_pos([pad, ui.cursor_pos()[1]]);
                                util::text_body(ui, "No saved levels yet.");
                                ui.set_cursor_pos([pad, ui.cursor_pos()[1] + scale.gap_xs()]);
                                util::text_dim(ui, "Press [N] to fetch a model from Poly Pizza.");
                            } else {
                                for (i, level) in levels.iter().enumerate() {
                                    let dc = util::diff_color(level.difficulty);
                                    let bg = if i % 2 == 0 {
                                        [0.05, 0.07, 0.10, 0.85]
                                    } else {
                                        [0.03, 0.04, 0.06, 0.75]
                                    };
                                    let _t3 = ui.push_style_color(imgui::StyleColor::ChildBg, bg);
                                    ui.child_window(format!("##r_{}", level.id))
                                        .size([w, row_h])
                                        .scroll_bar(false)
                                        .border(false)
                                        .build(|| {
                                            let vy = (row_h - 13.0) * 0.5;
                                            ui.set_cursor_pos([cx_name, vy]);
                                            util::text_body(ui, &util::truncate(&level.name, 26));
                                            ui.set_cursor_pos([cx_auth, vy]);
                                            ui.text_colored(
                                                [0.50, 0.56, 0.68, 0.85],
                                                &util::truncate(&level.author, 20),
                                            );
                                            ui.set_cursor_pos([cx_diff, vy]);
                                            ui.text_colored(
                                                dc,
                                                util::diff_label_str(level.difficulty),
                                            );
                                            ui.set_cursor_pos([cx_best, vy]);
                                            if let Some(t) = level.best_time_secs {
                                                ui.text_colored(
                                                    [0.60, 0.65, 0.75, 0.85],
                                                    format!("{:.1}s", t),
                                                );
                                            } else {
                                                ui.text_colored(
                                                    [0.28, 0.32, 0.42, 0.5],
                                                    "\u{2014}",
                                                );
                                            }
                                            ui.set_cursor_pos([cx_play, scale.su(1.0)]);
                                            if ui.button_with_size(
                                                format!("Play##{}", level.id),
                                                [play_w, row_h - scale.su(2.0)],
                                            ) {
                                                ctx.push_ui_action(UiInput::PlayLevel(
                                                    level.id.clone(),
                                                ));
                                            }
                                        });
                                    drop(_t3);
                                }
                            }
                        });
                    drop(_tok2);

                    ui.set_cursor_pos([pad, h - footer_h]);
                    ui.text_colored(
                        [0.28, 0.34, 0.46, 0.45],
                        "Models via Poly Pizza (poly.pizza) CC-BY  |  Cactus by SoyMaria",
                    );
                }

                drop(_ip2);
            });
        drop(_wp);
        drop(_ip);
    });
    scene.add_node(Rc::new(RefCell::new(menu)));
}
