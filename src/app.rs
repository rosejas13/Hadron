// egui UI: log panel, special-paths list, USB status, and a file-picker bridge
// that lets the USB worker thread ask the main thread to open a native dialog.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

use crate::config::Config;

pub enum UsbToUi {
    Log(String),
    Status(String),
    PickFile(Sender<Option<PathBuf>>),
}

pub struct HadronApp {
    cfg: Arc<Mutex<Config>>,
    rx: Receiver<UsbToUi>,
    log_lines: Vec<String>,
    status: String,
    special_paths: Vec<(String, String)>,
    selected: Vec<usize>,
    show_logs: bool,
    add_name: String,
    add_folder: Option<PathBuf>,
    pending_picker: Option<Sender<Option<PathBuf>>>,
}

impl HadronApp {
    pub fn new(cfg: Arc<Mutex<Config>>, rx: Receiver<UsbToUi>) -> Self {
        HadronApp {
            cfg,
            rx,
            log_lines: Vec::new(),
            status: "Starting Hadron...".to_string(),
            special_paths: Vec::new(),
            selected: Vec::new(),
            show_logs: true,
            add_name: String::new(),
            add_folder: None,
            pending_picker: None,
        }
    }

    fn refresh_paths(&mut self) {
        if let Ok(c) = self.cfg.lock() {
            self.special_paths = c.entries();
        }
    }

    fn commit_add(&mut self) {
        let name = self.add_name.trim().to_string();
        if name.is_empty() {
            self.add_folder = None;
            self.add_name.clear();
            return;
        }
        if let Some(folder) = self.add_folder.take() {
            if let Ok(mut c) = self.cfg.lock() {
                c.add(&name, &folder.to_string_lossy());
                let _ = c.save();
            }
        }
        self.add_name.clear();
    }

    fn remove_selected(&mut self) {
        let names: Vec<String> = self
            .selected
            .iter()
            .filter_map(|&i| self.special_paths.get(i).map(|(n, _)| n.clone()))
            .collect();
        if names.is_empty() {
            return;
        }
        if let Ok(mut c) = self.cfg.lock() {
            c.remove(&names);
            let _ = c.save();
        }
        self.selected.clear();
    }
}

impl eframe::App for HadronApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain USB->UI messages first.
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                UsbToUi::Log(line) => {
                    self.log_lines.push(line);
                    if self.log_lines.len() > 2000 {
                        let drop = self.log_lines.len() - 2000;
                        self.log_lines.drain(..drop);
                    }
                }
                UsbToUi::Status(s) => self.status = s,
                UsbToUi::PickFile(tx) => self.pending_picker = Some(tx),
            }
        }

        // If the USB worker asked for a file picker, open a native dialog now
        // (must run on the main thread on macOS). This briefly blocks the frame.
        if let Some(tx) = self.pending_picker.take() {
            let chosen = rfd::FileDialog::new().pick_file();
            let _ = tx.send(chosen);
        }

        self.refresh_paths();

        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(&self.status)
                            .color(egui::Color32::from_rgb(120, 120, 120)),
                    );
                });
            });

        if self.show_logs {
            egui::SidePanel::left("logs")
                .resizable(true)
                .default_width(380.0)
                .min_width(220.0)
                .show(ctx, |ui| {
                    ui.checkbox(&mut self.show_logs, "Show logs");
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for line in &self.log_lines {
                                ui.label(
                                    egui::RichText::new(line)
                                        .monospace()
                                        .color(egui::Color32::from_rgb(80, 200, 80)),
                                );
                            }
                        });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Add new path").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.add_name = folder
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        self.add_folder = Some(folder);
                    }
                }
                if ui.button("Remove selected path").clicked() {
                    self.remove_selected();
                }
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, (name, path)) in self.special_paths.iter().enumerate() {
                        let mut selected = self.selected.contains(&i);
                        let resp = ui.selectable_label(selected, format!("{name} ({path})"));
                        if resp.clicked() {
                            if selected {
                                self.selected.retain(|&x| x != i);
                            } else {
                                self.selected.push(i);
                                selected = true;
                            }
                        }
                        let _ = selected;
                    }
                });
        });

        // Add-path name dialog.
        if self.add_folder.is_some() {
            let mut still_open = true;
            egui::Window::new("Add special path")
                .open(&mut still_open)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Set the special path's name");
                    ui.text_edit_singleline(&mut self.add_name);
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            self.commit_add();
                        }
                        if ui.button("Cancel").clicked() {
                            self.add_folder = None;
                            self.add_name.clear();
                        }
                    });
                });
            if !still_open {
                self.add_folder = None;
                self.add_name.clear();
            }
        }
    }
}