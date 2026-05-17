use eframe::egui;
use crate::{transpile, Direction};

pub struct TranspilerApp {
    source_lang: Lang,
    target_lang: Lang,
    input_code: String,
    output_code: String,
    status: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
    Cpp,
    Rust,
}

impl Lang {
    fn label(&self) -> &'static str {
        match self {
            Lang::Cpp => "C++",
            Lang::Rust => "Rust",
        }
    }
}

impl Default for TranspilerApp {
    fn default() -> Self {
        Self {
            source_lang: Lang::Cpp,
            target_lang: Lang::Rust,
            input_code: include_str!("../examples/hello.cpp").to_string(),
            output_code: String::new(),
            status: "Ready — select languages and click Transpile".to_string(),
        }
    }
}

impl eframe::App for TranspilerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("C++ ↔ Rust Transpiler");
            });
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let color = if self.status.starts_with("Error") {
                    egui::Color32::RED
                } else if self.status.starts_with("Success") {
                    egui::Color32::GREEN
                } else if self.status.starts_with("Swapped") {
                    egui::Color32::YELLOW
                } else {
                    egui::Color32::GRAY
                };
                ui.label(egui::RichText::new(&self.status).color(color));
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let total = ui.available_size();
            let gap = 12.0;
            let btn_col_width = 120.0;
            let panel_width = (total.x - btn_col_width - gap * 2.0) / 2.0;
            let panel_height = total.y;

            ui.horizontal(|ui| {
                // Left: Source input
                ui.allocate_ui_with_layout(
                    egui::vec2(panel_width, panel_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.group(|ui| {
                            ui.set_min_size(egui::vec2(panel_width, panel_height));
                            ui.horizontal(|ui| {
                                ui.label("Source:");
                                egui::ComboBox::from_id_salt("src_lang")
                                    .selected_text(self.source_lang.label())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.source_lang, Lang::Cpp, Lang::Cpp.label());
                                        ui.selectable_value(&mut self.source_lang, Lang::Rust, Lang::Rust.label());
                                    });
                            });
                            ui.add_space(4.0);
                            ui.label("Input code:");
                            ui.add(
                                egui::TextEdit::multiline(&mut self.input_code)
                                    .font(egui::TextStyle::Monospace)
                                    .code_editor()
                                    .desired_rows(25)
                                    .lock_focus(true)
                                    .desired_width(panel_width - 16.0),
                            );
                        });
                    },
                );

                ui.add_space(gap);

                // Middle: Buttons
                ui.allocate_ui_with_layout(
                    egui::vec2(btn_col_width, panel_height),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.add_space(panel_height / 2.0 - 70.0);
                        if ui.add_sized([btn_col_width - 8.0, 40.0], egui::Button::new("▶ Transpile")).clicked() {
                            self.do_transpile();
                        }
                        ui.add_space(16.0);
                        if ui.add_sized([btn_col_width - 8.0, 40.0], egui::Button::new("🔄 Swap")).clicked() {
                            let old_src = self.source_lang;
                            let old_dst = self.target_lang;
                            let old_input = self.input_code.clone();
                            let old_output = self.output_code.clone();

                            self.source_lang = old_dst;
                            self.target_lang = old_src;
                            self.input_code = old_output;
                            self.output_code = old_input;
                            self.status = "Swapped — click Transpile to convert".to_string();
                            ctx.request_repaint();
                        }
                    },
                );

                ui.add_space(gap);

                // Right: Target output
                ui.allocate_ui_with_layout(
                    egui::vec2(panel_width, panel_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.group(|ui| {
                            ui.set_min_size(egui::vec2(panel_width, panel_height));
                            ui.horizontal(|ui| {
                                ui.label("Target:");
                                egui::ComboBox::from_id_salt("tgt_lang")
                                    .selected_text(self.target_lang.label())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.target_lang, Lang::Cpp, Lang::Cpp.label());
                                        ui.selectable_value(&mut self.target_lang, Lang::Rust, Lang::Rust.label());
                                    });
                            });
                            ui.add_space(4.0);
                            ui.label("Output code:");
                            ui.add(
                                egui::TextEdit::multiline(&mut self.output_code)
                                    .font(egui::TextStyle::Monospace)
                                    .code_editor()
                                    .desired_rows(25)
                                    .desired_width(panel_width - 16.0),
                            );
                        });
                    },
                );
            });
        });
    }
}

impl TranspilerApp {
    fn do_transpile(&mut self) {
        let direction = match (self.source_lang, self.target_lang) {
            (Lang::Cpp, Lang::Rust) => Direction::CppToRust,
            (Lang::Rust, Lang::Cpp) => Direction::RustToCpp,
            _ => {
                self.status = "Error: Source and target languages must be different".to_string();
                return;
            }
        };

        match transpile(&self.input_code, direction) {
            Ok(code) => {
                self.output_code = code;
                self.status = "Success!".to_string();
            }
            Err(e) => {
                self.status = format!("Error: {}", e);
            }
        }
    }
}

pub fn run_gui() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "C++ ↔ Rust Transpiler",
        options,
        Box::new(|_cc| Ok(Box::new(TranspilerApp::default()))),
    )
}
