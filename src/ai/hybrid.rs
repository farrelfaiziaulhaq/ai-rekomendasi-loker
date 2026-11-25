//! Mesin Rekomendasi Hybrid Multikriteria & Explainable AI (Slide 07-14 `AI_Sistem_Rekomendasi_Visual.pdf`).
//!
//! Dioptimalkan menggunakan `RecommenderIndex` yang telah diprakomputasi sehingga waktu
//! komputasi per permintaan turun ke orde mikrodetik (sub-milidetik) di Rust.

use crate::ai::constraints::check_hard_constraints;
use crate::ai::content_based::{build_user_profile_vector_indexed, score_content_based_indexed};
use crate::ai::index::RecommenderIndex;
use crate::ai::multicriteria::score_multi_criteria;
use crate::models::{
    AlgorithmMode, JobPosting, JobRecommendation, RecommendationResponse, UserInteraction,
    UserProfile,
};
use std::collections::HashSet;
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
    pub search_query: Option<&'a str>,
    pub category_filter: Option<&'a str>,
}

/// Menjalankan mesin rekomendasi memanfaatkan `RecommenderIndex` terprakomputasi (O(N_jobs) fast path).
pub fn recommend_jobs_indexed(
    req: RecommenderRequest<'_>,
    index: &RecommenderIndex,
) -> RecommendationResponse {
    let t0 = Instant::now();

    // 1. Bangun vektor profil pengguna (+ kata kunci pencarian bila diisi)
    let (user_vec, user_norm) =
        build_user_profile_vector_indexed(req.user, req.interactions, index, req.search_query);

    let user_skill_set: HashSet<String> = req
        .user
        .preferred_skills
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    // Token pencarian (case-insensitive) untuk filter teks langsung seperti portal loker
    let query_tokens: Vec<String> = req
        .search_query
        .map(|q| {
            q.to_lowercase()
                .split_whitespace()
                .filter(|t| !t.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let category_filter = req.category_filter.map(str::trim).filter(|c| !c.is_empty());

    // Catat lowongan yang sudah pernah dilamar/dinilai user pada data train
    let interacted_set: HashSet<&str> = req
        .interactions
        .iter()
        .filter(|it| it.user_id == req.user.id && (req.include_test_in_cf || it.split != "test"))
        .map(|it| it.job_id.as_str())
        .collect();

    let mut passed_list: Vec<JobRecommendation> = Vec::with_capacity(req.jobs.len());
    let mut filtered_out_list: Vec<JobRecommendation> = Vec::new();

    for (idx, job) in req.jobs.iter().enumerate() {
        if req.exclude_interacted_train && interacted_set.contains(job.id.as_str()) {
            continue;
        }

        if let Some(cat) = category_filter {
            if !job.category.eq_ignore_ascii_case(cat) {
                continue;
            }
        }

        if !query_tokens.is_empty() {
            let haystack = &index.job_search_haystacks[idx];
            if !query_tokens.iter().all(|t| haystack.contains(t)) {
                continue;
            }
        }

        let cb_proof = score_content_based_indexed(
            req.user,
            &user_skill_set,
            &user_vec,
            user_norm,
            job,
            &index.job_vectors[idx],
            index.job_norms[idx],
        );
        let cf_proof = index
            .cf_matrix_train
            .predict_for_job(&req.user.id, &job.id, 5);
        let mc_proof = score_multi_criteria(&req.user.weights, &job.criteria);
        let cons_res = check_hard_constraints(&req.user.constraints, job);

        let (alpha, beta, gamma) = match req.mode {
            AlgorithmMode::ContentBased => (1.0, 0.0, 0.0),
            AlgorithmMode::Collaborative => {
                if cf_proof.has_cf_signal {
                    (0.0, 1.0, 0.0)
                } else {
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

/// Wrapper kompatibel untuk pemanggilan langsung tanpa indeks tersimpan (membangun indeks sekali).
pub fn recommend_jobs(req: RecommenderRequest<'_>) -> RecommendationResponse {
    let index = RecommenderIndex::build(req.jobs, req.all_users, req.interactions);
    recommend_jobs_indexed(req, &index)
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
            "serupa pilihan {} (rating {:.1}/5)",
            top_n.neighbor_name, top_n.neighbor_rating_on_job
        ));
    }

    if mode == AlgorithmMode::HybridMulticriteria {
        parts.push(format!(
            "nilai prioritas {:.2}/5",
            mc.raw_weighted_rating_1_to_5
        ));
    }

    if !cons.passed {
        parts.push(format!(
            "tidak lolos syarat wajib ({})",
            cons.violations.join("; ")
        ));
    } else {
        parts.push(format!(
            "sesuai syarat {}, Rp {:.1} Jt/bln (kecocokan {:.0}%)",
            job.work_mode,
            job.salary_idr_millions,
            final_score * 100.0
        ));
    }

    parts.join(", ")
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
