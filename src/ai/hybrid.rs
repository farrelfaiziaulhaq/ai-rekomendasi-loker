//! Mesin Rekomendasi Hybrid Multikriteria & Explainable AI (Slide 07–14 `AI_Sistem_Rekomendasi_Visual.pdf`).
//!
//! Mengorkestrasi ke-4 komponen kecerdasan buatan:
//! 1. Content-Based Filtering (`S_CB`) — TF-IDF Cosine + Skill Coverage
//! 2. User-Based Collaborative Filtering (`S_CF`) — Pearson Neighborhood
//! 3. Multi-Criteria Utility (`S_MC`) — Adomavicius & Kwon (2015) 4-Aspect Weighted Score
//! 4. Hard Constraint Filtering — Eliminasi syarat mutlak sebelum/saat keputusan akhir
//!
//! Rumus Hybrid Multikriteria Adaptif (dengan penanganan Cold-Start otomatis):
//! - Jika sinyal Collaborative Filtering tersedia (`has_cf_signal = true`):
//!   `S_Final = 0.40 * S_CB + 0.25 * S_CF + 0.35 * S_MC`
//! - Jika pengguna baru / belum ada tetangga pada item tersebut (`Cold-Start`):
//!   bobot `β = 0.25` diredistribusikan proporsional ke Content-Based & Multi-Criteria:
//!   `S_Final = 0.5333 * S_CB + 0.00 * S_CF + 0.4667 * S_MC`

use crate::ai::collaborative::CollaborativeMatrix;
use crate::ai::constraints::check_hard_constraints;
use crate::ai::content_based::{build_user_profile_vector, job_document_text, score_content_based};
use crate::ai::multicriteria::score_multi_criteria;
use crate::ai::tfidf::TfidfCorpus;
use crate::models::{
    AlgorithmMode, JobPosting, JobRecommendation, RecommendationResponse, UserInteraction,
    UserProfile,
};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub struct RecommenderRequest<'a> {
    pub user: &'a UserProfile,
    pub jobs: &'a [JobPosting],
    pub all_users: &'a [UserProfile],
    pub interactions: &'a [UserInteraction],
    pub mode: AlgorithmMode,
    pub apply_hard_constraints: bool,
    pub top_k: usize,
    pub exclude_interacted_train: bool,
    pub include_test_in_cf: bool,
}

