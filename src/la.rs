//! # Langton's Ant – Rust + egui
//!
//! Grid: n rows × m cols;  0 = white,  1 = black
//! Ant:  (row, col, dir)   dir ∈ {0=North, 1=East, 2=South, 3=West}
//!
//! Rule:
//!   white cell → turn right (+1 mod 4), flip black,  move forward
//!   black cell → turn left  (+3 mod 4), flip white,  move forward

use crate::best_grids_f::Solution;
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

// ─── Direction table ──────────────────────────────────────────────────────────

const DELTAS: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];

/// ASCII arrows — guaranteed to render in any font.
/// 0=North  1=East  2=South  3=West
const ARROWS: [char; 4] = ['^', '>', 'v', '<'];

// ─── Simulation ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct LangtonsAnt {
    pub rows: usize,
    pub cols: usize,
    /// Flat row-major buffer: 0 = white, 1 = black, 2 = gray.
    pub grid: Vec<u8>,
    /// (row, col, direction)
    pub ant: (isize, isize, u8),
    pub stopped: bool,
    pub step_count: u64,
}

impl LangtonsAnt {
    pub fn new(rows: usize, cols: usize) -> Self {
        let rows = rows.max(2);
        let cols = cols.max(2);
        Self {
            rows,
            cols,
            grid: vec![0; rows * cols],
            ant: (rows as isize / 2, cols as isize / 2, 0),
            stopped: false,
            step_count: 0,
        }
    }

    pub fn from_solution(rows: usize, cols: usize, sol: &Solution) -> Self {
        let mut grid = Vec::with_capacity(rows * cols);

        for row in &sol.grid {
            for c in row.chars() {
                let v = match c {
                    '0' => 0,
                    '1' => 1,
                    '2' => 2,
                    _ => panic!("Invalid grid char: {}", c),
                };
                grid.push(v);
            }
        }

        Self {
            rows,
            cols,
            grid,
            ant: (sol.ant[0], sol.ant[1], sol.ant[2] as u8),
            stopped: false,
            step_count: 0,
        }
    }

    #[inline]
    pub fn in_bounds(&self, r: isize, c: isize) -> bool {
        r >= 0 && r < self.rows as isize && c >= 0 && c < self.cols as isize
    }

    #[inline]
    fn idx(&self, r: isize, c: isize) -> usize {
        r as usize * self.cols + c as usize
    }

    #[inline]
    pub fn cell(&self, r: isize, c: isize) -> u8 {
        self.grid[self.idx(r, c)]
    }

    #[inline]
    pub fn set_cell(&mut self, r: isize, c: isize, v: u8) {
        let i = self.idx(r, c);
        self.grid[i] = v;
    }

    /// One simulation step. Returns `true` while the ant is still inside.
    pub fn step(&mut self) -> bool {
        if self.stopped {
            return false;
        }
        let (r, c, d) = self.ant;
        if !self.in_bounds(r, c) {
            self.stopped = true;
            return false;
        }

        // Flip cell
        let color = self.cell(r, c);
        self.set_cell(r, c, 1 - color);

        // Turn: white→right (+1),  black→left (+3)
        let new_dir: u8 = if color == 0 { (d + 1) & 3 } else { (d + 3) & 3 };

        // Move
        let (dr, dc) = DELTAS[new_dir as usize];
        let nr = r + dr;
        let nc = c + dc;
        self.ant = (nr, nc, new_dir);
        self.step_count += 1;

        if !self.in_bounds(nr, nc) {
            self.stopped = true;
            return false;
        }
        true
    }

    /// Run up to `n` steps (stops early if ant exits).
    pub fn run_n(&mut self, n: usize) {
        for _ in 0..n {
            if !self.step() {
                break;
            }
        }
    }
}

// ─── App ─────────────────────────────────────────────────────────────────────

