//! Indeks Prakomputasi (`RecommenderIndex`) untuk optimasi latensi sub-milidetik.
//!
//! Menghindari tokenisasi ulang, pembangunan korpus TF-IDF ulang, dan kalkulasi ulang
//! matriks kemiripan Pearson pada setiap request HTTP. Dibangun sekali saat startup dan
//! diperbarui atomik hanya ketika ada mutasi data (interaksi baru / edit preferensi / reset).

use crate::ai::collaborative::CollaborativeMatrix;
use crate::ai::content_based::job_document_text;
use crate::ai::tfidf::{vector_norm, TfidfCorpus};
use crate::models::{JobPosting, UserInteraction, UserProfile};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct RecommenderIndex {
    pub corpus: TfidfCorpus,
    pub job_vectors: Vec<HashMap<String, f64>>,
    pub job_norms: Vec<f64>,
    pub job_search_haystacks: Vec<String>,
    pub job_id_to_idx: HashMap<String, usize>,
    pub cf_matrix_train: CollaborativeMatrix,
    pub build_time_micros: u128,
}

impl RecommenderIndex {
    pub fn build(
        jobs: &[JobPosting],
        users: &[UserProfile],
        interactions: &[UserInteraction],
    ) -> Self {
        let t0 = Instant::now();

        let docs: Vec<String> = jobs.iter().map(job_document_text).collect();
        let corpus = TfidfCorpus::fit(&docs);

        let mut job_vectors = Vec::with_capacity(jobs.len());
        let mut job_norms = Vec::with_capacity(jobs.len());
        let mut job_search_haystacks = Vec::with_capacity(jobs.len());
        let mut job_id_to_idx = HashMap::with_capacity(jobs.len());

        for (idx, (job, doc)) in jobs.iter().zip(docs.iter()).enumerate() {
            let vec = corpus.vectorize(doc);
            let norm = vector_norm(&vec);
            job_vectors.push(vec);
            job_norms.push(norm);
            job_search_haystacks.push(
                format!(
                    "{} {} {} {} {} {} {}",
                    job.title,
                    job.company,
                    job.category,
                    job.level,
                    job.work_mode,
                    job.location,
                    job.skills.join(" ")
                )
                .to_lowercase(),
            );
            job_id_to_idx.insert(job.id.clone(), idx);
        }

        let cf_matrix_train = CollaborativeMatrix::build(users, interactions, false);

        Self {
            corpus,
            job_vectors,
            job_norms,
            job_search_haystacks,
            job_id_to_idx,
            cf_matrix_train,
            build_time_micros: t0.elapsed().as_micros(),
        }
    }
}
