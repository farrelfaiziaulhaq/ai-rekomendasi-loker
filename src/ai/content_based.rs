//! Cara 1: Content-Based Filtering (Slide 05 `AI_Sistem_Rekomendasi_Visual.pdf`)
//! "Mencari isi yang mirip — Dasar rekomendasi: kemiripan isi dengan minat pengguna."
//!
//! Menggabungkan:
//! 1. Vektor TF-IDF dari profil pengguna (pilihan topik/skill eksplisit + ekspansi Rocchio dari
//!    riwayat lowongan yang disukai/dibuka pada Slide 04) terhadap deskripsi & judul lowongan.
//! 2. Kecocokan himpunan keahlian teknis eksplisit (Skill Coverage) & kesesuaian bidang karir.

use crate::ai::tfidf::{cosine_similarity_detailed, TfidfCorpus};
use crate::models::{ContentBasedProof, JobPosting, UserInteraction, UserProfile};
use std::collections::{HashMap, HashSet};

/// Menyusun representasi teks satu lowongan kerja untuk indeks TF-IDF.
/// Kata kunci skill diulang 2x agar bobot representasi teknis lebih tegas dibanding kata deskriptif umum.
pub fn job_document_text(job: &JobPosting) -> String {
    let skills_joined = job.skills.join(" ");
    format!(
        "{} {} {} {} {}",
        job.title, job.category, skills_joined, skills_joined, job.description
    )
}

/// Membangun vektor profil minat pengguna (User Profile Vector) dengan pendekatan Rocchio sederhana:
/// `V_user = 1.0 * V_explicit_profile + 0.55 * Mean(V_liked_jobs)`
pub fn build_user_profile_vector(
    user: &UserProfile,
    interactions: &[UserInteraction],
    jobs_map: &HashMap<String, &JobPosting>,
    corpus: &TfidfCorpus,
) -> HashMap<String, f64> {
    let skills_twice = format!(
        "{} {}",
        user.preferred_skills.join(" "),
        user.preferred_skills.join(" ")
    );
    let cats = user.preferred_categories.join(" ");
    let explicit_doc = format!("{} {} {}", user.role_headline, skills_twice, cats);
    let mut profile_vec = corpus.vectorize(&explicit_doc);

    // Tambahkan sinyal riwayat lowongan yang disukai / dilamar pengguna (rating >= 3.8) — Slide 04!
    let liked: Vec<&UserInteraction> = interactions
        .iter()
        .filter(|it| it.user_id == user.id && it.rating >= 3.8 && it.split != "test")
        .collect();

    if !liked.is_empty() {
        let scale = 0.55 / (liked.len() as f64);
        for it in liked {
            if let Some(job) = jobs_map.get(&it.job_id) {
                let j_vec = corpus.vectorize(&job_document_text(job));
                let rating_weight = (it.rating / 5.0) * scale;
                for (term, val) in j_vec {
                    *profile_vec.entry(term).or_insert(0.0) += val * rating_weight;
                }
            }
        }
    }

    profile_vec
}

/// Menghitung skor Content-Based Filtering `S_CB(u, j)` dalam rentang `[0.0, 1.0]`
/// beserta bukti lengkap (`ContentBasedProof`).
///
/// Rumus:
/// `S_CB(u, j) = 0.55 * Cosine_TFIDF(u, j) + 0.35 * SkillCoverage(u, j) + 0.10 * CategoryBonus(u, j)`
pub fn score_content_based(
    user: &UserProfile,
    user_vec: &HashMap<String, f64>,
    job: &JobPosting,
    job_vec: &HashMap<String, f64>,
) -> ContentBasedProof {
    let (tfidf_cos, dot, u_norm, j_norm, top_terms) =
        cosine_similarity_detailed(user_vec, job_vec, 6);

    let user_skill_set: HashSet<String> = user
        .preferred_skills
        .iter()
        .map(|s| s.to_lowercase())
        .collect();

    let mut matched_skills = Vec::new();
    let mut missing_skills = Vec::new();
    for sk in &job.skills {
        if user_skill_set.contains(&sk.to_lowercase()) {
            matched_skills.push(sk.clone());
        } else {
            missing_skills.push(sk.clone());
        }
    }

    let denom = job.skills.len().max(1) as f64;
    let skill_overlap_ratio = matched_skills.len() as f64 / denom;

    let category_match = user
        .preferred_categories
        .iter()
        .any(|c| c.eq_ignore_ascii_case(&job.category));
    let cat_bonus = if category_match { 1.0 } else { 0.0 };

    let final_cb =
        (0.55 * tfidf_cos + 0.35 * skill_overlap_ratio + 0.10 * cat_bonus).clamp(0.0, 1.0);

    ContentBasedProof {
        tfidf_cosine: round4(tfidf_cos),
        skill_overlap_ratio: round4(skill_overlap_ratio),
        category_match,
        matched_skills,
        missing_skills,
        top_matched_terms: top_terms,
        dot_product: round4(dot),
        user_norm: round4(u_norm),
        job_norm: round4(j_norm),
        final_cb_score: round4(final_cb),
    }
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
