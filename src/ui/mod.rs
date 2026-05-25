use eframe::egui;
use std::sync::mpsc;
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::haruhi;

enum Progress {
    Log(String),
    Percent(f32),
    Done(String),
    Error(String),
    Bytes(usize),
}

#[derive(PartialEq, Clone)]
enum AnimState {
    Idle,
    Processing,
    Done,
}

pub struct HaruhiCryptApp {
    key: String,
    file_path: String,
    output_folder: String,
    status: String,
    progress: f32,
    progress_rx: mpsc::Receiver<Progress>,
    log_messages: Vec<String>,
    haruhi_texture: Option<egui::TextureHandle>,
    haruhi_working_texture: Option<egui::TextureHandle>,
    anim_state: AnimState,
    bytes_processed: usize,
    music_volume: f32,
    music_playing: Arc<AtomicBool>,
    music_started: bool,
    music_sink: Option<Arc<rodio::Sink>>,
    _audio_stream: Option<rodio::OutputStream>, // Keep stream alive
}

impl HaruhiCryptApp {
    pub fn new() -> Self {
        Self {
            key: String::new(),
            file_path: String::new(),
            output_folder: String::new(),
            status: "Ready".to_string(),
            progress: 0.0,
            progress_rx: mpsc::channel().1,
            log_messages: Vec::new(),
            haruhi_texture: None,
            haruhi_working_texture: None,
            anim_state: AnimState::Idle,
            bytes_processed: 0,
            music_volume: 1.0,
            music_playing: Arc::new(AtomicBool::new(true)),
            music_started: false,
            music_sink: None,
            _audio_stream: None,
        }
    }

    fn format_number(n: usize) -> String {
        let s = n.to_string();
        let mut result = String::new();
        for (i, c) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.insert(0, ',');
            }
            result.insert(0, c);
        }
        result
    }

    fn generate_random_key(&mut self) {
        let mut key = String::with_capacity(32);
        let random_bytes: [u8; 16] = rand::random();
        for (i, byte) in random_bytes.iter().enumerate() {
            key.push_str(&format!("{:02x}", byte));
            if i % 4 == 3 && i < 15 {
                key.push('-');
            }
        }
        self.key = key;
        self.log("Key generated");
    }

    fn log(&mut self, msg: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        self.log_messages.push(format!("[{}] {}", timestamp, msg));
        if self.log_messages.len() > 100 {
            self.log_messages.remove(0);
        }
    }

    fn load_haruhi_image(&mut self, ctx: &egui::Context) {
        if self.haruhi_texture.is_none() {
            let img_data = include_bytes!("../../resources/haruhi.jpg");
            if let Ok(img) = image::load_from_memory(img_data) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.as_raw()[..]);
                let texture = ctx.load_texture("haruhi", color_image, egui::TextureOptions::default());
                self.haruhi_texture = Some(texture);
            }
        }
        if self.haruhi_working_texture.is_none() {
            let img_data = include_bytes!("../../resources/haruhi-1.jpg");
            if let Ok(img) = image::load_from_memory(img_data) {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.as_raw()[..]);
                let texture = ctx.load_texture("haruhi_working", color_image, egui::TextureOptions::default());
                self.haruhi_working_texture = Some(texture);
            }
        }
        self.start_music();
    }

    fn start_music(&mut self) {
        if self.music_started || !self.music_playing.load(Ordering::SeqCst) {
            return;
        }
        self.music_started = true;
        
        let music_data_1 = include_bytes!("../../resources/限界突破のメロディ+(Melody+of+Breaking+Limits).mp3").to_vec();
        let music_data_2 = include_bytes!("../../resources/限界突破のメロディ+(Melody+of+Breaking+Limits)-1.mp3").to_vec();
        
        if let Ok((stream, stream_handle)) = rodio::OutputStream::try_default() {
            if let Ok(sink) = rodio::Sink::try_new(&stream_handle) {
                let arc_sink = Arc::new(sink);
                self.music_sink = Some(arc_sink.clone());
                self._audio_stream = Some(stream); // Keep stream alive in the struct
                
                let playing = self.music_playing.clone();
                let sink_clone = arc_sink.clone();
                
                thread::spawn(move || {
                    loop {
                        if !playing.load(Ordering::SeqCst) {
                            break;
                        }
                        
                        // Refill the queue if it's getting empty (we want continuous playback)
                        // If length is 0, play both songs in sequence
                        if sink_clone.len() == 0 {
                            let cursor1 = std::io::Cursor::new(music_data_1.clone());
                            if let Ok(source) = rodio::Decoder::new(cursor1) {
                                sink_clone.append(source);
                            }
                            
                            let cursor2 = std::io::Cursor::new(music_data_2.clone());
                            if let Ok(source) = rodio::Decoder::new(cursor2) {
                                sink_clone.append(source);
                            }
                        }
                        
                        thread::sleep(std::time::Duration::from_millis(1000));
                    }
                });
            }
        }
    }

    fn process_progress(&mut self) {
        while let Ok(progress) = self.progress_rx.try_recv() {
            match progress {
                Progress::Log(msg) => {
                    self.log(&msg);
                }
                Progress::Percent(p) => {
                    self.progress = p;
                    self.status = format!("Processing... {:.0}%", p * 100.0);
                }
                Progress::Done(path) => {
                    self.status = "Done!".to_string();
                    self.progress = 1.0;
                    self.anim_state = AnimState::Done;
                    self.log(&format!("File saved: {}", path));
                }
                Progress::Error(e) => {
                    self.status = format!("Error!");
                    self.progress = 0.0;
                    self.anim_state = AnimState::Idle;
                    self.log(&format!("ERROR: {}", e));
                }
                Progress::Bytes(b) => {
                    self.bytes_processed = b;
                }
            }
        }
        if self.progress >= 1.0 && self.anim_state == AnimState::Processing {
            self.anim_state = AnimState::Done;
        }
    }

    fn is_output_folder_valid(&self) -> bool {
        !self.output_folder.trim().is_empty() && std::path::Path::new(&self.output_folder).is_dir()
    }

    fn is_processing(&self) -> bool {
        self.anim_state == AnimState::Processing
    }
}

