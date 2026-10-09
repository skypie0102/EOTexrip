#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use eframe::egui;
use eotexrip::{
    catalog::{Catalog, Category, Game, Override, Overrides},
    pipeline, workspace,
};
use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

enum Event {
    Progress(pipeline::Progress),
    Done(Box<Catalog>, bool),
    Error(String),
}
enum Job {
    Extract(pipeline::Options),
    Export(PathBuf),
    Apply(PathBuf, Overrides),
    Replay(PathBuf),
}
struct App {
    smoke_test: bool,
    started: Instant,
    input: String,
    output: String,
    title: String,
    game: Game,
    catalog: Option<Catalog>,
    metadata_only: bool,
    search: String,
    review_only: bool,
    category_filter: Option<Category>,
    selected: Option<usize>,
    edit_name: String,
    edit_category: Category,
    same_source: bool,
    choices: Overrides,
    pending: usize,
    preview: Option<egui::TextureHandle>,
    preview_error: String,
    receiver: Option<mpsc::Receiver<Event>>,
    cancel: Arc<AtomicBool>,
    status: String,
    error: String,
    progress: pipeline::Progress,
}
impl Default for App {
    fn default() -> Self {
        Self {
            smoke_test: false,
            started: Instant::now(),
            input: String::new(),
            output: String::new(),
            title: String::new(),
            game: Game::Eou,
            catalog: None,
            metadata_only: false,
            search: String::new(),
            review_only: true,
            category_filter: None,
            selected: None,
            edit_name: String::new(),
            edit_category: Category::Misc,
            same_source: false,
            choices: Overrides::default(),
            pending: 0,
            preview: None,
            preview_error: String::new(),
            receiver: None,
            cancel: Arc::new(AtomicBool::new(false)),
            status: "Choose a decrypted game dump or resource folder to begin.".into(),
            error: String::new(),
            progress: pipeline::Progress {
                resources: 0,
                textures: 0,
                message: String::new(),
            },
        }
    }
}
impl App {
    fn start(&mut self, job: Job, ctx: &egui::Context) {
        if self.receiver.is_some() {
            return;
        }
        self.error.clear();
        self.status = "Working…".into();
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = self.cancel.clone();
        let repaint = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        std::thread::spawn(move || {
            let metadata = matches!(&job, Job::Replay(_));
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match job {
                Job::Extract(options) => pipeline::extract(&options, &cancel, |p| {
                    let _ = tx.send(Event::Progress(p));
                    repaint.request_repaint();
                }),
                Job::Export(root) => pipeline::export(&root, &cancel),
                Job::Apply(root, choices) => workspace::reassign(&root, choices),
                Job::Replay(path) => pipeline::replay(&path, None),
            }));
            let event=match result {Ok(Ok(catalog))=>Event::Done(Box::new(catalog),metadata),Ok(Err(error))=>Event::Error(format!("{error:#}")),Err(_)=>Event::Error("The job stopped unexpectedly. Reopen the workspace to recover any interrupted publication.".into())};
            let _ = tx.send(event);
            repaint.request_repaint();
        });
    }
    fn install(&mut self, catalog: Catalog, metadata: bool) {
        self.game = catalog.game;
        self.title = catalog.title_id.clone().unwrap_or_default();
        self.status = format!(
            "{} images · {} category reviews · {} name reviews",
            catalog.summary.unique_images,
            catalog.summary.category_review,
            catalog.summary.name_review
        );
        self.catalog = Some(catalog);
        self.metadata_only = metadata;
        self.selected = None;
        self.preview = None;
        self.pending = 0;
        self.choices = if metadata {
            Overrides::default()
        } else {
            File::open(Path::new(&self.output).join(".eouhd/overrides.json"))
                .ok()
                .and_then(|f| serde_json::from_reader(f).ok())
                .unwrap_or_default()
        };
    }
    fn poll(&mut self) {
        let events: Vec<_> = self
            .receiver
            .as_ref()
            .map(|r| r.try_iter().collect())
            .unwrap_or_default();
        for event in events {
            match event {
                Event::Progress(p) => {
                    self.status = p.message.clone();
                    self.progress = p;
                }
                Event::Done(c, metadata) => {
                    self.install(*c, metadata);
                    self.receiver = None;
                }
                Event::Error(message) => {
                    self.error = message;
                    self.status = "Job stopped. Existing published masters are preserved.".into();
                    self.receiver = None;
                }
            }
        }
    }
    fn select(&mut self, index: usize, ctx: &egui::Context) {
        self.selected = Some(index);
        self.preview = None;
        self.preview_error.clear();
        self.same_source = false;
        if let Some(asset) = self.catalog.as_ref().and_then(|c| c.assets.get(index)) {
            self.edit_name = asset.name.clone();
            self.edit_category = asset.category.category;
            if !self.metadata_only {
                match read_image(
                    &Path::new(&self.output)
                        .join("azahar_pack_master")
                        .join(&asset.master_file),
                ) {
                    Ok(image) => {
                        self.preview =
                            Some(ctx.load_texture(&asset.id, image, egui::TextureOptions::NEAREST))
                    }
                    Err(error) => self.preview_error = format!("Preview unavailable: {error}"),
                }
            }
        }
    }
    fn queue_choice(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        let Some(catalog) = &self.catalog else {
            return;
        };
        let selected = &catalog.assets[index];
        let ids: Vec<_> = catalog
            .assets
            .iter()
            .filter(|a| a.id == selected.id || self.same_source && a.source == selected.source)
            .map(|a| (a.id.clone(), a.aliases.clone(), a.id == selected.id))
            .collect();
        for (id, aliases, is_selected) in ids {
            let name = if is_selected {
                Some(eotexrip::naming::safe_stem(&self.edit_name))
            } else {
                self.choices.assets.get(&id).and_then(|o| o.name.clone())
            };
            let choice = Override {
                category: Some(self.edit_category),
                name,
                confirmed: true,
            };
            self.choices.assets.insert(id, choice.clone());
            for alias in aliases {
                self.choices.assets.insert(alias, choice.clone());
            }
            self.pending += 1;
        }
        self.status = format!(
            "{} corrections queued. Save corrections to move images and refresh the emulator pack.",
            self.pending
        );
    }
    fn setup(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let idle = self.receiver.is_none();
        ui.horizontal(|ui| {
            ui.heading("EO-Texrip");
            ui.label(eotexrip::VERSION);
        });
        ui.label("Extract → organize → edit masters → prepare for Azahar");
        ui.add_space(8.0);
        ui.add_enabled_ui(idle, |ui| {
            egui::Grid::new("setup")
                .num_columns(3)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Game");
                    egui::ComboBox::from_id_salt("game")
                        .selected_text(self.game.label())
                        .show_ui(ui, |ui| {
                            for game in Game::ALL {
                                ui.selectable_value(&mut self.game, game, game.label());
                            }
                        });
                    ui.label(if self.game.research_only() {
                        "Research only"
                    } else {
                        "3DS"
                    });
                    ui.end_row();
                    ui.label("Input");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.input)
                            .desired_width(f32::INFINITY)
                            .hint_text("Decrypted ROM, RomFS, or resource folder"),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("File…").clicked()
                            && let Some(p) = rfd::FileDialog::new()
                                .add_filter(
                                    "Game or resource",
                                    &[
                                        "3ds", "cci", "cia", "cxi", "app", "romfs", "stex", "bam",
                                        "bam2", "bcmdl", "bch", "bcfnt", "ctpk",
                                    ],
                                )
                                .pick_file()
                        {
                            self.input = p.display().to_string();
                        }
                        if ui.button("Folder…").clicked()
                            && let Some(p) = rfd::FileDialog::new().pick_folder()
                        {
                            self.input = p.display().to_string();
                        }
                    });
                    ui.end_row();
                    ui.label("Workspace");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.output)
                            .desired_width(f32::INFINITY)
                            .hint_text("Folder for masters, catalog, and emulator pack"),
                    );
                    if ui.button("Choose…").clicked()
                        && let Some(p) = rfd::FileDialog::new().pick_folder()
                    {
                        self.output = p.display().to_string();
                    }
                    ui.end_row();
                    ui.label("Title ID");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.title)
                            .desired_width(220.0)
                            .hint_text("Auto-detected from a ROM"),
                    );
                    ui.label("Needed for emulator deployment from loose files");
                    ui.end_row();
                });
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        !self.input.trim().is_empty()
                            && !self.output.trim().is_empty()
                            && !self.game.research_only()
                            && self.pending == 0,
                        egui::Button::new("Extract and organize"),
                    )
                    .clicked()
                {
                    self.start(
                        Job::Extract(pipeline::Options {
                            input: self.input.trim().into(),
                            output: self.output.trim().into(),
                            game: self.game,
                            title_id: if self.title.trim().is_empty() {
                                None
                            } else {
                                Some(self.title.trim().to_uppercase())
                            },
                        }),
                        ctx,
                    );
                }
                if ui
                    .add_enabled(
                        !self.output.is_empty() && self.pending == 0,
                        egui::Button::new("Load workspace"),
                    )
                    .clicked()
                {
                    match workspace::load(Path::new(&self.output)) {
                        Ok(c) => self.install(c, false),
                        Err(e) => self.error = format!("{e:#}"),
                    }
                }
                if ui
                    .add_enabled(
                        self.catalog.is_some() && !self.metadata_only && self.pending == 0,
                        egui::Button::new("Rebuild emulator pack"),
                    )
                    .clicked()
                {
                    self.start(Job::Export(self.output.clone().into()), ctx);
                }
                if ui.button("Analyze saved metadata…").clicked()
                    && let Some(p) = rfd::FileDialog::new()
                        .add_filter("Calibration bundle", &["json"])
                        .pick_file()
                {
                    self.start(Job::Replay(p), ctx);
                }
                if ui
                    .add_enabled(!self.output.is_empty(), egui::Button::new("Open output"))
                    .clicked()
                    && let Err(e) = open_folder(Path::new(&self.output))
                {
                    self.error = e.to_string();
                }
            });
        });
        if !idle {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!(
                    "{} resources · {} textures",
                    self.progress.resources, self.progress.textures
                ));
                if ui.button("Cancel extraction").clicked() {
                    self.cancel.store(true, Ordering::Relaxed);
                    self.status =
                        "Cancellation requested. Finishing the current safe operation…".into();
                }
            });
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        ui.add_space(6.0);
        ui.label(&self.status);
        if !self.error.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(255, 130, 120), &self.error);
        }
    }
    fn review(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let Some(catalog) = &self.catalog else {
            ui.add_space(30.0);
            ui.label("Texture previews, original sources, naming evidence, and category corrections appear here after extraction.");
            return;
        };
        let s = &catalog.summary;
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("{} images", s.unique_images));
            ui.label(format!(
                "{} mapped · {} PNG only · {} category reviews · {} name reviews · {} issues",
                s.mapped_images, s.png_only_images, s.category_review, s.name_review, s.issues
            ));
        });
        if self.metadata_only {
            ui.colored_label(egui::Color32::from_rgb(230,190,90),"Metadata review: previews and pixel validation require an extracted workspace. Corrections can be saved for later use.");
        }
        let issues = catalog.issues.clone();
        egui::CollapsingHeader::new(format!("Extraction issues ({})", issues.len())).show(
            ui,
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("issues")
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for issue in &issues {
                            ui.label(format!(
                                "{} · {}: {}",
                                issue.source, issue.stage, issue.message
                            ));
                        }
                    });
            },
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Search names and original sources")
                    .desired_width(280.0),
            );
            ui.checkbox(&mut self.review_only, "Needs review");
            egui::ComboBox::from_id_salt("filter")
                .selected_text(
                    self.category_filter
                        .map(|c| c.folder())
                        .unwrap_or("All categories"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.category_filter, None, "All categories");
                    for category in Category::ALL {
                        ui.selectable_value(
                            &mut self.category_filter,
                            Some(category),
                            category.folder(),
                        );
                    }
                });
        });
        let needle = self.search.to_lowercase();
        let catalog = self.catalog.as_ref().unwrap();
        let rows: Vec<_> = catalog
            .assets
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                (!self.review_only || a.category.needs_review || a.name_needs_review)
                    && self
                        .category_filter
                        .is_none_or(|c| a.category.category == c)
                    && (needle.is_empty()
                        || a.name.to_lowercase().contains(&needle)
                        || a.origins.iter().any(|o| {
                            o.source.to_lowercase().contains(&needle)
                                || o.internal_name.to_lowercase().contains(&needle)
                        })
                        || a.source.to_lowercase().contains(&needle))
            })
            .map(|(i, a)| {
                (
                    i,
                    a.name.clone(),
                    a.category.category,
                    a.category.needs_review,
                    a.name_needs_review,
                )
            })
            .collect();
        ui.label(format!("{} shown", rows.len()));
        let mut clicked = None;
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("assets")
                .auto_shrink([false, false])
                .max_height(480.0)
                .show_rows(&mut columns[0], 25.0, rows.len(), |ui, range| {
                    for row in &rows[range] {
                        let (index, name, category, category_review, name_review) = row;
                        let label = format!(
                            "{} / {}{}",
                            category.folder(),
                            name,
                            if *category_review || *name_review {
                                "  • review"
                            } else {
                                ""
                            }
                        );
                        if ui
                            .selectable_label(self.selected == Some(*index), label)
                            .clicked()
                        {
                            clicked = Some(*index);
                        }
                    }
                });
            let ui = &mut columns[1];
            if let Some(asset) = self
                .selected
                .and_then(|i| self.catalog.as_ref().and_then(|c| c.assets.get(i)))
                .cloned()
            {
                egui::ScrollArea::vertical()
                    .id_salt("detail")
                    .max_height(480.0)
                    .show(ui, |ui| {
                        if let Some(preview) = &self.preview {
                            ui.add(
                                egui::Image::new(preview)
                                    .max_size(egui::vec2(320.0, 210.0))
                                    .maintain_aspect_ratio(true),
                            );
                        }
                        if !self.preview_error.is_empty() {
                            ui.label(&self.preview_error);
                        }
                        ui.strong(&asset.name);
                        ui.label(format!(
                            "{}×{} · {} · {} runtime hashes",
                            asset.width,
                            asset.height,
                            eotexrip::pica::FORMAT_NAMES
                                .get(asset.format as usize)
                                .unwrap_or(&"unknown"),
                            asset.runtime_hashes.len()
                        ));
                        ui.label(format!(
                            "Category: {} ({})",
                            asset.category.category.folder(),
                            asset.category.grade
                        ));
                        ui.label(format!(
                            "Name basis: {}{}",
                            asset.name_basis,
                            if asset.name_needs_review {
                                " · identity needs review"
                            } else {
                                ""
                            }
                        ));
                        ui.add_enabled_ui(self.receiver.is_none(), |ui| {
                            ui.label("Confirmed filename");
                            ui.text_edit_singleline(&mut self.edit_name);
                            egui::ComboBox::from_id_salt("edit_category")
                                .selected_text(self.edit_category.folder())
                                .show_ui(ui, |ui| {
                                    for c in Category::ALL {
                                        ui.selectable_value(&mut self.edit_category, c, c.folder());
                                    }
                                });
                            ui.checkbox(
                                &mut self.same_source,
                                "Apply category to every texture in this source",
                            );
                            if ui.button("Queue confirmed correction").clicked() {
                                self.queue_choice();
                            }
                        });
                        egui::CollapsingHeader::new("Why this category?")
                            .default_open(true)
                            .show(ui, |ui| {
                                if asset.category.evidence.is_empty() {
                                    ui.label("No verified category evidence found.");
                                }
                                for e in &asset.category.evidence {
                                    ui.label(format!(
                                        "{}: {} ({})",
                                        e.category.folder(),
                                        e.detail,
                                        e.rule
                                    ));
                                }
                            });
                        egui::CollapsingHeader::new("Original identities and runtime hashes").show(
                            ui,
                            |ui| {
                                ui.monospace(&asset.id);
                                ui.label(&asset.source);
                                ui.label(&asset.internal_name);
                                for origin in &asset.origins {
                                    ui.label(format!(
                                        "{} → {}",
                                        origin.source, origin.internal_name
                                    ));
                                }
                                for hash in &asset.runtime_hashes {
                                    ui.monospace(hash);
                                }
                            },
                        );
                    });
            } else {
                ui.label("Select an image to inspect its evidence and preview.");
            }
        });
        if let Some(index) = clicked {
            self.select(index, ctx);
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.pending > 0 && !self.metadata_only && self.receiver.is_none(),
                    egui::Button::new(format!(
                        "Save {} corrections and rebuild pack",
                        self.pending
                    )),
                )
                .clicked()
            {
                self.start(
                    Job::Apply(self.output.clone().into(), self.choices.clone()),
                    ctx,
                );
            }
            if ui
                .add_enabled(
                    self.pending > 0 && self.receiver.is_none(),
                    egui::Button::new("Save correction file…"),
                )
                .clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .set_file_name(format!("{}-confirmed-names.json", self.game.prefix()))
                    .save_file()
            {
                match workspace::save_choices(&p, &self.choices) {
                    Ok(()) => self.status = format!("Saved {}", p.display()),
                    Err(e) => self.error = e.to_string(),
                }
            }
            if ui
                .add_enabled(
                    self.receiver.is_none(),
                    egui::Button::new("Import correction file…"),
                )
                .clicked()
                && let Some(p) = rfd::FileDialog::new()
                    .add_filter("Corrections", &["json"])
                    .pick_file()
            {
                match File::open(p)
                    .map_err(anyhow::Error::from)
                    .and_then(|f| serde_json::from_reader::<_, Overrides>(f).map_err(Into::into))
                {
                    Ok(choices) => {
                        self.pending = choices.assets.len();
                        self.choices.assets.extend(choices.assets);
                        self.status =
                            "Corrections imported; save them to rebuild the workspace.".into();
                    }
                    Err(e) => self.error = e.to_string(),
                }
            }
        });
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        if self.smoke_test {
            ctx.request_repaint_after(Duration::from_millis(50));
            if self.started.elapsed() > Duration::from_millis(800) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if self.receiver.is_none() {
            ctx.input(|i| {
                if let Some(path) = i.raw.dropped_files.iter().find_map(|f| f.path.as_ref()) {
                    self.input = path.display().to_string();
                }
            });
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.setup(ui, ctx);
            ui.separator();
            self.review(ui, ctx);
        });
    }
}
fn read_image(path: &Path) -> anyhow::Result<egui::ColorImage> {
    let mut decoder = png::Decoder::new(BufReader::new(File::open(path)?));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let info = reader.info();
    anyhow::ensure!(
        info.width <= 8192 && info.height <= 8192,
        "preview exceeds 8192 pixels per side"
    );
    let mut data = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or_else(|| anyhow::anyhow!("preview size overflow"))?
    ];
    let frame = reader.next_frame(&mut data)?;
    data.truncate(frame.buffer_size());
    let mut rgba = Vec::with_capacity(frame.width as usize * frame.height as usize * 4);
    match frame.color_type {
        png::ColorType::Rgba => rgba = data,
        png::ColorType::Rgb => {
            for p in data.as_chunks::<3>().0 {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for p in data.as_chunks::<2>().0 {
                rgba.extend_from_slice(&[p[0], p[0], p[0], p[1]]);
            }
        }
        png::ColorType::Grayscale => {
            for p in data {
                rgba.extend_from_slice(&[p, p, p, 255]);
            }
        }
        _ => anyhow::bail!("unsupported preview color type"),
    }
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [frame.width as usize, frame.height as usize],
        &rgba,
    ))
}
fn open_folder(path: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    let command = "explorer";
    #[cfg(target_os = "macos")]
    let command = "open";
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let command = "xdg-open";
    std::process::Command::new(command).arg(path).spawn()?;
    Ok(())
}
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1150.0, 900.0])
            .with_min_inner_size([850.0, 650.0]),
        ..Default::default()
    };
    let smoke_test = std::env::args().any(|arg| arg == "--smoke-test");
    let result = eframe::run_native(
        "EO-Texrip",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(App {
                smoke_test,
                ..Default::default()
            }))
        }),
    );
    if let Err(error) = &result
        && !smoke_test {
            rfd::MessageDialog::new()
                .set_title("EO-Texrip could not start")
                .set_description(format!(
                    "{error}\n\nCheck that your graphics driver supports OpenGL 3.3."
                ))
                .set_level(rfd::MessageLevel::Error)
                .show();
        }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_can_render_and_queue_confirmed_source_corrections() {
        let t = tempfile::tempdir().unwrap();
        let input = t.path().join("in");
        let output = t.path().join("out");
        eotexrip::demo::create(&input).unwrap();
        let catalog = pipeline::extract(
            &pipeline::Options {
                input,
                output: output.clone(),
                game: Game::Eou,
                title_id: Some("00040000000EC700".into()),
            },
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
        let mut app = App {
            output: output.display().to_string(),
            ..Default::default()
        };
        app.install(catalog, false);
        let ctx = egui::Context::default();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1150.0, 900.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    app.setup(ui, ctx);
                    app.review(ui, ctx);
                });
            },
        );
        app.select(0, &ctx);
        assert!(app.preview.is_some());
        app.edit_name = "confirmed name".into();
        app.edit_category = Category::Ui;
        app.queue_choice();
        assert_eq!(app.pending, 1);
        let a = &app.catalog.as_ref().unwrap().assets[0];
        assert_eq!(
            app.choices.assets[&a.id].name.as_deref(),
            Some("confirmed_name")
        );
        assert!(app.choices.assets[&a.id].confirmed);
    }
}
