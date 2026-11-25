//! Evaluasi Ilmiah Sistem Rekomendasi (Slide 15 `AI_Sistem_Rekomendasi_Visual.pdf`).
//! Memanfaatkan `RecommenderIndex` terprakomputasi untuk pengukuran latensi dan metrik
//! ablasi secara akurat pada 5 konfigurasi metode.

use crate::ai::constraints::check_hard_constraints;
use crate::ai::hybrid::{recommend_jobs_indexed, RecommenderRequest};
use crate::ai::index::RecommenderIndex;
use crate::models::{AlgorithmMode, JobPosting, UserInteraction, UserProfile};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodMetrics {
    pub method_id: String,
    pub method_label: String,
    pub apply_constraints: bool,
    pub precision_at_3: f64,
    pub precision_at_5: f64,
    pub precision_at_10: f64,
    pub recall_at_5: f64,
    pub f1_at_5: f64,
    pub ndcg_at_5: f64,
    pub ndcg_at_10: f64,
    pub hit_rate_at_5: f64,
    pub constraint_satisfaction_rate: f64,
    pub avg_latency_micros: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationReport {
    pub evaluated_users_count: usize,
    pub catalog_jobs_count: usize,
    pub total_interactions_count: usize,
    pub train_interactions_count: usize,
    pub test_interactions_count: usize,
    pub index_build_micros: u128,
    pub relevance_definition: String,
    pub methods: Vec<MethodMetrics>,
}

fn ground_truth_relevant_jobs(
    user: &UserProfile,
    jobs: &[JobPosting],
    train_seen: &HashSet<String>,
    test_ratings: &HashMap<String, f64>,
) -> HashMap<String, f64> {
    let mut rel_gains = HashMap::new();
    let u_skills: HashSet<String> = user
        .preferred_skills
        .iter()
        .map(|s| s.to_lowercase())
        .collect();
    let w = user.weights.normalized();

    for job in jobs {
        if train_seen.contains(&job.id) {
            continue;
        }
        if !check_hard_constraints(&user.constraints, job).passed {
            continue;
        }

        let matched_skills = job
            .skills
            .iter()
            .filter(|s| u_skills.contains(&s.to_lowercase()))
            .count();

        let mc_raw = w.c1_role_clarity * job.criteria.c1_role_clarity
            + w.c2_technical_challenge * job.criteria.c2_technical_challenge
            + w.c3_compensation * job.criteria.c3_compensation
            + w.c4_flexibility * job.criteria.c4_flexibility;

        if let Some(&test_r) = test_ratings.get(&job.id) {
            if test_r >= 3.8 {
                rel_gains.insert(job.id.clone(), test_r);
                continue;
            }
        }

        if matched_skills >= 2 && mc_raw >= 3.85 {
            rel_gains.insert(job.id.clone(), mc_raw);
        }
    }
    rel_gains
}

fn calc_ndcg(rec_ids: &[String], rel_gains: &HashMap<String, f64>, k: usize) -> f64 {
    let mut dcg = 0.0;
    for (idx, jid) in rec_ids.iter().take(k).enumerate() {
        if let Some(&gain) = rel_gains.get(jid) {
            let rel = (gain - 2.5).max(0.5);
            dcg += (2.0_f64.powf(rel) - 1.0) / ((idx as f64 + 2.0).log2());
        }
    }

    let mut ideal_gains: Vec<f64> = rel_gains.values().copied().collect();
    ideal_gains.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let mut idcg = 0.0;
    for (idx, gain) in ideal_gains.into_iter().take(k).enumerate() {
        let rel = (gain - 2.5).max(0.5);
        idcg += (2.0_f64.powf(rel) - 1.0) / ((idx as f64 + 2.0).log2());
    }

    if idcg <= 1e-12 {
        0.0
    } else {
        (dcg / idcg).clamp(0.0, 1.0)
    }
}

/// Menjalankan evaluasi komparatif menggunakan indeks yang sudah dibangun.
pub fn run_full_evaluation_indexed(
    users: &[UserProfile],
    jobs: &[JobPosting],
    interactions: &[UserInteraction],
    index: &RecommenderIndex,
) -> EvaluationReport {
    let train_count = interactions.iter().filter(|i| i.split != "test").count();
    let test_count = interactions.iter().filter(|i| i.split == "test").count();

    let configs = vec![
        (
            "cb_only",
            "1. Content-Based (TF-IDF + Cosine)",
            AlgorithmMode::ContentBased,
            false,
        ),
        (
            "cf_only",
            "2. Collaborative (User-Based Pearson)",
            AlgorithmMode::Collaborative,
            false,
        ),
        (
            "hybrid_classic",
            "3. Hybrid Klasik (CB + CF)",
            AlgorithmMode::HybridClassic,
            false,
        ),
        (
            "hmc_no_filter",
            "4. Hybrid Multi-Criteria (Tanpa Filter)",
            AlgorithmMode::HybridMulticriteria,
            false,
        ),
        (
            "proposed_hmc_cf",
            "5. Metode Usulan: Hybrid Multi-Criteria + Filter Syarat (HMC-CF)",
            AlgorithmMode::HybridMulticriteria,
            true,
        ),
    ];

    let mut method_results = Vec::new();

    for (m_id, m_label, mode, apply_cons) in configs {
        let mut p3_sum = 0.0;
        let mut p5_sum = 0.0;
        let mut p10_sum = 0.0;
        let mut r5_sum = 0.0;
        let mut ndcg5_sum = 0.0;
        let mut ndcg10_sum = 0.0;
        let mut hit5_sum = 0.0;
        let mut csr_sum = 0.0;
        let mut latency_sum: u128 = 0;
        let mut valid_users = 0usize;

        for u in users {
            let train_seen: HashSet<String> = interactions
                .iter()
                .filter(|i| i.user_id == u.id && i.split != "test")
                .map(|i| i.job_id.clone())
                .collect();

            let test_ratings: HashMap<String, f64> = interactions
                .iter()
                .filter(|i| i.user_id == u.id && i.split == "test")
                .map(|i| (i.job_id.clone(), i.rating))
                .collect();

            let rel_map = ground_truth_relevant_jobs(u, jobs, &train_seen, &test_ratings);
            if rel_map.is_empty() {
                continue;
            }
            valid_users += 1;

            let resp = recommend_jobs_indexed(
                RecommenderRequest {
                    user: u,
                    jobs,
                    all_users: users,
                    interactions,
                    mode,
                    apply_hard_constraints: apply_cons,
                    top_k: 10,
                    exclude_interacted_train: true,
                    include_test_in_cf: false,
                    search_query: None,
                    category_filter: None,
                },
                index,
            );

            latency_sum += resp.execution_time_micros;
            let rec_ids: Vec<String> = resp
                .recommendations
                .iter()
                .map(|r| r.job.id.clone())
                .collect();

            let hits_3 = rec_ids
                .iter()
                .take(3)
                .filter(|id| rel_map.contains_key(*id))
                .count();
            let hits_5 = rec_ids
                .iter()
                .take(5)
                .filter(|id| rel_map.contains_key(*id))
                .count();
            let hits_10 = rec_ids
                .iter()
                .take(10)
                .filter(|id| rel_map.contains_key(*id))
                .count();

            let d3 = rec_ids.len().clamp(1, 3) as f64;
            let d5 = rec_ids.len().clamp(1, 5) as f64;
            let d10 = rec_ids.len().clamp(1, 10) as f64;

            p3_sum += hits_3 as f64 / d3;
            p5_sum += hits_5 as f64 / d5;
            p10_sum += hits_10 as f64 / d10;
            r5_sum += hits_5 as f64 / (rel_map.len() as f64);
            ndcg5_sum += calc_ndcg(&rec_ids, &rel_map, 5);
            ndcg10_sum += calc_ndcg(&rec_ids, &rel_map, 10);
            if hits_5 > 0 {
                hit5_sum += 1.0;
            }

            let top5_slice: Vec<_> = resp.recommendations.iter().take(5).collect();
            if !top5_slice.is_empty() {
                let sat = top5_slice.iter().filter(|r| r.constraints.passed).count() as f64
                    / top5_slice.len() as f64;
                csr_sum += sat;
            }
        }

        let n = valid_users.max(1) as f64;
        let p5 = p5_sum / n;
        let r5 = r5_sum / n;
        let f1_5 = if (p5 + r5) > 1e-9 {
            2.0 * p5 * r5 / (p5 + r5)
        } else {
            0.0
        };

        method_results.push(MethodMetrics {
            method_id: m_id.into(),
            method_label: m_label.into(),
            apply_constraints: apply_cons,
            precision_at_3: round4(p3_sum / n),
            precision_at_5: round4(p5),
            precision_at_10: round4(p10_sum / n),
            recall_at_5: round4(r5),
            f1_at_5: round4(f1_5),
            ndcg_at_5: round4(ndcg5_sum / n),
            ndcg_at_10: round4(ndcg10_sum / n),
            hit_rate_at_5: round4(hit5_sum / n),
            constraint_satisfaction_rate: round4(csr_sum / n),
            avg_latency_micros: latency_sum / (valid_users.max(1) as u128),
        });
    }

    EvaluationReport {
        evaluated_users_count: users.len(),
        catalog_jobs_count: jobs.len(),
        total_interactions_count: interactions.len(),
        train_interactions_count: train_count,
        test_interactions_count: test_count,
        index_build_micros: index.build_time_micros,
        relevance_definition: "Lowongan di luar train split yang memenuhi syarat wajib pengguna serta disukai pada test split (rating >= 3.8) atau memiliki minimal 2 skill cocok dengan skor multikriteria >= 3.85/5.0.".into(),
        methods: method_results,
    }
}

pub fn run_full_evaluation(
    users: &[UserProfile],
    jobs: &[JobPosting],
    interactions: &[UserInteraction],
) -> EvaluationReport {
    let index = RecommenderIndex::build(jobs, users, interactions);
    run_full_evaluation_indexed(users, jobs, interactions, &index)
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