impl eframe::App for HaruhiCryptApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.load_haruhi_image(ctx);
        self.process_progress();

        let output_folder_valid = self.is_output_folder_valid();
        let can_encrypt = !self.key.is_empty() && !self.file_path.is_empty() && output_folder_valid && !self.is_processing();
        let can_decrypt = !self.key.is_empty() && !self.file_path.is_empty() && output_folder_valid && !self.is_processing();

        let is_dark = self.anim_state == AnimState::Processing || self.anim_state == AnimState::Done;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading("🔐 HaruhiCrypt");
                ui.add_space(5.0);

                let status_color = match self.anim_state {
                    AnimState::Idle => egui::Color32::GRAY,
                    AnimState::Processing => egui::Color32::from_rgb(255, 200, 100),
                    AnimState::Done => egui::Color32::from_rgb(100, 255, 150),
                };
                let status_text = match self.anim_state {
                    AnimState::Idle => "⚡ Ready".to_string(),
                    AnimState::Processing => format!("⚙️ {}", self.status),
                    AnimState::Done => "✅ Done!".to_string(),
                };
                ui.label(egui::RichText::new(status_text).color(status_color).small());

                ui.add_space(5.0);
                ui.label(egui::RichText::new("Encryption based on the Haruhi Problem").weak());
                ui.add_space(10.0);

                if let (Some(base_texture), Some(working_texture)) = (&self.haruhi_texture, &self.haruhi_working_texture) {
                    let image_size = if is_dark { 250.0 } else { 380.0 };
                    let sized = egui::load::SizedTexture::from_handle(if is_dark { working_texture } else { base_texture });
                    let image = egui::Image::from_texture(sized).fit_to_exact_size([image_size, image_size].into());
                    ui.add(image);

                    if self.anim_state == AnimState::Processing {
                        let time = ctx.input(|i| i.time);
                        let pulse = (time as f32 * 2.0).sin().abs() * 0.15 + 0.05;
                        let rect = ui.available_rect_before_wrap();
                        ui.painter().rect_filled(rect, 0.0, egui::Color32::from_rgba_unmultiplied(155, 89, 182, (pulse * 255.0) as u8));
                    }
                }
            });

            ui.add_space(15.0);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.group(|ui| {
                        ui.strong("🔑 Key");
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.key);
                            if ui.button("🎲 Generate").clicked() {
                                self.generate_random_key();
                            }
                        });
                        ui.add_space(3.0);
                        ui.label(egui::RichText::new("16 bytes hex key (e.g., 0123-4567-89ab-cdef)").weak());
                    });

                    ui.add_space(10.0);

                    ui.group(|ui| {
                        ui.strong("📁 Files");
                        ui.add_space(5.0);

                        ui.label("Input File:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.file_path);
                            if ui.button("📂 Browse").clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    self.file_path = path.to_string_lossy().to_string();
                                    self.log(&format!("File selected: {}", self.file_path));
                                }
                            }
                        });

                        ui.add_space(8.0);

                        ui.label("Output Folder:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.output_folder);
                            if ui.button("📂 Browse").clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                    self.output_folder = path.to_string_lossy().to_string();
                                    self.log(&format!("Output folder: {}", self.output_folder));
                                }
                            }
                        });
                        if !output_folder_valid && !self.output_folder.is_empty() {
                            ui.label(egui::RichText::new("⚠ Invalid folder").color(egui::Color32::RED).small());
                        } else if self.output_folder.is_empty() {
                            ui.label(egui::RichText::new("⚠ Output folder is required").color(egui::Color32::RED).small());
                        }
                    });
                });

                ui.add_space(30.0);

                ui.vertical(|ui| {
                    ui.add_space(40.0);

                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 220.0) / 2.0);
                        
                        let encrypt_btn = egui::Button::new(egui::RichText::new("🔒 ENCRYPT").color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(155, 89, 182)).min_size(egui::vec2(100.0, 32.0));
                        let decrypt_btn = egui::Button::new(egui::RichText::new("🔓 DECRYPT").color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(26, 188, 156)).min_size(egui::vec2(100.0, 32.0));

                        let encrypt_btn = ui.add_enabled(can_encrypt, encrypt_btn);
                        let decrypt_btn = ui.add_enabled(can_decrypt, decrypt_btn);

                        if encrypt_btn.clicked() {
                            let file_path = self.file_path.clone();
                            let output_folder = self.output_folder.clone();
                            let (base_path, original_ext) = {
                                let path = std::path::Path::new(&file_path);
                                let base = path.file_stem().and_then(|s| s.to_str()).unwrap_or(&file_path);
                                let ext = path.extension().and_then(|e| e.to_str()).map(|s| s.to_string());
                                (base.to_string(), ext)
                            };
                            let key = self.key.clone();
                            let (tx, rx) = mpsc::channel();
                            self.progress_rx = rx;
                            self.progress = 0.0;
                            self.bytes_processed = 0;
                            self.anim_state = AnimState::Processing;

                            thread::spawn(move || {
                                tx.send(Progress::Log("Starting encryption...".to_string())).ok();
                                tx.send(Progress::Percent(0.1)).ok();

                                let data = match std::fs::read(&file_path) {
                                    Ok(d) => { tx.send(Progress::Bytes(d.len())).ok(); d }
                                    Err(e) => {
                                        tx.send(Progress::Error(e.to_string())).ok();
                                        return;
                                    }
                                };

                                tx.send(Progress::Log(format!("Read {} bytes", data.len()))).ok();
                                tx.send(Progress::Percent(0.3)).ok();

                                let encrypted = haruhi::encrypt_data(&key, &data, original_ext.as_deref());
                                tx.send(Progress::Log("Data encrypted".to_string())).ok();
                                tx.send(Progress::Percent(0.8)).ok();

                                let file_name = format!("{}.haruhi", base_path);
                                let output_path = std::path::Path::new(&output_folder).join(file_name).to_string_lossy().to_string();

                                if let Err(e) = std::fs::write(&output_path, &encrypted) {
                                    tx.send(Progress::Error(e.to_string())).ok();
                                    return;
                                }
                                tx.send(Progress::Done(output_path)).ok();
                            });
                        }

                        ui.add_space(10.0);

                        if decrypt_btn.clicked() {
                            let file_path = self.file_path.clone();
                            let output_folder = self.output_folder.clone();
                            let key = self.key.clone();
                            let (tx, rx) = mpsc::channel();
                            self.progress_rx = rx;
                            self.progress = 0.0;
                            self.bytes_processed = 0;
                            self.anim_state = AnimState::Processing;

                            thread::spawn(move || {
                                tx.send(Progress::Log("Starting decryption...".to_string())).ok();
                                tx.send(Progress::Percent(0.1)).ok();

                                let data = match std::fs::read(&file_path) {
                                    Ok(d) => { tx.send(Progress::Bytes(d.len())).ok(); d }
                                    Err(e) => {
                                        tx.send(Progress::Error(e.to_string())).ok();
                                        return;
                                    }
                                };

                                tx.send(Progress::Log(format!("Read {} bytes", data.len()))).ok();
                                tx.send(Progress::Percent(0.3)).ok();

                                match haruhi::decrypt_data(&key, &data) {
                                    Ok((decrypted, ext)) => {
                                        tx.send(Progress::Log("Data decrypted".to_string())).ok();
                                        tx.send(Progress::Percent(0.8)).ok();
                                        let base_name = if file_path.ends_with(".haruhi") {
                                            std::path::Path::new(&file_path)
                                                .file_stem()
                                                .and_then(|s| s.to_str())
                                                .unwrap_or("output")
                                                .to_string()
                                        } else {
                                            "output".to_string()
                                        };
                                        let file_name = format!("{}.{}", base_name, ext);
                                        let output_path = std::path::Path::new(&output_folder).join(file_name).to_string_lossy().to_string();

                                        if let Err(e) = std::fs::write(&output_path, &decrypted) {
                                            tx.send(Progress::Error(e.to_string())).ok();
                                            return;
                                        }
                                        tx.send(Progress::Done(output_path)).ok();
                                    },
                                    Err(e) => {
                                        tx.send(Progress::Error(e)).ok();
                                    }
                                }
                            });
                        }
                    });

                    ui.add_space(20.0);

                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 280.0) / 2.0);
                        ui.group(|ui| {
                            ui.set_min_width(120.0);
                            ui.strong("💾 Bytes");
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new(format!("{}", Self::format_number(self.bytes_processed))).size(16.0));
                        });

                        ui.add_space(20.0);

                        ui.group(|ui| {
                            ui.set_min_width(120.0);
                            ui.strong("🔔 Status");
                            ui.add_space(5.0);
                            let status_text = match self.anim_state {
                                AnimState::Idle => "Ready",
                                AnimState::Processing => "Working...",
                                AnimState::Done => "Complete!",
                            };
                            ui.label(egui::RichText::new(status_text).size(16.0));
                        });
                    });

                    ui.add_space(15.0);

                    ui.horizontal(|ui| {
                        ui.add_space((ui.available_width() - 250.0) / 2.0);
                        ui.label("🔊");
                        if ui.add(egui::Slider::new(&mut self.music_volume, 0.0..=1.0)
                            .text("Volume")
                            .clamp_to_range(true)).changed() {
                            if let Some(sink) = &self.music_sink {
                                sink.set_volume(self.music_volume);
                            }
                        }
                    });
                });
            });

            ui.add_space(10.0);

            let progress_bar = egui::ProgressBar::new(self.progress)
                .show_percentage()
                .animate(true)
                .fill(egui::Color32::from_rgb(155, 89, 182));
            ui.add(progress_bar);

            ui.add_space(10.0);

            let available_height = ui.available_height();
            let terminal_height = (available_height * 0.2).clamp(80.0, 200.0);

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.strong("📋 Terminal");
                ui.add_space(5.0);
                egui::ScrollArea::vertical()
                    .max_height(terminal_height)
                    .show(ui, |ui| {
                        let log_text = self.log_messages.join("\n");
                        ui.label(egui::RichText::new(log_text).monospace().small());
                    });
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.add_space(5.0);
                ui.separator();
                ui.label(egui::RichText::new("HaruhiCrypt v0.2.0").weak());
            });
        });
    }
}

pub fn run() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([850.0, 750.0])
            .with_min_inner_size([550.0, 650.0])
            .with_title("HaruhiCrypt"),
        ..Default::default()
    };

    eframe::run_native(
        "HaruhiCrypt",
        options,
        Box::new(|_cc| Box::new(HaruhiCryptApp::new()) as Box<dyn eframe::App>),
    ).expect("Failed to run eframe");
}