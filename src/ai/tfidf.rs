//! Implementasi murni (from-scratch) Tokenisasi Teks Dwibahasa (Indonesia + Inggris),
//! pembobotan TF-IDF (Term Frequency - Inverse Document Frequency), dan Cosine Similarity.
//!
//! Digunakan pada tahap Content-Based Filtering (Slide 05 `AI_Sistem_Rekomendasi_Visual.pdf`).

use crate::models::TermContribution;
use std::collections::{HashMap, HashSet};

const STOPWORDS: &[&str] = &[
    // Bahasa Indonesia
    "dan",
    "atau",
    "yang",
    "untuk",
    "pada",
    "dari",
    "dengan",
    "dalam",
    "ke",
    "di",
    "ini",
    "itu",
    "akan",
    "oleh",
    "sebagai",
    "serta",
    "dapat",
    "kami",
    "anda",
    "tim",
    "posisi",
    "membuka",
    "kandidat",
    "berfokus",
    "standar",
    "kerja",
    "mode",
    "divisi",
    // Bahasa Inggris
    "the",
    "and",
    "for",
    "with",
    "from",
    "that",
    "this",
    "are",
    "you",
    "our",
    "will",
    "your",
    "have",
    "has",
    "not",
    "but",
    "all",
    "can",
    "more",
    "about",
    "into",
    "their",
    "who",
    "what",
    "when",
    "where",
    "role",
    "work",
    "working",
    "team",
    "company",
    "job",
    "looking",
    "join",
    "experience",
    "years",
    "using",
    "build",
    "building",
    "across",
];

/// Mengubah teks mentah menjadi daftar token bersih (huruf kecil, mempertahankan simbol teknis seperti `c++`, `ci/cd`, `tf-idf`).
pub fn tokenize(text: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOPWORDS.iter().copied().collect();
    text.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '+' || c == '#' || c == '-' || c == '/'))
        .map(|s| s.trim_matches(|c: char| c == '-' || c == '/'))
        .filter(|s| s.len() >= 2 && !stop.contains(s))
        .filter(|s| !s.chars().all(|c| c.is_ascii_digit()))
        .map(String::from)
        .collect()
}

/// Korpus TF-IDF yang menyimpan nilai IDF tiap kata dari seluruh katalog lowongan kerja.
#[derive(Debug, Clone)]
pub struct TfidfCorpus {
    pub doc_count: usize,
    pub idf: HashMap<String, f64>,
}

impl TfidfCorpus {
    /// Membangun kamus IDF dari kumpulan dokumen lowongan kerja:
    /// `IDF(t) = ln((1 + N) / (1 + df(t))) + 1.0` (Smoothed IDF agar tidak pernah membagi nol).
    pub fn fit(documents: &[String]) -> Self {
        let n = documents.len();
        let mut df: HashMap<String, usize> = HashMap::new();

        for doc in documents {
            let unique_terms: HashSet<String> = tokenize(doc).into_iter().collect();
            for term in unique_terms {
                *df.entry(term).or_insert(0) += 1;
            }
        }

        let mut idf = HashMap::with_capacity(df.len());
        for (term, freq) in df {
            let score = ((1.0 + n as f64) / (1.0 + freq as f64)).ln() + 1.0;
            idf.insert(term, score);
        }

        Self { doc_count: n, idf }
    }

    /// Mengubah satu teks menjadi vektor jarang (sparse vector) TF-IDF:
    /// `TF(t, d) = count(t, d) / total_terms(d)`
    /// `TF-IDF(t, d) = TF(t, d) * IDF(t)`
    pub fn vectorize(&self, text: &str) -> HashMap<String, f64> {
        let tokens = tokenize(text);
        if tokens.is_empty() {
            return HashMap::new();
        }
        let total = tokens.len() as f64;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for t in tokens {
            *counts.entry(t).or_insert(0) += 1;
        }

        let default_idf = ((1.0 + self.doc_count as f64) / 1.0).ln() + 1.0;
        let mut vec = HashMap::with_capacity(counts.len());
        for (term, c) in counts {
            let tf = c as f64 / total;
            let idf = *self.idf.get(&term).unwrap_or(&default_idf);
            vec.insert(term, tf * idf);
        }
        vec
    }
}

/// Menghitung Cosine Similarity antara dua vektor TF-IDF beserta rincian bukti matematisnya:
/// `Cosine(A, B) = (A · B) / (||A|| * ||B||)`
pub fn cosine_similarity_detailed(
    vec_user: &HashMap<String, f64>,
    vec_job: &HashMap<String, f64>,
    top_n_terms: usize,
) -> (f64, f64, f64, f64, Vec<TermContribution>) {
    let norm_u = vec_user.values().map(|v| v * v).sum::<f64>().sqrt();
    let norm_j = vec_job.values().map(|v| v * v).sum::<f64>().sqrt();

    if norm_u <= 1e-12 || norm_j <= 1e-12 {
        return (0.0, 0.0, norm_u, norm_j, Vec::new());
    }

    let mut dot = 0.0;
    let mut contributions = Vec::new();

    for (term, &w_u) in vec_user {
        if let Some(&w_j) = vec_job.get(term) {
            let prod = w_u * w_j;
            dot += prod;
            contributions.push(TermContribution {
                term: term.clone(),
                tfidf_user: (w_u * 10000.0).round() / 10000.0,
                tfidf_job: (w_j * 10000.0).round() / 10000.0,
                product: (prod * 10000.0).round() / 10000.0,
            });
        }
    }

    contributions.sort_by(|a, b| {
        b.product
            .partial_cmp(&a.product)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    contributions.truncate(top_n_terms);

    let cosine = (dot / (norm_u * norm_j)).clamp(0.0, 1.0);
    (cosine, dot, norm_u, norm_j, contributions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tfidf_cosine_identik_bernilai_satu() {
        let docs = vec![
            "rust backend engineer tokio axum firestore".to_string(),
            "python data analyst sql tableau pandas".to_string(),
        ];
        let corpus = TfidfCorpus::fit(&docs);
        let v1 = corpus.vectorize(&docs[0]);
        let (cos, _, _, _, terms) = cosine_similarity_detailed(&v1, &v1, 5);
        assert!((cos - 1.0).abs() < 1e-9);
        assert!(!terms.is_empty());
    }

    #[test]
    fn tfidf_dokumen_berbeda_total_bernilai_nol() {
        let docs = vec![
            "rust tokio axum".to_string(),
            "tableau excel akuntansi".to_string(),
        ];
        let corpus = TfidfCorpus::fit(&docs);
        let v1 = corpus.vectorize(&docs[0]);
        let v2 = corpus.vectorize(&docs[1]);
        let (cos, _, _, _, _) = cosine_similarity_detailed(&v1, &v2, 5);
        assert_eq!(cos, 0.0);
    }
}
