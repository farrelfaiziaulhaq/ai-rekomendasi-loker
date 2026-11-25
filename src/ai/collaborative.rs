//! Cara 2: User-Based Collaborative Filtering (Slide 06 `AI_Sistem_Rekomendasi_Visual.pdf`)
//! "Melihat pengguna yang mirip — Karena pola pilihannya mirip, item menjadi kandidat."
//!
//! Menggunakan Mean-Centered Pearson Correlation (setara Adjusted Cosine antar-pengguna)
//! pada matriks interaksi Pengguna × Lowongan:
//!
//! `Sim(u, v) = Σ_{i ∈ I_uv} (r_{u,i} - r̄_u)(r_{v,i} - r̄_v) / (√Σ(r_{u,i} - r̄_u)² * √Σ(r_{v,i} - r̄_v)²)`
//!
//! Prediksi rating pengguna `u` terhadap lowongan `j`:
//! `r̂_CF(u, j) = r̄_u + [ Σ_{v ∈ N(u;j)} Sim(u, v) * (r_{v,j} - r̄_v) ] / [ Σ_{v ∈ N(u;j)} |Sim(u, v)| ]`

use crate::models::{CollaborativeProof, SimilarNeighborProof, UserInteraction, UserProfile};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CollaborativeMatrix {
    /// `ratings[user_id][job_id] = rating (1.0 .. 5.0)`
    pub ratings: HashMap<String, HashMap<String, f64>>,
    /// Rata-rata rating tiap pengguna `r̄_u`
    pub user_means: HashMap<String, f64>,
    /// Nama tampilan pengguna untuk keperluan Explainable AI
    pub user_names: HashMap<String, String>,
}

impl CollaborativeMatrix {
    /// Membangun matriks interaksi dari data historis (mengabaikan split `"test"` agar evaluasi jujur!).
    pub fn build(
        users: &[UserProfile],
        interactions: &[UserInteraction],
        include_test: bool,
    ) -> Self {
        let mut ratings: HashMap<String, HashMap<String, f64>> = HashMap::new();
        let mut user_names = HashMap::new();

        for u in users {
            user_names.insert(u.id.clone(), u.name.clone());
            ratings.entry(u.id.clone()).or_default();
        }

        for it in interactions {
            if !include_test && it.split == "test" {
                continue;
            }
            ratings
                .entry(it.user_id.clone())
                .or_default()
                .insert(it.job_id.clone(), it.rating);
        }

        let mut user_means = HashMap::new();
        for (uid, map) in &ratings {
            if map.is_empty() {
                user_means.insert(uid.clone(), 3.5);
            } else {
                let sum: f64 = map.values().sum();
                user_means.insert(uid.clone(), sum / map.len() as f64);
            }
        }

        Self {
            ratings,
            user_means,
            user_names,
        }
    }

    /// Menghitung kemiripan pola rating antara pengguna `u` dan pengguna `v` dalam rentang `[-1.0, 1.0]`,
    /// beserta jumlah lowongan yang sama-sama pernah dinilai (`shared_count`).
    pub fn user_similarity(&self, user_u: &str, user_v: &str) -> (f64, usize) {
        let (Some(map_u), Some(map_v)) = (self.ratings.get(user_u), self.ratings.get(user_v))
        else {
            return (0.0, 0);
        };
        let mean_u = *self.user_means.get(user_u).unwrap_or(&3.5);
        let mean_v = *self.user_means.get(user_v).unwrap_or(&3.5);

        let mut shared = Vec::new();
        for (job_id, &r_u) in map_u {
            if let Some(&r_v) = map_v.get(job_id) {
                shared.push((r_u, r_v));
            }
        }

        let shared_count = shared.len();
        if shared_count == 0 {
            return (0.0, 0);
        }

        let mut num = 0.0;
        let mut den_u = 0.0;
        let mut den_v = 0.0;
        for &(r_u, r_v) in &shared {
            let du = r_u - mean_u;
            let dv = r_v - mean_v;
            num += du * dv;
            den_u += du * du;
            den_v += dv * dv;
        }

        let raw_sim = if den_u <= 1e-9 || den_v <= 1e-9 {
            // Jika seluruh rating pada irisan bernilai identik konstan (misal sama-sama 5.0 seperti Slide 06 Dina & Bima!),
            // gunakan kesamaan jarak rata-rata mutlak agar pola identik tetap terdeteksi positif!
            let avg_diff =
                shared.iter().map(|(ru, rv)| (ru - rv).abs()).sum::<f64>() / (shared_count as f64);
            (1.0 - avg_diff / 4.0).clamp(0.0, 1.0)
        } else {
            (num / (den_u.sqrt() * den_v.sqrt())).clamp(-1.0, 1.0)
        };

        // Significance weighting ringan agar pengguna yang hanya kebetulan beririsan 1 item tidak mendominasi berlebihan
        let sig_factor = (shared_count as f64 / 3.0).min(1.0);
        (raw_sim * sig_factor, shared_count)
    }

