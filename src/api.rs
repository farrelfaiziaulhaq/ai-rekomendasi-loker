//! Endpoint HTTP REST API (`axum`) untuk KarirAI.
//! Seluruh endpoint baca memanfaatkan `Arc` snapshot dan `RecommenderIndex` terprakomputasi
//! agar latensi pencarian & geser slider terasa instan di browser.

use crate::ai::evaluation::EvaluationReport;
use crate::ai::hybrid::{recommend_jobs_indexed, RecommenderRequest};
use crate::ai::multicriteria::{verify_lecturer_slide_case, LecturerSlideVerification};
use crate::firebase::{DataStore, FirebaseStatus};
use crate::models::{
    AlgorithmMode, CriteriaWeights, HardConstraints, JobPosting, RecommendationResponse,
    UserInteraction, UserProfile,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

pub fn api_router(store: Arc<DataStore>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/firebase/status", get(firebase_status))
        .route("/firebase/sync", post(firebase_sync))
        .route("/reset", post(reset_data))
        .route("/jobs", get(list_jobs))
        .route("/users", get(list_users))
        .route("/users/{id}/preferences", put(update_user_preferences))
        .route("/users/{id}/interactions", get(get_user_interactions))
        .route("/interactions", post(record_interaction))
        .route("/recommend", post(run_recommendation))
        .route("/cf-matrix/{user_id}", get(get_cf_neighbors_summary))
        .route("/lecturer-case-study", get(lecturer_case_study))
        .route("/evaluate", get(evaluate_all_methods))
        .with_state(store)
}

async fn health(State(store): State<Arc<DataStore>>) -> Json<serde_json::Value> {
    let idx = store.index().await;
    Json(json!({
        "status": "ok",
        "engine": "KarirAI Rust Hybrid Multi-Criteria Recommender",
        "version": env!("CARGO_PKG_VERSION"),
        "indexed_jobs": idx.job_vectors.len(),
        "index_build_micros": idx.build_time_micros
    }))
}

async fn firebase_status(State(store): State<Arc<DataStore>>) -> Json<FirebaseStatus> {
    Json(store.status().await)
}

async fn firebase_sync(
    State(store): State<Arc<DataStore>>,
) -> Result<Json<FirebaseStatus>, (StatusCode, Json<serde_json::Value>)> {
    store.sync_all_to_firestore().await.map(Json).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Sinkronisasi Firebase gagal: {e}") })),
        )
    })
}

async fn reset_data(State(store): State<Arc<DataStore>>) -> Json<FirebaseStatus> {
    store.reset_to_seed().await;
    Json(store.status().await)
}

async fn list_jobs(State(store): State<Arc<DataStore>>) -> Json<Vec<JobPosting>> {
    let snap = store.snapshot().await;
    Json(snap.jobs.clone())
}

async fn list_users(State(store): State<Arc<DataStore>>) -> Json<Vec<UserProfile>> {
    let snap = store.snapshot().await;
    Json(snap.users.clone())
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesPayload {
    pub preferred_skills: Vec<String>,
    pub preferred_categories: Vec<String>,
    pub experience_years: u32,
    pub weights: CriteriaWeights,
    pub constraints: HardConstraints,
}

async fn update_user_preferences(
    Path(id): Path<String>,
    State(store): State<Arc<DataStore>>,
    Json(payload): Json<UpdatePreferencesPayload>,
) -> Result<Json<UserProfile>, (StatusCode, Json<serde_json::Value>)> {
    let updated_opt = store
        .mutate_state(|st| {
            let u = st.users.iter_mut().find(|u| u.id == id)?;
            u.preferred_skills = payload.preferred_skills;
            u.preferred_categories = payload.preferred_categories;
            u.experience_years = payload.experience_years;
            u.weights = payload.weights.normalized();
            u.constraints = payload.constraints;
            Some(u.clone())
        })
        .await;

    let Some(updated_user) = updated_opt else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Pengguna tidak ditemukan" })),
        ));
    };

    if let Ok(val) = serde_json::to_value(&updated_user) {
        let store_bg = store.clone();
        let uid = updated_user.id.clone();
        tokio::spawn(async move {
            let _ = store_bg.upsert_firestore_doc("users", &uid, &val).await;
        });
    }

    Ok(Json(updated_user))
}

#[derive(Debug, Serialize)]
pub struct EnrichedInteraction {
    pub interaction: UserInteraction,
    pub job_title: String,
    pub company: String,
    pub category: String,
    pub location: String,
    pub work_mode: String,
    pub salary_idr_millions: f64,
}

async fn get_user_interactions(
    Path(id): Path<String>,
    State(store): State<Arc<DataStore>>,
) -> Json<Vec<EnrichedInteraction>> {
    let snap = store.snapshot().await;
    let list: Vec<EnrichedInteraction> = snap
        .interactions
        .iter()
        .filter(|it| it.user_id == id && it.split != "test")
        .filter_map(|it| {
            let job = snap.jobs.iter().find(|j| j.id == it.job_id)?;
            Some(EnrichedInteraction {
                interaction: it.clone(),
                job_title: job.title.clone(),
                company: job.company.clone(),
                category: job.category.clone(),
                location: job.location.clone(),
                work_mode: job.work_mode.clone(),
                salary_idr_millions: job.salary_idr_millions,
            })
        })
        .collect();
    Json(list)
}

#[derive(Debug, Deserialize)]
pub struct RecordInteractionPayload {
    pub user_id: String,
    pub job_id: String,
    pub rating: f64,
    pub action: String,
}

