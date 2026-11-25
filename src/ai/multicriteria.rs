//! Multi-Criteria Recommender System (MCRS) — Slide 09 s.d. 17 `AI_Sistem_Rekomendasi_Visual.pdf`
//! & Referensi Utama Slide 18: Adomavicius & Kwon (2015), *Multi-Criteria Recommender Systems*.
//!
//! Mengatasi kelemahan single-rating rata-rata (dua lowongan dengan rata-rata sama 4.0 dapat
//! memiliki profil aspek yang bertolak belakang) dengan menghitung fungsi utilitas berbobot:
//!
//! `R_MC(u, j) = w_1 * C_1 + w_2 * C_2 + w_3 * C_3 + w_4 * C_4` (skala 1.0 – 5.0)
//! `S_MC(u, j) = (R_MC(u, j) - 1.0) / 4.0` (dinormalisasi ke rentang 0.0 – 1.0)

use crate::models::{CriteriaWeights, JobCriteria, MultiCriteriaProof};
use serde::{Deserialize, Serialize};

/// Menghitung skor Multi-Criteria seorang pengguna terhadap satu lowongan kerja
/// beserta jejak rumus eksplisitnya untuk ditampilkan ke pengguna / dosen.
pub fn score_multi_criteria(
    weights: &CriteriaWeights,
    criteria: &JobCriteria,
) -> MultiCriteriaProof {
    let w = weights.normalized();
    let raw = w.c1_role_clarity * criteria.c1_role_clarity
        + w.c2_technical_challenge * criteria.c2_technical_challenge
        + w.c3_compensation * criteria.c3_compensation
        + w.c4_flexibility * criteria.c4_flexibility;

    let norm = ((raw - 1.0) / 4.0).clamp(0.0, 1.0);
    let trace = format!(
        "({:.0}% × {:.1}[Kejelasan]) + ({:.0}% × {:.1}[Tantangan]) + ({:.0}% × {:.1}[Gaji]) + ({:.0}% × {:.1}[Fleksibilitas]) = {:.2} / 5.00 (Norm: {:.4})",
        w.c1_role_clarity * 100.0,
        criteria.c1_role_clarity,
        w.c2_technical_challenge * 100.0,
        criteria.c2_technical_challenge,
        w.c3_compensation * 100.0,
        criteria.c3_compensation,
        w.c4_flexibility * 100.0,
        criteria.c4_flexibility,
        raw,
        norm
    );

    MultiCriteriaProof {
        normalized_weights: CriteriaWeights {
            c1_role_clarity: round4(w.c1_role_clarity),
            c2_technical_challenge: round4(w.c2_technical_challenge),
            c3_compensation: round4(w.c3_compensation),
            c4_flexibility: round4(w.c4_flexibility),
        },
        raw_weighted_rating_1_to_5: round4(raw),
        normalized_mc_score: round4(norm),
        formula_trace: trace,
    }
}

/// Struktur hasil verifikasi langsung studi kasus pada Slide 09–17 PDF Dosen
/// agar saat presentasi mahasiswa dapat mendemonstrasikan kesesuaian 100% angka di slide vs engine Rust!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LecturerSlideVerification {
    pub item_a_name: String,
    pub item_b_name: String,
    pub item_a_clarity: f64,
    pub item_a_practice: f64,
    pub item_a_simple_avg: f64,
    pub item_b_clarity: f64,
    pub item_b_practice: f64,
    pub item_b_simple_avg: f64,
    pub dina_70_30_score_a: f64,
    pub dina_70_30_score_b: f64,
    pub dina_winner: String,
    pub bima_30_70_score_a: f64,
    pub bima_30_70_score_b: f64,
    pub bima_winner: String,
    pub pair_exercise_50_50_score_a: f64,
    pub pair_exercise_50_50_score_b: f64,
    pub min_practice_requirement: f64,
    pub item_a_passes_requirement: bool,
    pub item_b_passes_requirement: bool,
    pub pair_exercise_final_choice: String,
    pub pair_exercise_explanation: String,
}