    /// Memprediksi ketertarikan pengguna `target_user` terhadap lowongan `job_id` menggunakan Top-K tetangga terdekat.
    pub fn predict_for_job(
        &self,
        target_user: &str,
        job_id: &str,
        top_k_neighbors: usize,
    ) -> CollaborativeProof {
        let mean_u = *self.user_means.get(target_user).unwrap_or(&3.5);
        let mut candidates: Vec<SimilarNeighborProof> = Vec::new();

        for (other_uid, other_map) in &self.ratings {
            if other_uid == target_user {
                continue;
            }
            let Some(&r_vj) = other_map.get(job_id) else {
                continue;
            };
            let (sim, shared_cnt) = self.user_similarity(target_user, other_uid);
            if sim > 0.05 && shared_cnt >= 1 {
                let mean_v = *self.user_means.get(other_uid).unwrap_or(&3.5);
                let name = self
                    .user_names
                    .get(other_uid)
                    .cloned()
                    .unwrap_or_else(|| other_uid.clone());
                candidates.push(SimilarNeighborProof {
                    neighbor_id: other_uid.clone(),
                    neighbor_name: name,
                    similarity: round4(sim),
                    neighbor_rating_on_job: r_vj,
                    neighbor_mean_rating: round4(mean_v),
                    shared_jobs_count: shared_cnt,
                });
            }
        }

        candidates.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(top_k_neighbors);

        if candidates.is_empty() {
            return CollaborativeProof {
                target_user_mean: round4(mean_u),
                predicted_rating_1_to_5: round4(mean_u),
                normalized_cf_score: 0.0,
                has_cf_signal: false,
                neighbors_used: Vec::new(),
            };
        }

        let mut weighted_dev = 0.0;
        let mut sim_sum = 0.0;
        for n in &candidates {
            weighted_dev += n.similarity * (n.neighbor_rating_on_job - n.neighbor_mean_rating);
            sim_sum += n.similarity.abs();
        }

        let predicted = (mean_u + (weighted_dev / sim_sum.max(1e-9))).clamp(1.0, 5.0);
        let norm_score = ((predicted - 1.0) / 4.0).clamp(0.0, 1.0);

        CollaborativeProof {
            target_user_mean: round4(mean_u),
            predicted_rating_1_to_5: round4(predicted),
            normalized_cf_score: round4(norm_score),
            has_cf_signal: true,
            neighbors_used: candidates,
        }
    }
}

fn round4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CriteriaWeights, HardConstraints};

    fn dummy_user(id: &str, name: &str) -> UserProfile {
        UserProfile {
            id: id.into(),
            name: name.into(),
            email: format!("{id}@test.id"),
            role_headline: "Dev".into(),
            persona_tag: "".into(),
            experience_years: 1,
            preferred_skills: vec![],
            preferred_categories: vec![],
            weights: CriteriaWeights {
                c1_role_clarity: 0.25,
                c2_technical_challenge: 0.25,
                c3_compensation: 0.25,
                c4_flexibility: 0.25,
            },
            constraints: HardConstraints {
                min_salary_idr_millions: 0.0,
                max_required_experience_years: 10,
                allowed_work_modes: vec!["Remote".into()],
                min_role_clarity: 0.0,
                min_technical_challenge: 0.0,
            },
        }
    }

    #[test]
    fn skenario_slide_06_dina_dan_bima_pola_mirip() {
        // Slide 06 PDF Dosen:
        // Dina: Python dasar = Suka (5.0), Logika = Suka (5.0), Latihan Python = Belum mencoba
        // Bima: Python dasar = Suka (5.0), Logika = Suka (5.0), Latihan Python = Suka (5.0)
        let users = vec![dummy_user("DINA", "Dina"), dummy_user("BIMA", "Bima")];
        let interactions = vec![
            UserInteraction {
                id: "1".into(),
                user_id: "DINA".into(),
                job_id: "PY_DASAR".into(),
                rating: 5.0,
                action: "applied".into(),
                split: "train".into(),
                aspect_ratings: None,
            },
            UserInteraction {
                id: "2".into(),
                user_id: "DINA".into(),
                job_id: "LOGIKA".into(),
                rating: 5.0,
                action: "applied".into(),
                split: "train".into(),
                aspect_ratings: None,
            },
            UserInteraction {
                id: "3".into(),
                user_id: "BIMA".into(),
                job_id: "PY_DASAR".into(),
                rating: 5.0,
                action: "applied".into(),
                split: "train".into(),
                aspect_ratings: None,
            },
            UserInteraction {
                id: "4".into(),
                user_id: "BIMA".into(),
                job_id: "LOGIKA".into(),
                rating: 5.0,
                action: "applied".into(),
                split: "train".into(),
                aspect_ratings: None,
            },
            UserInteraction {
                id: "5".into(),
                user_id: "BIMA".into(),
                job_id: "LATIHAN_PY".into(),
                rating: 5.0,
                action: "applied".into(),
                split: "train".into(),
                aspect_ratings: None,
            },
        ];
        let matrix = CollaborativeMatrix::build(&users, &interactions, false);
        let proof = matrix.predict_for_job("DINA", "LATIHAN_PY", 5);
        assert!(proof.has_cf_signal);
        assert_eq!(proof.neighbors_used[0].neighbor_id, "BIMA");
        assert!(proof.predicted_rating_1_to_5 >= 4.9);
    }
}