pub struct App {
    sim: LangtonsAnt,
    snapshot: LangtonsAnt,
    running: bool,
    /// Steps per rendered frame (continuous run).
    steps_per_frame: usize,
    cell_size: f32,
    /// false = click toggles cell colour; true = click moves ant.
    ant_mode: bool,
    /// Grid resize inputs.
    input_rows: String,
    input_cols: String,
    /// "Run N steps" input field.
    run_n_input: String,
    /// Status message shown after a "Run N" action.
    run_n_status: String,
    /// Stored configurations loaded from TOML.
    pub bests: Option<crate::best_grids_f::Root>,
}

impl App {
    pub fn new(rows: usize, cols: usize, bests: Option<crate::best_grids_f::Root>) -> Self {
        let sim = LangtonsAnt::new(rows, cols);
        let snapshot = sim.clone();
        let cell_size = (600.0 / rows.max(cols) as f32).clamp(6.0, 40.0);
        App {
            sim,
            snapshot,
            running: false,
            steps_per_frame: 50,
            cell_size,
            ant_mode: false,
            input_rows: rows.to_string(),
            input_cols: cols.to_string(),
            run_n_input: String::new(),
            run_n_status: String::new(),
            bests,
        }
    }

    fn take_snapshot(&mut self) {
        self.snapshot = self.sim.clone();
    }

    fn reset(&mut self) {
        self.sim = self.snapshot.clone();
        self.running = false;
        self.run_n_status.clear();
    }

    fn resize_grid(&mut self) {
        let rows = self.input_rows.trim().parse::<usize>().unwrap_or(70).max(2);
        let cols = self.input_cols.trim().parse::<usize>().unwrap_or(70).max(2);
        self.sim = LangtonsAnt::new(rows, cols);
        self.snapshot = self.sim.clone();
        self.running = false;
        self.cell_size = (600.0 / rows.max(cols) as f32).clamp(4.0, 40.0);
        self.run_n_status.clear();
    }

    /// Execute exactly N steps (or until the ant exits), then stop auto-run.
    fn do_run_n(&mut self) {
        self.running = false;
        match self.run_n_input.trim().parse::<u64>() {
            Ok(0) => {
                self.run_n_status = "Enter a number > 0.".to_string();
            }
            Ok(n) => {
                let before = self.sim.step_count;
                self.sim.run_n(n as usize);
                let done = self.sim.step_count - before;
                self.run_n_status = if self.sim.stopped {
                    format!("Ran {} steps, then ant exited (total {}).", done, self.sim.step_count)
                } else {
                    format!("Ran {} steps (total {}).", done, self.sim.step_count)
                };
            }
            Err(_) => {
                self.run_n_status = "Invalid number.".to_string();
            }
        }
    }

