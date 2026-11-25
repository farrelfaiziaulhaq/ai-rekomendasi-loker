//! Titik masuk utama (`main.rs`) aplikasi `KarirAI`:
//! Sistem Rekomendasi Lowongan Kerja menggunakan Rust (Axum + Tokio) dan Google Firebase.

use axum::Router;
use karir_ai::{
    ai::multicriteria::verify_lecturer_slide_case, api::api_router, firebase::DataStore,
};
use std::path::PathBuf;
use tower_http::{cors::CorsLayer, services::ServeDir};

fn locate_project_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("data").join("processed").exists() {
        return cwd;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent.join("data").join("processed").exists() {
                return parent.to_path_buf();
            }
        }
    }
    cwd
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = locate_project_root();
    let store = DataStore::init(&root).await;

    if args.iter().any(|a| a == "--reset") {
        store.reset_to_seed().await;
        println!("[OK] Data berhasil di-reset ke kondisi awal dataset terproses.");
        if args.len() == 2 {
            return;
        }
    }

    if args.iter().any(|a| a == "--case-study") {
        let cs = verify_lecturer_slide_case(None, None);
        println!("=== VERIFIKASI STUDI KASUS SLIDE PDF DOSEN (SLIDE 09 - 17) ===");
        println!(
            "Item A: {} -> Kejelasan={:.1}, Tantangan={:.1}, Rata-rata={:.1}",
            cs.item_a_name, cs.item_a_clarity, cs.item_a_practice, cs.item_a_simple_avg
        );
        println!(
            "Item B: {} -> Kejelasan={:.1}, Tantangan={:.1}, Rata-rata={:.1}",
            cs.item_b_name, cs.item_b_clarity, cs.item_b_practice, cs.item_b_simple_avg
        );
        println!(
            "Prioritas Dina (70% Kejelasan, 30% Tantangan): Skor A = {:.2}, Skor B = {:.2} => Terpilih: {}",
            cs.dina_70_30_score_a, cs.dina_70_30_score_b, cs.dina_winner
        );
        println!(
            "Prioritas Bima (30% Kejelasan, 70% Tantangan): Skor A = {:.2}, Skor B = {:.2} => Terpilih: {}",
            cs.bima_30_70_score_a, cs.bima_30_70_score_b, cs.bima_winner
        );
        println!(
            "Latihan Slide 16 (50%-50%, Syarat Tantangan >= {:.1}): Skor A={:.2}, Skor B={:.2} => {}",
            cs.min_practice_requirement,
            cs.pair_exercise_50_50_score_a,
            cs.pair_exercise_50_50_score_b,
            cs.pair_exercise_final_choice
        );
        println!("Penjelasan: {}", cs.pair_exercise_explanation);
        return;
    }

    if args.iter().any(|a| a == "--eval") {
        let report = store.evaluation_report().await;
        println!(
            "=== EVALUASI KOMPARATIF ALGORITMA AI ({} Users, {} Jobs, {} Interactions | Index build: {} us) ===",
            report.evaluated_users_count,
            report.catalog_jobs_count,
            report.total_interactions_count,
            report.index_build_micros
        );
        println!(
            "{:<64} | {:>6} | {:>6} | {:>6} | {:>6} | {:>6} | {:>7} | {:>8}",
            "Metode Algoritma", "P@3", "P@5", "R@5", "F1@5", "NDCG@5", "CSR(%)", "Lat(us)"
        );
        println!("{}", "-".repeat(126));
        for m in &report.methods {
            println!(
                "{:<64} | {:>6.4} | {:>6.4} | {:>6.4} | {:>6.4} | {:>6.4} | {:>6.1}% | {:>8}",
                m.method_label,
                m.precision_at_3,
                m.precision_at_5,
                m.recall_at_5,
                m.f1_at_5,
                m.ndcg_at_5,
                m.constraint_satisfaction_rate * 100.0,
                m.avg_latency_micros
            );
        }
        return;
    }

    let fb_status = store.status().await;
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let bind_addr = format!("0.0.0.0:{port}");

    let static_dir = root.join("static");
    let app = Router::new()
        .nest("/api", api_router(store))
        .fallback_service(ServeDir::new(static_dir).append_index_html_on_directories(true))
        .layer(CorsLayer::permissive());

    println!("==========================================================================");
    println!("  KarirAI: Sistem Rekomendasi Lowongan Kerja (Rust + Google Firebase)");
    println!("  URL Aplikasi Web  : http://127.0.0.1:{port}");
    println!("  Mode Basis Data   : {}", fb_status.mode);
    println!(
        "  Indeks TF-IDF+CF  : siap dalam {} mikrodetik",
        fb_status.index_build_micros
    );
    if let Some(ui) = fb_status.emulator_ui_url {
        println!("  Firebase UI (:4040): {ui}");
    }
    println!("==========================================================================");

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("Gagal membuka port {bind_addr}: {e}"));
    axum::serve(listener, app)
        .await
        .expect("Server Axum berhenti secara tak terduga");
}