/// Menjalankan simulasi numerik persis Slide 09, 12, 13, 14, 16, & 17 `AI_Sistem_Rekomendasi_Visual.pdf`
/// ditranslasikan ke domain Lowongan Kerja IT:
/// - Lowongan A (Junior Python Mentorship): Kejelasan = 5, Tantangan/Latihan = 3 (Rata-rata = 4.0)
/// - Lowongan B (Competitive Systems Lab): Kejelasan = 3, Tantangan/Latihan = 5 (Rata-rata = 4.0)
pub fn verify_lecturer_slide_case(
    custom_w_clarity: Option<f64>,
    custom_min_practice: Option<f64>,
) -> LecturerSlideVerification {
    let a_c = 5.0_f64;
    let a_p = 3.0_f64;
    let b_c = 3.0_f64;
    let b_p = 5.0_f64;

    // Slide 12: Dina (70% Kejelasan, 30% Latihan) -> A = 4.4, B = 3.6
    let dina_a = round2(0.7 * a_c + 0.3 * a_p);
    let dina_b = round2(0.7 * b_c + 0.3 * b_p);

    // Slide 13: Bima (30% Kejelasan, 70% Latihan) -> A = 3.6, B = 4.4
    let bima_a = round2(0.3 * a_c + 0.7 * a_p);
    let bima_b = round2(0.3 * b_c + 0.7 * b_p);

    // Slide 16–17: Latihan Berpasangan (default 50%-50%, syarat latihan/tantangan minimal 4.0)
    let w_c = custom_w_clarity.unwrap_or(0.5).clamp(0.0, 1.0);
    let w_p = 1.0 - w_c;
    let min_prac = custom_min_practice.unwrap_or(4.0);

    let ex_a = round2(w_c * a_c + w_p * a_p);
    let ex_b = round2(w_c * b_c + w_p * b_p);
    let a_pass = a_p >= min_prac;
    let b_pass = b_p >= min_prac;

    let (choice, reason) = match (a_pass, b_pass) {
        (true, true) => {
            if ex_a >= ex_b {
                (
                    "Lowongan A".to_string(),
                    format!("Kedua lowongan memenuhi syarat minimal ({min_prac:.1}), dan Lowongan A unggul dengan skor {ex_a:.2} vs {ex_b:.2}."),
                )
            } else {
                (
                    "Lowongan B".to_string(),
                    format!("Kedua lowongan memenuhi syarat minimal ({min_prac:.1}), dan Lowongan B unggul dengan skor {ex_b:.2} vs {ex_a:.2}."),
                )
            }
        }
        (false, true) => (
            "Lowongan B".to_string(),
            format!(
                "Meskipun skor berbobot A = {ex_a:.2} dan B = {ex_b:.2}, Lowongan A gugur karena aspek Tantangan/Latihan ({a_p:.1}) < syarat minimal ({min_prac:.1}). Hanya Lowongan B ({b_p:.1} >= {min_prac:.1}) yang lolos filter wajib (Slide 14 & 17)."
            ),
        ),
        (true, false) => (
            "Lowongan A".to_string(),
            format!("Hanya Lowongan A yang memenuhi syarat minimal ({min_prac:.1})."),
        ),
        (false, false) => (
            "Tidak Ada yang Lolos".to_string(),
            format!("Baik A ({a_p:.1}) maupun B ({b_p:.1}) belum mencapai syarat minimal {min_prac:.1}."),
        ),
    };

    LecturerSlideVerification {
        item_a_name: "Lowongan A: Junior Python Developer (Full Mentorship)".into(),
        item_b_name: "Lowongan B: Systems & Algorithm Engineer (Intensive Lab)".into(),
        item_a_clarity: a_c,
        item_a_practice: a_p,
        item_a_simple_avg: (a_c + a_p) / 2.0,
        item_b_clarity: b_c,
        item_b_practice: b_p,
        item_b_simple_avg: (b_c + b_p) / 2.0,
        dina_70_30_score_a: dina_a,
        dina_70_30_score_b: dina_b,
        dina_winner: if dina_a >= dina_b {
            "Lowongan A".into()
        } else {
            "Lowongan B".into()
        },
        bima_30_70_score_a: bima_a,
        bima_30_70_score_b: bima_b,
        bima_winner: if bima_b >= bima_a {
            "Lowongan B".into()
        } else {
            "Lowongan A".into()
        },
        pair_exercise_50_50_score_a: ex_a,
        pair_exercise_50_50_score_b: ex_b,
        min_practice_requirement: min_prac,
        item_a_passes_requirement: a_pass,
        item_b_passes_requirement: b_pass,
        pair_exercise_final_choice: choice,
        pair_exercise_explanation: reason,
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifikasi_angka_persis_slide_12_13_16_17_dosen() {
        let res = verify_lecturer_slide_case(None, None);
        assert_eq!(res.item_a_simple_avg, 4.0);
        assert_eq!(res.item_b_simple_avg, 4.0);
        assert_eq!(res.dina_70_30_score_a, 4.4);
        assert_eq!(res.dina_70_30_score_b, 3.6);
        assert_eq!(res.dina_winner, "Lowongan A");
        assert_eq!(res.bima_30_70_score_a, 3.6);
        assert_eq!(res.bima_30_70_score_b, 4.4);
        assert_eq!(res.bima_winner, "Lowongan B");
        assert_eq!(res.pair_exercise_50_50_score_a, 4.0);
        assert_eq!(res.pair_exercise_50_50_score_b, 4.0);
        assert!(!res.item_a_passes_requirement);
        assert!(res.item_b_passes_requirement);
        assert_eq!(res.pair_exercise_final_choice, "Lowongan B");
    }
}
