//! Struktur data domain untuk Sistem Rekomendasi Lowongan Kerja (`karir-ai`).
//! Selaras dengan konsep pada `AI_Sistem_Rekomendasi_Visual.pdf` (Slide 01–18):
//! - Profil pengguna, preferensi skill, bobot multikriteria, & syarat wajib (hard constraints)
//! - Katalog lowongan kerja beserta skor 4 aspek multikriteria
//! - Matriks interaksi pengguna (rating 1–5 & riwayat aktivitas: viewed, bookmarked, applied)
//! - Bukti penjelasan rekomendasi (Explainable AI / White-Box Proof)

use serde::{Deserialize, Serialize};

/// Empat aspek penilaian multikriteria (skala 1.0 s.d. 5.0)
/// Mengadopsi Adomavicius & Kwon (2015) serta memperluas 2 aspek pada Slide 09–13 PDF Dosen
/// (Kejelasan & Latihan/Tantangan) ke ranah rekrutmen industri IT.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobCriteria {
    /// C1: Kejelasan deskripsi peran, SOP onboarding, & bimbingan mentor (paralel aspek "Kejelasan")
    pub c1_role_clarity: f64,
    /// C2: Intensitas tantangan teknis, arsitektur, & eksplorasi proyek (paralel aspek "Latihan")
    pub c2_technical_challenge: f64,
    /// C3: Daya tarik kompensasi gaji terhadap standar industri
    pub c3_compensation: f64,
    /// C4: Fleksibilitas mode kerja (Remote / Hybrid / Onsite) & keseimbangan kerja
    pub c4_flexibility: f64,
}

impl JobCriteria {
    pub fn average(&self) -> f64 {
        (self.c1_role_clarity
            + self.c2_technical_challenge
            + self.c3_compensation
            + self.c4_flexibility)
            / 4.0
    }
}

/// Representasi satu lowongan kerja di katalog & koleksi `jobs` Cloud Firestore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobPosting {
    pub id: String,
    pub source_dataset: String,
    pub source_job_id: String,
    pub title: String,
    pub company: String,
    pub category: String,
    pub level: String,
    pub min_experience_years: u32,
    pub work_mode: String,
    pub location: String,
    pub salary_idr_millions: f64,
    pub salary_usd_annual_ref: u64,
    pub skills: Vec<String>,
    pub description: String,
    pub criteria: JobCriteria,
    pub overall_avg_rating: f64,
}

/// Bobot tingkat kepentingan tiap kriteria menurut pengguna (Slide 11–13 PDF Dosen).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriteriaWeights {
    pub c1_role_clarity: f64,
    pub c2_technical_challenge: f64,
    pub c3_compensation: f64,
    pub c4_flexibility: f64,
}

impl CriteriaWeights {
    /// Menormalisasi bobot agar jumlah ke-4 bobot selalu tepat = 1.0 (100%).
    pub fn normalized(&self) -> Self {
        let sum = self.c1_role_clarity.max(0.0)
            + self.c2_technical_challenge.max(0.0)
            + self.c3_compensation.max(0.0)
            + self.c4_flexibility.max(0.0);
        if sum <= 1e-9 {
            return Self {
                c1_role_clarity: 0.25,
                c2_technical_challenge: 0.25,
                c3_compensation: 0.25,
                c4_flexibility: 0.25,
            };
        }
        Self {
            c1_role_clarity: self.c1_role_clarity.max(0.0) / sum,
            c2_technical_challenge: self.c2_technical_challenge.max(0.0) / sum,
            c3_compensation: self.c3_compensation.max(0.0) / sum,
            c4_flexibility: self.c4_flexibility.max(0.0) / sum,
        }
    }
}

/// Syarat wajib pengguna (Hard Constraints — Slide 14, 16, 17 PDF Dosen).
/// "Skor total tinggi belum cukup jika item tidak memenuhi syarat pengguna."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardConstraints {
    pub min_salary_idr_millions: f64,
    pub max_required_experience_years: u32,
    pub allowed_work_modes: Vec<String>,
    pub min_role_clarity: f64,
    pub min_technical_challenge: f64,
}

/// Profil pencari kerja di koleksi `users` Cloud Firestore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role_headline: String,
    pub persona_tag: String,
    pub experience_years: u32,
    pub preferred_skills: Vec<String>,
    pub preferred_categories: Vec<String>,
    pub weights: CriteriaWeights,
    pub constraints: HardConstraints,
}

