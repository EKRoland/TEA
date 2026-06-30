mod best_grids_f;
mod la;
use la::App;

// ─── VERSION NATIVE (PC) ──────────────────────────────────────────
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let bests_str = include_str!("max_steps_f.toml");
    let bests: Option<best_grids_f::Root> = toml::from_str(bests_str)
        .map_err(|e| {
            eprintln!("Erreur lors du parsing des bests : {}", e);
            e
        })
        .ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("🍵🐜 | TEA, The Escaping Ant | 🍵🐜")
            .with_inner_size([1020.0, 780.0])
            .with_min_inner_size([700.0, 500.0]),
        ..Default::default()
    };

   
    eframe::run_native(
        "🍵🐜 | TEA, The Escaping Ant | 🍵🐜",
        options,
        Box::new(|_cc| Box::new(App::new(70, 70, bests))),
    )
}

// ─── VERSION WEB (WebAssembly) ──────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
fn main() {} 

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Initialise les logs dans la console du navigateur
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let bests_str = include_str!("max_steps_f.toml");
    let bests: Option<best_grids_f::Root> = toml::from_str(bests_str).ok();

    let web_options = eframe::WebOptions::default();

    // 1. On utilise le web_sys interne d'eframe pour récupérer la fenêtre et le document
    let window = eframe::web_sys::window().expect("Pas de fenêtre globale (window) disponible");
    let document = window.document().expect("Impossible de récupérer le document HTML");
    
    

    // 2. On lance l'application
    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id",
                web_options,
                Box::new(|_cc| Box::new(App::new(70, 70, bests))),
            )
            .await
            .expect("Échec du lancement d'egui sur le Web");
    });

    Ok(())
}