    /// Helper to load a selected configuration solution
    fn load_config(&mut self, rows: usize, cols: usize, sol: &Solution) {
        self.sim = LangtonsAnt::from_solution(rows, cols, sol);
        self.snapshot = self.sim.clone();
        self.running = false;
        self.cell_size = (600.0 / rows.max(cols) as f32).clamp(4.0, 40.0);
        self.input_rows = rows.to_string();
        self.input_cols = cols.to_string();
        self.run_n_status.clear();
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ─── Personnalisation des polices pour plus de visibilité ───
        let mut style = (*ctx.style()).clone();
        
        // On modifie l'épaisseur et la taille des styles par défaut
        style.text_styles.insert(
            egui::TextStyle::Body,
            FontId::new(14.0, egui::FontFamily::Proportional) // Légèrement plus grand (14.0 au lieu de 12.5)
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            FontId::new(14.0, egui::FontFamily::Proportional) // Boutons plus lisibles
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            FontId::new(19.0, egui::FontFamily::Proportional) // Titres plus imposants
        );
        
        // Optionnel : Augmenter l'épaisseur des bordures des boutons/widgets pour le contraste
        style.visuals.widgets.inactive.bg_stroke.width = 1.2;
        style.visuals.widgets.hovered.bg_stroke.width = 1.5;
        style.visuals.widgets.active.bg_stroke.width = 1.5;
        
        ctx.set_style(style);
        // ─────────────────────────────────────────────────────────────
        
        // Continuous execution
        if self.running && !self.sim.stopped {
            self.sim.run_n(self.steps_per_frame);
            ctx.request_repaint();
        }

        // Variable locale indispensable pour stocker la configuration à charger
        let mut pending_load = None;

        // ── Left Side Panel (Controls + Best Configurations Matrix) ──────────
        egui::SidePanel::left("control_panel")
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.heading("Simulation Controls");
                ui.separator();

                // L1: Run, Pause, Step, Multisteps
                ui.horizontal(|ui| {
                    let not_stopped = !self.sim.stopped;
                    if ui.add_enabled(not_stopped && !self.running, egui::Button::new("Run")).clicked() {
                        self.running = true;
                    }
                    if ui.add_enabled(self.running, egui::Button::new("Pause")).clicked() {
                        self.running = false;
                    }
                    if ui.add_enabled(not_stopped, egui::Button::new("Step x1")).clicked() {
                        self.running = false;
                        self.sim.step();
                        self.run_n_status.clear();
                    }
                    
                    ui.separator();
                    
                    ui.label("Steps x");
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.run_n_input)
                            .desired_width(50.0)
                            .hint_text("N..."),
                    );
                    if ui.add_enabled(not_stopped, egui::Button::new("Jump")).clicked()
                        || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        self.do_run_n();
                    }
                });

                ui.add_space(6.0);

                // L2: Toggle Cell, Move ant, Grid resize&reset
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.ant_mode, false, "Toggle Cell");
                    ui.selectable_value(&mut self.ant_mode, true, "Move Ant");
                    
                    ui.separator();
                    
                    ui.add(egui::TextEdit::singleline(&mut self.input_rows).desired_width(25.0));
                    ui.label("x");
                    ui.add(egui::TextEdit::singleline(&mut self.input_cols).desired_width(25.0));
                    if ui.button("Resize").clicked() {
                        self.resize_grid();
                    }
                });

                ui.add_space(6.0);

                // L3: Snapshot, Reset, Speed, Zoom
                ui.horizontal(|ui| {
                    if ui.button("Snapshot").clicked() {
                        self.running = false;
                        self.take_snapshot();
                    }
                    if ui.button("Reset").clicked() {
                        self.reset();
                    }
                    
                    ui.separator();
                    
                    ui.label("Spd:");
                    ui.add(
                        egui::Slider::new(&mut self.steps_per_frame, 1..=10_000)
                            .logarithmic(true)
                            .show_value(false),
                    );
                    ui.label("Zm:");
                    ui.add(egui::Slider::new(&mut self.cell_size, 3.0..=80.0).show_value(false));
                });

                ui.add_space(6.0);

                // L4: Steps and current info (Enlarged and Highlighted)
                ui.add_space(4.0);
                let status_is_error = self.sim.stopped;
                let frame_bg = if status_is_error {
                    Color32::from_rgb(60, 15, 15)
                } else if self.running {
                    Color32::from_rgb(15, 45, 15)
                } else {
                    Color32::from_rgb(35, 35, 35)
                };

                egui::Frame::none()
                    .fill(frame_bg)
                    .inner_margin(8.0)
                    .rounding(4.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            let status_str = if self.sim.stopped {
                                "OUT OF BOUNDS"
                            } else if self.running {
                                "RUNNING"
                            } else {
                                "PAUSED"
                            };

                            let text_color = if status_is_error {
                                Color32::LIGHT_RED
                            } else if self.running {
                                Color32::LIGHT_GREEN
                            } else {
                                Color32::LIGHT_BLUE
                            };

                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new(
                                    egui::RichText::new(format!("Steps: {}", self.sim.step_count))
                                        .font(FontId::proportional(16.0))
                                        .strong()
                                        .color(Color32::GOLD)
                                ));
                                ui.label(egui::RichText::new("|").font(FontId::proportional(16.0)).color(Color32::GRAY));
                                ui.add(egui::Label::new(
                                    egui::RichText::new(status_str)
                                        .font(FontId::proportional(16.0))
                                        .strong()
                                        .color(text_color)
                                ));
                            });

                            ui.add_space(2.0);

                            ui.horizontal(|ui| {
                                ui.add(egui::Label::new(
                                    egui::RichText::new(format!("Cell: ({}, {})", self.sim.ant.0, self.sim.ant.1))
                                        .font(FontId::monospace(12.0))
                                        .color(Color32::LIGHT_GRAY)
                                ));
                                ui.label("|");
                                ui.add(egui::Label::new(
                                    egui::RichText::new(format!("Dir: {}", ARROWS[self.sim.ant.2 as usize]))
                                        .font(FontId::monospace(12.0))
                                        .strong()
                                        .color(Color32::WHITE)
                                ));
                            });
                        });
                    });

                if !self.run_n_status.is_empty() {
                    ui.add_space(2.0);
                    ui.colored_label(Color32::from_rgb(30, 140, 30), &self.run_n_status);
                }

                ui.add_space(15.0);
                ui.heading("Escaping Time Configurations");
                ui.separator();

                // On extrait sécurisé l'option self.bests ici pour l'utiliser en dessous
                if let Some(bests) = &self.bests {
                    let disponible = ui.available_height() - 100.0;
                    
                    egui::ScrollArea::vertical()
                        .max_height(disponible)
                        .show(ui, |ui| {
                            egui::Grid::new("best_grids_matrix")
                                .striped(true)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    for r in 3..=9 {
                                        ui.label(format!(" {}x ", r));
                                    }
                                    ui.end_row();

                                    for max_c in 3..=20 {
                                        let mut row_has_data = false;
                                        for r in 3..=9 {
                                            if bests.best_grids.iter().any(|g| g.rows == r && g.cols == max_c) {
                                                row_has_data = true;
                                                break;
                                            }
                                        }

                                        if row_has_data {
                                            for r in 3..=9 {
                                                if let Some(grid_block) = bests.best_grids.iter().find(|g| g.rows == r && g.cols == max_c) {
                                                    let count = grid_block.solution.len();
                                                    let btn_text = format!("{}×{}", r, max_c);
                                                    
                                                    if count == 1 {
                                                        if ui.button(btn_text).clicked() {
                                                            pending_load = Some((grid_block.rows, grid_block.cols, grid_block.solution[0].clone()));
                                                        }
                                                    } else if count > 1 {
                                                        let menu_id = ui.make_persistent_id(format!("menu_{}_{}", r, max_c));
                                                        let button_response = ui.button(btn_text);
                                                        
                                                        if button_response.hovered() || button_response.clicked() {
                                                            ui.memory_mut(|mem| mem.open_popup(menu_id));
                                                        }

                                                        egui::popup_below_widget(ui, menu_id, &button_response,  |ui| {
                                                            ui.set_min_width(100.0);
                                                            for sol in &grid_block.solution {
                                                                if ui.button(&sol.name).clicked() {
                                                                    pending_load = Some((grid_block.rows, grid_block.cols, sol.clone()));
                                                                }
                                                            }
                                                        });
                                                    } else {
                                                        ui.label(""); 
                                                    }
                                                } else {
                                                    ui.label(""); 
                                                }
                                            }
                                            ui.end_row();
                                        }
                                    }
                                });
                        });

                    if let Some((r, c, sol)) = pending_load {
                        self.load_config(r, c, &sol);
                    }

                    // 2. Zone basse (About the Algorithm) accrochée en bas
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button("The paper📄 ").on_hover_text("Link to the paper with description of the algorithm").clicked() {
                                ui.ctx().output_mut(|o| {
                                    o.open_url = Some(egui::output::OpenUrl {
                                        url: "https://arxiv.org/abs/2606.26677".to_string(),
                                        new_tab: true,
                                    });
                                });
                            }

                            if ui.button("💾 Simulation Code (.zip)").on_hover_text("Télécharger le code source du projet").clicked() {
                                ui.ctx().output_mut(|o| {
                                    o.open_url = Some(egui::output::OpenUrl {
                                        url: "https://github.com/EKRoland/TEA/tree/main/codes".to_string(),
                                        new_tab: false,
                                    });
                                });
                            }
                        });

                        ui.separator();
                        ui.heading("Resources");
                        ui.add_space(5.0);
                    });

                } else {
                    ui.colored_label(Color32::LIGHT_RED, "No TOML configuration loaded.");
                }
            }); // Fin du SidePanel

        // ── Central Panel (Centered Simulation Grid Canvas) ──────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let cs = self.cell_size;
                    let rows = self.sim.rows;
                    let cols = self.sim.cols;
                    
                    let desired_size = Vec2::new(cols as f32 * cs, rows as f32 * cs);
                    
                    let available_size = ui.available_size();
                    let extra_space = (available_size - desired_size).max(Vec2::ZERO);
                    
                    ui.allocate_space(Vec2::new(available_size.x, extra_space.y * 0.5));
                    
                    ui.horizontal(|ui| {
                        ui.allocate_space(Vec2::new(extra_space.x * 0.5, desired_size.y));
                        
                        let (response, painter) = ui.allocate_painter(desired_size, Sense::click());
                        let origin = response.rect.min;

                        for r in 0..rows {
                            for c in 0..cols {
                                let fill = match self.sim.grid[r * cols + c] {
                                    0 => Color32::WHITE,
                                    1 => Color32::BLACK,
                                    2 => Color32::from_gray(120),
                                    _ => Color32::WHITE,
                                };
                                let rect = Rect::from_min_size(
                                    Pos2::new(origin.x + c as f32 * cs, origin.y + r as f32 * cs),
                                    Vec2::splat(cs),
                                );
                                painter.rect_filled(rect, 0.0, fill);
                                if cs >= 5.0 {
                                    painter.rect_stroke(
                                        rect,
                                        0.0,
                                        Stroke::new(0.4, Color32::from_gray(170)),
                                    );
                                }
                            }
                        }

                        let (ar, ac, ad) = self.sim.ant;
                        if self.sim.in_bounds(ar, ac) {
                            let center = Pos2::new(
                                origin.x + ac as f32 * cs + cs * 0.5,
                                origin.y + ar as f32 * cs + cs * 0.5,
                            );
                            painter.circle_filled(center, cs * 0.42, Color32::from_rgb(220, 30, 30));
                            painter.text(
                                center,
                                egui::Align2::CENTER_CENTER,
                                ARROWS[ad as usize].to_string(),
                                FontId::monospace(cs * 0.60),
                                Color32::WHITE,
                            );
                        }

                        if response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let c = ((pos.x - origin.x) / cs) as isize;
                                let r = ((pos.y - origin.y) / cs) as isize;
                                if self.sim.in_bounds(r, c) {
                                    if self.ant_mode {
                                        let (ar, ac, ad) = self.sim.ant;
                                        if ar == r && ac == c {
                                            self.sim.ant = (ar, ac, (ad + 1) & 3);
                                        } else {
                                            self.sim.ant = (r, c, self.sim.ant.2);
                                            self.sim.stopped = false;
                                        }
                                    } else {
                                        let idx = r as usize * cols + c as usize;
                                        let current_val = self.sim.grid[idx];
                                        if current_val == 2 {
                                            self.sim.grid[idx] = 0;
                                        } else {
                                            self.sim.grid[idx] = 1 - current_val;
                                        }
                                    }
                                }
                            }
                        }
                    });
                });
        });
    }
}