//! Endpoint HTTP REST API (`axum`) untuk antarmuka web `KarirAI`, simulasi materi PDF dosen,
//! pembaruan preferensi/interaksi real-time ke Firebase Firestore, serta evaluasi algoritma AI.

use crate::ai::collaborative::CollaborativeMatrix;
use crate::ai::evaluation::{run_full_evaluation, EvaluationReport};
use crate::ai::hybrid::{recommend_jobs, RecommenderRequest};
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

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "engine": "KarirAI Rust Hybrid Multi-Criteria Recommender",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn firebase_status(State(store): State<Arc<DataStore>>) -> Json<FirebaseStatus> {
    Json(store.status().await)
}

async fn firebase_sync(
    State(store): State<Arc<DataStore>>,
) -> Result<Json<FirebaseStatus>, (StatusCode, String)> {
    store.sync_all_to_firestore().await.map(Json).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Sinkronisasi Firebase gagal: {e}"),
        )
    })
}

async fn reset_data(State(store): State<Arc<DataStore>>) -> Json<FirebaseStatus> {
    store.reset_to_seed().await;
    Json(store.status().await)
}

async fn list_jobs(State(store): State<Arc<DataStore>>) -> Json<Vec<JobPosting>> {
    let guard = store.state.read().await;
    Json(guard.jobs.clone())
}

async fn list_users(State(store): State<Arc<DataStore>>) -> Json<Vec<UserProfile>> {
    let guard = store.state.read().await;
    Json(guard.users.clone())
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
) -> Result<Json<UserProfile>, (StatusCode, String)> {
    let updated_user = {
        let mut guard = store.state.write().await;
        let Some(u) = guard.users.iter_mut().find(|u| u.id == id) else {
            return Err((StatusCode::NOT_FOUND, "Pengguna tidak ditemukan".into()));
        };
        u.preferred_skills = payload.preferred_skills;
        u.preferred_categories = payload.preferred_categories;
        u.experience_years = payload.experience_years;
        u.weights = payload.weights.normalized();
        u.constraints = payload.constraints;
        u.clone()
    };

    store.save_mirror().await;
    if let Ok(val) = serde_json::to_value(&updated_user) {
        let _ = store
            .upsert_firestore_doc("users", &updated_user.id, &val)
            .await;
    }

    Ok(Json(updated_user))
}

#[derive(Debug, Serialize)]
pub struct EnrichedInteraction {
    pub interaction: UserInteraction,
    pub job_title: String,
    pub company: String,
    pub category: String,
}

async fn get_user_interactions(
    Path(id): Path<String>,
    State(store): State<Arc<DataStore>>,
) -> Json<Vec<EnrichedInteraction>> {
    let guard = store.state.read().await;
    let list: Vec<EnrichedInteraction> = guard
        .interactions
        .iter()
        .filter(|it| it.user_id == id && it.split != "test")
        .filter_map(|it| {
            let job = guard.jobs.iter().find(|j| j.id == it.job_id)?;
            Some(EnrichedInteraction {
                interaction: it.clone(),
                job_title: job.title.clone(),
                company: job.company.clone(),
                category: job.category.clone(),
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
    let new_int = {
        let mut guard = store.state.write().await;
        let rating = payload.rating.clamp(1.0, 5.0);

        if let Some(existing) = guard
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
                id: format!("INT-LIVE-{:04}", guard.interactions.len() + 1),
                user_id: payload.user_id,
                job_id: payload.job_id,
                rating,
                action: payload.action,
                split: "live".to_string(),
                aspect_ratings: None,
            };
            guard.interactions.push(created.clone());
            created
        }
    };

    store.save_mirror().await;
    if let Ok(val) = serde_json::to_value(&new_int) {
        let _ = store
            .upsert_firestore_doc("interactions", &new_int.id, &val)
            .await;
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
    pub override_weights: Option<CriteriaWeights>,
    pub override_constraints: Option<HardConstraints>,
}

fn default_true() -> bool {
    true
}
fn default_top_k() -> usize {
    10
}

async fn run_recommendation(
    State(store): State<Arc<DataStore>>,
    Json(payload): Json<RecommendPayload>,
) -> Result<Json<RecommendationResponse>, (StatusCode, String)> {
    let snapshot = store.state.read().await.clone();
    let Some(mut target_user) = snapshot
        .users
        .iter()
        .find(|u| u.id == payload.user_id)
        .cloned()
    else {
        return Err((StatusCode::NOT_FOUND, "User ID tidak ditemukan".into()));
    };

    if let Some(w) = payload.override_weights {
        target_user.weights = w.normalized();
    }
    if let Some(c) = payload.override_constraints {
        target_user.constraints = c;
    }

    let resp = recommend_jobs(RecommenderRequest {
        user: &target_user,
        jobs: &snapshot.jobs,
        all_users: &snapshot.users,
        interactions: &snapshot.interactions,
        mode: payload.mode,
        apply_hard_constraints: payload.apply_hard_constraints,
        top_k: payload.top_k.clamp(1, 30),
        exclude_interacted_train: payload.exclude_interacted,
        include_test_in_cf: false,
    });

    // Catat log rekomendasi terakhir ke Firestore (non-blocking) sebagai jejak audit sistem AI
    let store_bg = store.clone();
    let log_doc_id = format!("REC-{}", resp.user_id);
    let top_titles: Vec<String> = resp
        .recommendations
        .iter()
        .take(3)
        .map(|r| format!("{} ({:.1}%)", r.job.title, r.match_percentage))
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
    let snapshot = store.state.read().await;
    let matrix = CollaborativeMatrix::build(&snapshot.users, &snapshot.interactions, false);
    let mut list = Vec::new();
    for u in &snapshot.users {
        if u.id == user_id {
            continue;
        }
        let (sim, shared) = matrix.user_similarity(&user_id, &u.id);
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
    let snapshot = store.state.read().await;
    Json(run_full_evaluation(
        &snapshot.users,
        &snapshot.jobs,
        &snapshot.interactions,
    ))
}
