use std::{cell::RefCell, rc::Rc};
use formosaic_engine::architecture::scene::node::{ui_node::UiNode, scenegraph::Scenegraph};
use formosaic_engine::input::{Event, Key};
use imgui::*;
use crate::formosaic::UiState;
use super::util;

pub fn register(scene: &Scenegraph, state: Rc<RefCell<UiState>>) {
    let menu = UiNode::new("menu", move |ui, w, h, _ctx| {
        let s = state.borrow();
        if !s.show_menu { return; }
        let is_touch = s.is_touch;
        let levels   = s.levels.clone();
        let is_dl    = s.is_downloading;

        let menu_flags = WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE
            | WindowFlags::NO_SAVED_SETTINGS | WindowFlags::NO_BACKGROUND
            | WindowFlags::NO_SCROLL_WITH_MOUSE;
        let pad = (w * 0.02).max(8.0);

        let _wp = ui.push_style_var(imgui::StyleVar::WindowPadding([0.0, 0.0]));
        let _ip = ui.push_style_var(imgui::StyleVar::ItemSpacing([0.0, 0.0]));
        ui.window("##menu")
            .flags(menu_flags)
            .position([0.0, 0.0], Condition::Always)
            .size([w, h], Condition::Always)
            .build(|| {
                let _ip2 = ui.push_style_var(imgui::StyleVar::ItemSpacing([8.0, 4.0]));
                let mut fetch_online  = false;
                let mut random_saved  = false;
                let mut play_level_id: Option<String> = None;

                if is_touch {
                    let btn_h    = (h * 0.08).max(52.0);
                    let title_h  = (h * 0.05).max(32.0);
                    let btns_y   = h - btn_h - 80.0;
                    let list_h   = (btns_y - title_h - pad).max(0.0);
                    let row_h    = (h * 0.12).max(80.0);
                    let m        = pad;
                    let row_w    = w - m * 2.0;

                    let _tok = ui.push_style_color(imgui::StyleColor::ChildBg, [0.03, 0.04, 0.06, 0.95]);
                    ui.child_window("##hdr").size([w, title_h]).scroll_bar(false).border(false).build(|| {
                        let bar_w = 140.0_f32;
                        ui.set_cursor_pos([pad, (title_h - 16.0) * 0.5]);
                        ui.text_colored([0.85, 0.62, 0.18, 1.0], "FORMOSAIC");
                        if is_dl {
                            ui.same_line_with_spacing(0.0, pad);
                            ui.text_colored([0.4, 0.75, 1.0, 0.9], "Fetching...");
                        }
                        if let Some(p) = s.download_progress {
                            ui.set_cursor_pos([w - pad - bar_w, (title_h - 10.0) * 0.5]);
                            imgui::ProgressBar::new(p)
                                .size([bar_w, 10.0])
                                .overlay_text(&format!("{:.0}%", p * 100.0))
                                .build(ui);
                        }
                    });
                    drop(_tok);

                    let _tok2 = ui.push_style_color(imgui::StyleColor::ChildBg, [0.0, 0.0, 0.0, 0.0]);
                    ui.child_window("##levels")
                        .size([w, list_h])
                        .scroll_bar(false)
                        .border(false)
                        .build(|| {
                            if levels.is_empty() {
                                ui.dummy([1.0, pad]);
                                ui.set_cursor_pos([pad, ui.cursor_pos()[1]]);
                                ui.text_colored([0.36, 0.42, 0.56, 1.0], "No saved levels yet.");
                                ui.set_cursor_pos([pad, ui.cursor_pos()[1] + 4.0]);
                                ui.text_colored([0.36, 0.42, 0.56, 0.7], "Use the buttons below to fetch one.");
                            } else {
                                for level in &levels {
                                    let dc = util::diff_color(level.difficulty);
                                    let _t3 = ui.push_style_color(imgui::StyleColor::ChildBg, [0.04, 0.06, 0.09, 0.72]);
                                    let _t4 = ui.push_style_color(imgui::StyleColor::Border, dc);
                                    ui.set_cursor_pos([m, ui.cursor_pos()[1]]);
                                    ui.child_window(format!("##r_{}", level.id))
                                        .size([row_w, row_h])
                                        .scroll_bar(false)
                                        .border(true)
                                        .build(|| {
                                            let ip = pad * 0.8;
                                            let pw = 90.0_f32;
                                            let ph = 36.0_f32;
                                            ui.set_cursor_pos([ip, ip * 0.5]);
                                            ui.text_colored([0.88, 0.90, 0.96, 1.0], util::truncate(&level.name, 18));
                                            ui.same_line_with_spacing(0.0, ip * 0.5);
                                            ui.text_colored(dc, util::diff_label_str(level.difficulty));
                                            if let Some(t) = level.best_time_secs {
                                                ui.same_line_with_spacing(0.0, ip * 0.5);
                                                ui.text_colored([0.5, 0.55, 0.65, 0.8], format!("{:.1}s", t));
                                            }
                                            ui.set_cursor_pos([ip, ip * 0.5 + 22.0]);
                                            ui.text_colored([0.36, 0.42, 0.56, 0.8], util::truncate(&level.author, 22));
                                            ui.set_cursor_pos([row_w - pw - ip, (row_h - ph) * 0.5]);
                                            if ui.button_with_size("Play", [pw, ph]) {
                                                play_level_id = Some(level.id.clone());
                                            }
                                        });
                                    drop(_t3); drop(_t4);
                                    ui.dummy([1.0, pad * 0.3]);
                                }
                            }
                        });
                    drop(_tok2);

                    let half = (w - pad * 3.0) * 0.5;
                    ui.set_cursor_pos([pad, btns_y]);
                    if ui.button_with_size("+ Fetch Online", [half, btn_h]) { fetch_online = true; }
                    ui.same_line_with_spacing(0.0, pad);
                    if ui.button_with_size("Random", [half, btn_h]) { random_saved = true; }

                } else {
                    let bar_h    = 28.0_f32;
                    let hdr_h    = 20.0_f32;
                    let row_h    = 24.0_f32;
                    let footer_h = 16.0_f32;

                    let cx_name = pad;
                    let cx_auth = w * 0.30;
                    let cx_diff = w * 0.52;
                    let cx_best = w * 0.64;
                    let cx_play = w * 0.75;
                    let play_w  = w - cx_play - pad;

                    let _tok = ui.push_style_color(imgui::StyleColor::ChildBg, [0.03, 0.04, 0.06, 0.95]);
                    ui.child_window("##bar").size([w, bar_h]).scroll_bar(false).border(false).build(|| {
                        let btn_h_bar = bar_h - 4.0;
                        let n_w   = 150.0_f32;
                        let r_w   = 100.0_f32;
                        let gap   = 4.0_f32;
                        let ver_w = 32.0_f32;
                        let bar_w = 150.0_f32;

                        let r_x  = w - ver_w - pad - gap - r_w;
                        let n_x  = r_x - gap - n_w;

                        ui.set_cursor_pos([pad, (bar_h - 14.0) * 0.5]);
                        ui.text_colored([0.85, 0.62, 0.18, 1.0], "FORMOSAIC");
                        if is_dl {
                            ui.same_line_with_spacing(0.0, pad);
                            ui.set_cursor_pos([ui.cursor_pos()[0], (bar_h - 14.0) * 0.5]);
                            ui.text_colored([0.4, 0.75, 1.0, 0.9], "Fetching...");
                        }
                        if let Some(p) = s.download_progress {
                            ui.set_cursor_pos([n_x - gap - bar_w, 5.0]);
                            imgui::ProgressBar::new(p)
                                .size([bar_w, 10.0])
                                .overlay_text(&format!("{:.0}%", p * 100.0))
                                .build(ui);
                        }
                        ui.set_cursor_pos([w - ver_w - pad, (bar_h - 13.0) * 0.5]);
                        ui.text_colored([0.28, 0.34, 0.46, 0.6], "v0.1");
                        ui.set_cursor_pos([r_x, 2.0]);
                        if ui.button_with_size("[R] Random", [r_w, btn_h_bar]) { random_saved = true; }
                        ui.set_cursor_pos([n_x, 2.0]);
                        if ui.button_with_size("[N] Fetch Online", [n_w, btn_h_bar]) { fetch_online = true; }
                    });
                    drop(_tok);

                    let _tok_hdr = ui.push_style_color(imgui::StyleColor::ChildBg, [0.04, 0.05, 0.08, 0.95]);
                    ui.child_window("##colhdr").size([w, hdr_h]).scroll_bar(false).border(false).build(|| {
                        let vy = (hdr_h - 13.0) * 0.5;
                        ui.set_cursor_pos([cx_name, vy]);
                        ui.text_colored([0.36, 0.42, 0.56, 0.7], "Name");
                        ui.set_cursor_pos([cx_auth, vy]);
                        ui.text_colored([0.36, 0.42, 0.56, 0.7], "Author");
                        ui.set_cursor_pos([cx_diff, vy]);
                        ui.text_colored([0.36, 0.42, 0.56, 0.7], "Difficulty");
                        ui.set_cursor_pos([cx_best, vy]);
                        ui.text_colored([0.36, 0.42, 0.56, 0.7], "Best");
                    });
                    drop(_tok_hdr);

                    let list_h = h - bar_h - hdr_h - footer_h - 2.0;
                    let _tok2 = ui.push_style_color(imgui::StyleColor::ChildBg, [0.0, 0.0, 0.0, 0.0]);
                    ui.child_window("##levels")
                        .size([w, list_h])
                        .scroll_bar(false)
                        .border(false)
                        .build(|| {
                            if levels.is_empty() {
                                ui.dummy([1.0, pad]);
                                ui.set_cursor_pos([pad, ui.cursor_pos()[1]]);
                                ui.text_colored([0.36, 0.42, 0.56, 1.0], "No saved levels yet.");
                                ui.set_cursor_pos([pad, ui.cursor_pos()[1] + 4.0]);
                                ui.text_colored([0.36, 0.42, 0.56, 0.7], "Press [N] to fetch a model from Poly Pizza.");
                            } else {
                                for (i, level) in levels.iter().enumerate() {
                                    let dc = util::diff_color(level.difficulty);
                                    let bg = if i % 2 == 0 { [0.05, 0.07, 0.10, 0.85] }
                                             else           { [0.03, 0.04, 0.06, 0.75] };
                                    let _t3 = ui.push_style_color(imgui::StyleColor::ChildBg, bg);
                                    ui.child_window(format!("##r_{}", level.id))
                                        .size([w, row_h])
                                        .scroll_bar(false)
                                        .border(false)
                                        .build(|| {
                                            let vy = (row_h - 13.0) * 0.5;
                                            ui.set_cursor_pos([cx_name, vy]);
                                            ui.text_colored([0.88, 0.90, 0.96, 1.0], util::truncate(&level.name, 26));
                                            ui.set_cursor_pos([cx_auth, vy]);
                                            ui.text_colored([0.50, 0.56, 0.68, 0.85], util::truncate(&level.author, 20));
                                            ui.set_cursor_pos([cx_diff, vy]);
                                            ui.text_colored(dc, util::diff_label_str(level.difficulty));
                                            ui.set_cursor_pos([cx_best, vy]);
                                            if let Some(t) = level.best_time_secs {
                                                ui.text_colored([0.60, 0.65, 0.75, 0.85], format!("{:.1}s", t));
                                            } else {
                                                ui.text_colored([0.28, 0.32, 0.42, 0.5], "\u{2014}");
                                            }
                                            ui.set_cursor_pos([cx_play, 1.0]);
                                            if ui.button_with_size(
                                                format!("Play##{}", level.id),
                                                [play_w, row_h - 2.0]
                                            ) {
                                                play_level_id = Some(level.id.clone());
                                            }
                                        });
                                    drop(_t3);
                                }
                            }
                        });
                    drop(_tok2);

                    ui.set_cursor_pos([pad, h - footer_h]);
                    ui.text_colored([0.28, 0.34, 0.46, 0.45],
                        "Models via Poly Pizza (poly.pizza) CC-BY  |  Cactus by SoyMaria");
                }

                drop(s);
                if fetch_online { state.borrow_mut().queued_events.push(Event::KeyDown { key: Key::N }); }
                if random_saved { state.borrow_mut().queued_events.push(Event::KeyDown { key: Key::R }); }
                if let Some(id) = play_level_id { state.borrow_mut().play_specific = Some(id); }
                drop(_ip2);
            });
        drop(_wp); drop(_ip);
    });
    scene.add_node(Rc::new(RefCell::new(menu)));
}