async fn record_interaction(
    State(store): State<Arc<DataStore>>,
    Json(payload): Json<RecordInteractionPayload>,
) -> Json<UserInteraction> {
    let new_int = store
        .mutate_state(|st| {
            let rating = payload.rating.clamp(1.0, 5.0);
            if let Some(existing) = st
                .interactions
                .iter_mut()
                .find(|i| i.user_id == payload.user_id && i.job_id == payload.job_id)
            {
                existing.rating = rating;
                existing.action = payload.action.clone();
                existing.split = "live".to_string();
                existing.clone()
            } else {
                let created = UserInteraction {
                    id: format!("INT-LIVE-{:04}", st.interactions.len() + 1),
                    user_id: payload.user_id,
                    job_id: payload.job_id,
                    rating,
                    action: payload.action,
                    split: "live".to_string(),
                    aspect_ratings: None,
                };
                st.interactions.push(created.clone());
                created
            }
        })
        .await;

    if let Ok(val) = serde_json::to_value(&new_int) {
        let store_bg = store.clone();
        let doc_id = new_int.id.clone();
        tokio::spawn(async move {
            let _ = store_bg
                .upsert_firestore_doc("interactions", &doc_id, &val)
                .await;
        });
    }

    Json(new_int)
}

#[derive(Debug, Deserialize)]
pub struct RecommendPayload {
    pub user_id: String,
    #[serde(default)]
    pub mode: AlgorithmMode,
    #[serde(default = "default_true")]
    pub apply_hard_constraints: bool,
    #[serde(default = "default_top_k")]
    pub top_k: usize,
    #[serde(default = "default_true")]
    pub exclude_interacted: bool,
    pub search_query: Option<String>,
    pub category_filter: Option<String>,
    pub override_weights: Option<CriteriaWeights>,
    pub override_constraints: Option<HardConstraints>,
}

fn default_true() -> bool {
    true
}
fn default_top_k() -> usize {
    12
}

async fn run_recommendation(
    State(store): State<Arc<DataStore>>,
    Json(payload): Json<RecommendPayload>,
) -> Result<Json<RecommendationResponse>, (StatusCode, Json<serde_json::Value>)> {
    let snap = store.snapshot().await;
    let index = store.index().await;

    let Some(mut target_user) = snap.users.iter().find(|u| u.id == payload.user_id).cloned() else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Profil pencari kerja tidak ditemukan" })),
        ));
    };

    if let Some(w) = payload.override_weights {
        target_user.weights = w.normalized();
    }
    if let Some(c) = payload.override_constraints {
        target_user.constraints = c;
    }

    let resp = recommend_jobs_indexed(
        RecommenderRequest {
            user: &target_user,
            jobs: &snap.jobs,
            all_users: &snap.users,
            interactions: &snap.interactions,
            mode: payload.mode,
            apply_hard_constraints: payload.apply_hard_constraints,
            top_k: payload.top_k.clamp(1, 40),
            exclude_interacted_train: payload.exclude_interacted,
            include_test_in_cf: false,
            search_query: payload.search_query.as_deref(),
            category_filter: payload.category_filter.as_deref(),
        },
        &index,
    );

    let store_bg = store.clone();
    let log_doc_id = format!("REC-{}", resp.user_id);
    let top_titles: Vec<String> = resp
        .recommendations
        .iter()
        .take(3)
        .map(|r| format!("{} ({:.0}%)", r.job.title, r.match_percentage))
        .collect();
    let mode_str = format!("{:?}", resp.algorithm_mode);
    let latency = resp.execution_time_micros;
    tokio::spawn(async move {
        let _ = store_bg
            .upsert_firestore_doc(
                "recommendation_logs",
                &log_doc_id,
                &json!({
                    "user_id": log_doc_id,
                    "mode": mode_str,
                    "execution_time_micros": latency,
                    "top_3_jobs": top_titles,
                }),
            )
            .await;
    });

    Ok(Json(resp))
}

#[derive(Debug, Serialize)]
pub struct NeighborOverview {
    pub neighbor_id: String,
    pub neighbor_name: String,
    pub role_headline: String,
    pub similarity: f64,
    pub shared_jobs_count: usize,
}

async fn get_cf_neighbors_summary(
    Path(user_id): Path<String>,
    State(store): State<Arc<DataStore>>,
) -> Json<Vec<NeighborOverview>> {
    let snap = store.snapshot().await;
    let index = store.index().await;
    let mut list = Vec::new();
    for u in &snap.users {
        if u.id == user_id {
            continue;
        }
        let (sim, shared) = index.cf_matrix_train.user_similarity(&user_id, &u.id);
        list.push(NeighborOverview {
            neighbor_id: u.id.clone(),
            neighbor_name: u.name.clone(),
            role_headline: u.role_headline.clone(),
            similarity: (sim * 10000.0).round() / 10000.0,
            shared_jobs_count: shared,
        });
    }
    list.sort_by(|a, b| {
        b.similarity
            .partial_cmp(&a.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Json(list)
}

#[derive(Debug, Deserialize)]
pub struct CaseStudyQuery {
    pub w_clarity: Option<f64>,
    pub min_practice: Option<f64>,
}

async fn lecturer_case_study(Query(q): Query<CaseStudyQuery>) -> Json<LecturerSlideVerification> {
    Json(verify_lecturer_slide_case(q.w_clarity, q.min_practice))
}

async fn evaluate_all_methods(State(store): State<Arc<DataStore>>) -> Json<EvaluationReport> {
    let report = store.evaluation_report().await;
    Json((*report).clone())
}
