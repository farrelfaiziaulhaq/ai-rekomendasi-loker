//! Cara 1: Content-Based Filtering (Slide 05 `AI_Sistem_Rekomendasi_Visual.pdf`)
//!
//! Mendukung fast-path memanfaatkan `RecommenderIndex` (vektor & norma lowongan sudah diprakomputasi)
//! serta kata kunci pencarian interaktif dari kolom pencarian utama.

use crate::ai::index::RecommenderIndex;
use crate::ai::tfidf::{cosine_similarity_with_norms, vector_norm};
use crate::models::{ContentBasedProof, JobPosting, UserInteraction, UserProfile};
use std::collections::{HashMap, HashSet};

/// Menyusun representasi teks satu lowongan kerja untuk indeks TF-IDF.
pub fn job_document_text(job: &JobPosting) -> String {
    let skills_joined = job.skills.join(" ");
    format!(
        "{} {} {} {} {}",
        job.title, job.category, skills_joined, skills_joined, job.description
    )
}

/// Membangun vektor profil minat pengguna (Rocchio) + pengayaan kata kunci pencarian opsional,
/// menggunakan vektor lowongan yang sudah diindeks di `RecommenderIndex` (tanpa re-tokenisasi!).
pub fn build_user_profile_vector_indexed(
    user: &UserProfile,
    interactions: &[UserInteraction],
    index: &RecommenderIndex,
    search_query: Option<&str>,
) -> (HashMap<String, f64>, f64) {
    let skills_twice = format!(
        "{} {}",
        user.preferred_skills.join(" "),
        user.preferred_skills.join(" ")
    );
    let cats = user.preferred_categories.join(" ");
    let explicit_doc = format!("{} {} {}", user.role_headline, skills_twice, cats);
    let mut profile_vec = index.corpus.vectorize(&explicit_doc);

    // Perkuat dengan sinyal riwayat lowongan yang disukai / dilamar pengguna (rating >= 3.8)
    let liked: Vec<&UserInteraction> = interactions
        .iter()
        .filter(|it| it.user_id == user.id && it.rating >= 3.8 && it.split != "test")
        .collect();

    if !liked.is_empty() {
        let scale = 0.55 / (liked.len() as f64);
        for it in liked {
            if let Some(&job_idx) = index.job_id_to_idx.get(&it.job_id) {
                let rating_weight = (it.rating / 5.0) * scale;
                for (term, &val) in &index.job_vectors[job_idx] {
                    *profile_vec.entry(term.clone()).or_insert(0.0) += val * rating_weight;
                }
            }
        }
    }

    // Jika pengguna mengetik kata kunci di kotak pencarian, perkuat bobot term pencarian pada vektor query
    if let Some(q) = search_query.map(str::trim).filter(|s| !s.is_empty()) {
        let q_vec = index.corpus.vectorize(q);
        for (term, val) in q_vec {
            *profile_vec.entry(term).or_insert(0.0) += val * 1.15;
        }
    }

    let norm = vector_norm(&profile_vec);
    (profile_vec, norm)
}

/// Menghitung skor Content-Based Filtering `S_CB(u, j)` menggunakan vektor & norma terindeks.
pub fn score_content_based_indexed(
    user: &UserProfile,
    user_skill_set: &HashSet<String>,
    user_vec: &HashMap<String, f64>,
    user_norm: f64,
    job: &JobPosting,
    job_vec: &HashMap<String, f64>,
    job_norm: f64,
) -> ContentBasedProof {
    let (tfidf_cos, dot, u_norm, j_norm, top_terms) =
        cosine_similarity_with_norms(user_vec, user_norm, job_vec, job_norm, 6);

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