/// Data petunjuk interaksi pengguna (Slide 04 PDF Dosen: Rating, Pilihan Topik, Riwayat Dibuka).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteraction {
    pub id: String,
    pub user_id: String,
    pub job_id: String,
    pub rating: f64,
    /// "viewed" | "bookmarked" | "applied"
    pub action: String,
    /// "train" | "test" | "live"
    #[serde(default = "default_split")]
    pub split: String,
    pub aspect_ratings: Option<JobCriteria>,
}

fn default_split() -> String {
    "live".to_string()
}

/// Mode algoritma rekomendasi yang dapat dipilih & dibandingkan langsung di UI maupun evaluasi.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AlgorithmMode {
    /// Cara 1 (Slide 05): Content-Based Filtering (TF-IDF Cosine + Skill Overlap)
    ContentBased,
    /// Cara 2 (Slide 06): User-Based Collaborative Filtering (Pearson Neighborhood)
    Collaborative,
    /// Gabungan Klasik (Slide 08): Hybrid Content-Based + Collaborative Filtering
    HybridClassic,
    /// Metode Usulan Utama (Slide 08–14): Hybrid Multi-Criteria + Hard Constraint Filtering
    #[default]
    HybridMulticriteria,
}

/// Rincian kontribusi satu kata kunci pada perhitungan TF-IDF Cosine Similarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermContribution {
    pub term: String,
    pub tfidf_user: f64,
    pub tfidf_job: f64,
    pub product: f64,
}

/// Bukti perhitungan transparan Content-Based Filtering (Slide 05).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBasedProof {
    pub tfidf_cosine: f64,
    pub skill_overlap_ratio: f64,
    pub category_match: bool,
    pub matched_skills: Vec<String>,
    pub missing_skills: Vec<String>,
    pub top_matched_terms: Vec<TermContribution>,
    pub dot_product: f64,
    pub user_norm: f64,
    pub job_norm: f64,
    pub final_cb_score: f64,
}

/// Bukti satu pengguna tetangga yang mirip pada Collaborative Filtering (Slide 06).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarNeighborProof {
    pub neighbor_id: String,
    pub neighbor_name: String,
    pub similarity: f64,
    pub neighbor_rating_on_job: f64,
    pub neighbor_mean_rating: f64,
    pub shared_jobs_count: usize,
}

/// Bukti perhitungan transparan Collaborative Filtering (Slide 06).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborativeProof {
    pub target_user_mean: f64,
    pub predicted_rating_1_to_5: f64,
    pub normalized_cf_score: f64,
    pub has_cf_signal: bool,
    pub neighbors_used: Vec<SimilarNeighborProof>,
}

/// Bukti perhitungan transparan Multi-Criteria Scoring (Slide 09–13).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCriteriaProof {
    pub normalized_weights: CriteriaWeights,
    pub raw_weighted_rating_1_to_5: f64,
    pub normalized_mc_score: f64,
    pub formula_trace: String,
}

/// Hasil pemeriksaan syarat wajib (Slide 14).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintCheckResult {
    pub passed: bool,
    pub violations: Vec<String>,
    pub passed_checks: Vec<String>,
}

/// Satu hasil rekomendasi lowongan kerja lengkap dengan penjelasan (White-Box AI).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecommendation {
    pub rank: usize,
    pub job: JobPosting,
    pub final_score: f64,
    pub match_percentage: f64,
    pub alpha_cb_weight: f64,
    pub beta_cf_weight: f64,
    pub gamma_mc_weight: f64,
    pub content_based: ContentBasedProof,
    pub collaborative: CollaborativeProof,
    pub multi_criteria: MultiCriteriaProof,
    pub constraints: ConstraintCheckResult,
    pub human_explanation: String,
}

/// Respons lengkap mesin rekomendasi (termasuk kandidat yang gugur karena filter syarat wajib!).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationResponse {
    pub user_id: String,
    pub user_name: String,
    pub algorithm_mode: AlgorithmMode,
    pub apply_hard_constraints: bool,
    pub execution_time_micros: u128,
    pub total_catalog_jobs: usize,
    pub already_interacted_count: usize,
    pub passed_constraints_count: usize,
    pub filtered_out_count: usize,
    pub recommendations: Vec<JobRecommendation>,
    /// Kandidat berskor tinggi namun tereliminasi oleh Hard Constraints (bukti Slide 14!)
    pub filtered_out_examples: Vec<JobRecommendation>,
}