pub fn recommend_jobs(req: RecommenderRequest<'_>) -> RecommendationResponse {
    let t0 = Instant::now();

    // 1. Siapkan korpus TF-IDF seluruh katalog lowongan
    let docs: Vec<String> = req.jobs.iter().map(job_document_text).collect();
    let corpus = TfidfCorpus::fit(&docs);

    let jobs_map: HashMap<String, &JobPosting> =
        req.jobs.iter().map(|j| (j.id.clone(), j)).collect();

    // 2. Bangun vektor profil minat pengguna (Slide 04 & 05)
    let user_vec = build_user_profile_vector(req.user, req.interactions, &jobs_map, &corpus);

    // 3. Bangun matriks interaksi Collaborative Filtering (Slide 06)
    let cf_matrix =
        CollaborativeMatrix::build(req.all_users, req.interactions, req.include_test_in_cf);

    // Catat lowongan yang sudah pernah dilamar/dinilai oleh user pada data train agar fokus ke kandidat baru
    let interacted_set: HashSet<String> = req
        .interactions
        .iter()
        .filter(|it| it.user_id == req.user.id && (req.include_test_in_cf || it.split != "test"))
        .map(|it| it.job_id.clone())
        .collect();

    let mut passed_list: Vec<JobRecommendation> = Vec::new();
    let mut filtered_out_list: Vec<JobRecommendation> = Vec::new();

    for (idx, job) in req.jobs.iter().enumerate() {
        if req.exclude_interacted_train && interacted_set.contains(&job.id) {
            continue;
        }

        let job_vec = corpus.vectorize(&docs[idx]);
        let cb_proof = score_content_based(req.user, &user_vec, job, &job_vec);
        let cf_proof = cf_matrix.predict_for_job(&req.user.id, &job.id, 5);
        let mc_proof = score_multi_criteria(&req.user.weights, &job.criteria);
        let cons_res = check_hard_constraints(&req.user.constraints, job);

        // Tentukan bobot kombinasi (alpha, beta, gamma) sesuai mode algoritma
        let (alpha, beta, gamma) = match req.mode {
            AlgorithmMode::ContentBased => (1.0, 0.0, 0.0),
            AlgorithmMode::Collaborative => {
                if cf_proof.has_cf_signal {
                    (0.0, 1.0, 0.0)
                } else {
                    // Fallback minimal agar urutan deterministik saat murni CF mengalami cold-start
                    (0.25, 0.0, 0.0)
                }
            }
            AlgorithmMode::HybridClassic => {
                if cf_proof.has_cf_signal {
                    (0.60, 0.40, 0.0)
                } else {
                    (1.0, 0.0, 0.0)
                }
            }
            AlgorithmMode::HybridMulticriteria => {
                if cf_proof.has_cf_signal {
                    (0.40, 0.25, 0.35)
                } else {
                    // Redistribusi otomatis saat Cold-Start
                    (0.40 / 0.75, 0.0, 0.35 / 0.75)
                }
            }
        };

        let final_score = (alpha * cb_proof.final_cb_score
            + beta * cf_proof.normalized_cf_score
            + gamma * mc_proof.normalized_mc_score)
            .clamp(0.0, 1.0);

        let explanation = build_human_explanation(
            job,
            &cb_proof,
            &cf_proof,
            &mc_proof,
            &cons_res,
            req.mode,
            final_score,
        );

        let rec = JobRecommendation {
            rank: 0,
            job: job.clone(),
            final_score: round4(final_score),
            match_percentage: ((final_score * 1000.0).round() / 10.0).clamp(0.0, 100.0),
            alpha_cb_weight: round4(alpha),
            beta_cf_weight: round4(beta),
            gamma_mc_weight: round4(gamma),
            content_based: cb_proof,
            collaborative: cf_proof,
            multi_criteria: mc_proof,
            constraints: cons_res.clone(),
            human_explanation: explanation,
        };

        if !req.apply_hard_constraints || cons_res.passed {
            passed_list.push(rec);
        } else {
            filtered_out_list.push(rec);
        }
    }

    let cmp_desc = |a: &JobRecommendation, b: &JobRecommendation| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    };

    passed_list.sort_by(cmp_desc);
    filtered_out_list.sort_by(cmp_desc);

    let passed_count = passed_list.len();
    let filtered_count = filtered_out_list.len();

    for (i, item) in passed_list.iter_mut().enumerate() {
        item.rank = i + 1;
    }
    passed_list.truncate(req.top_k);

    for (i, item) in filtered_out_list.iter_mut().enumerate() {
        item.rank = i + 1;
    }
    filtered_out_list.truncate(5);

    RecommendationResponse {
        user_id: req.user.id.clone(),
        user_name: req.user.name.clone(),
        algorithm_mode: req.mode,
        apply_hard_constraints: req.apply_hard_constraints,
        execution_time_micros: t0.elapsed().as_micros(),
        total_catalog_jobs: req.jobs.len(),
        already_interacted_count: interacted_set.len(),
        passed_constraints_count: passed_count,
        filtered_out_count: filtered_count,
        recommendations: passed_list,
        filtered_out_examples: filtered_out_list,
    }
}

fn build_human_explanation(
    job: &JobPosting,
    cb: &crate::models::ContentBasedProof,
    cf: &crate::models::CollaborativeProof,
    mc: &crate::models::MultiCriteriaProof,
    cons: &crate::models::ConstraintCheckResult,
    mode: AlgorithmMode,
    final_score: f64,
) -> String {
    let mut parts = Vec::new();
    if !cb.matched_skills.is_empty() {
        parts.push(format!(
            "Cocok dengan {} keahlian Anda ({})",
            cb.matched_skills.len(),
            cb.matched_skills
                .iter()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else {
        parts.push(format!(
            "Kemiripan konten TF-IDF {:.0}%",
            cb.tfidf_cosine * 100.0
        ));
    }

    if cf.has_cf_signal && mode != AlgorithmMode::ContentBased {
        let top_n = &cf.neighbors_used[0];
        parts.push(format!(
            "Disukai pengguna berpola serupa seperti {} (sim {:.2}, rating {:.1}/5)",
            top_n.neighbor_name, top_n.similarity, top_n.neighbor_rating_on_job
        ));
    }

    if mode == AlgorithmMode::HybridMulticriteria {
        parts.push(format!(
            "Skor multikriteria {:.2}/5.00 sesuai bobot prioritas Anda",
            mc.raw_weighted_rating_1_to_5
        ));
    }

    if !cons.passed {
        parts.push(format!(
            "Namun tereliminasi syarat wajib: {}",
            cons.violations.join("; ")
        ));
    } else {
        parts.push(format!(
            "Lolos seluruh syarat wajib ({}, Rp {:.1} Jt/bln) — kecocokan total {:.1}%",
            job.work_mode,
            job.salary_idr_millions,
            final_score * 100.0
        ));
    }

    parts.join(" • ")
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
