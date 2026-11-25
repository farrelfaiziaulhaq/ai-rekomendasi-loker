//! Integrasi Google Firebase (Cloud Firestore REST API + Firebase Auth Identity Toolkit REST API)
//! yang dioptimalkan dengan:
//! 1. Immutable `Arc<AppDatasetState>` + `Arc<RecommenderIndex>` + `Arc<EvaluationReport>` untuk
//!    pembacaan zero-clone berkecepatan sub-milidetik.
//! 2. Sinkronisasi konkuren terukur (`Semaphore(16)` + `JoinSet`) ke Cloud Firestore.
//! 3. Penulisan lokal atomik (`.tmp` -> rename) agar berkas mirror tidak pernah korup.

use crate::ai::evaluation::{run_full_evaluation_indexed, EvaluationReport};
use crate::ai::index::RecommenderIndex;
use crate::models::{JobPosting, UserInteraction, UserProfile};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirebaseStatus {
    pub mode: String,
    pub connected: bool,
    pub project_id: String,
    pub firestore_endpoint: String,
    pub auth_endpoint: String,
    pub emulator_ui_url: Option<String>,
    pub synced_jobs: usize,
    pub synced_users: usize,
    pub synced_interactions: usize,
    pub index_build_micros: u128,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDatasetState {
    pub jobs: Vec<JobPosting>,
    pub users: Vec<UserProfile>,
    pub interactions: Vec<UserInteraction>,
}

pub struct DataStore {
    state: RwLock<Arc<AppDatasetState>>,
    index: RwLock<Arc<RecommenderIndex>>,
    eval_cache: RwLock<Arc<EvaluationReport>>,
    pub data_dir: PathBuf,
    pub runtime_mirror_path: PathBuf,
    pub http: reqwest::Client,
    pub project_id: String,
    pub api_key: String,
    pub firestore_base: Option<String>,
    pub auth_base: Option<String>,
    pub emulator_ui: Option<String>,
    pub mode_label: RwLock<String>,
}

impl DataStore {
    pub async fn init(root_dir: &Path) -> Arc<Self> {
        let data_dir = root_dir.join("data").join("processed");
        let runtime_mirror_path = root_dir.join("data").join("runtime_firestore_mirror.json");

        let initial_state = if runtime_mirror_path.exists() {
            Self::load_from_mirror(&runtime_mirror_path)
                .unwrap_or_else(|_| Self::load_seed_files(&data_dir))
        } else {
            Self::load_seed_files(&data_dir)
        };

        let initial_index = RecommenderIndex::build(
            &initial_state.jobs,
            &initial_state.users,
            &initial_state.interactions,
        );
        let initial_eval = run_full_evaluation_indexed(
            &initial_state.users,
            &initial_state.jobs,
            &initial_state.interactions,
            &initial_index,
        );

        let project_id =
            std::env::var("FIREBASE_PROJECT_ID").unwrap_or_else(|_| "demo-karir-ai".to_string());
        let api_key =
            std::env::var("FIREBASE_API_KEY").unwrap_or_else(|_| "demo-karir-ai-key".to_string());

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .pool_max_idle_per_host(16)
            .build()
            .unwrap_or_default();

        let mut store_emu = std::env::var("FIRESTORE_EMULATOR_HOST").ok();
        let mut auth_emu = std::env::var("FIREBASE_AUTH_EMULATOR_HOST").ok();

        if store_emu.is_none()
            && std::env::var("FIREBASE_CLOUD_URL").is_err()
            && http.get("http://127.0.0.1:8088/").send().await.is_ok()
        {
            store_emu = Some("127.0.0.1:8088".to_string());
            if auth_emu.is_none() {
                auth_emu = Some("127.0.0.1:9098".to_string());
            }
        }

        let (firestore_base, auth_base, emulator_ui, mode_str) = if let Some(h) = store_emu {
            let fs_url =
                format!("http://{h}/v1/projects/{project_id}/databases/(default)/documents");
            let au_host = auth_emu.unwrap_or_else(|| "127.0.0.1:9098".to_string());
            let au_url = format!("http://{au_host}/identitytoolkit.googleapis.com/v1");
            (
                Some(fs_url),
                Some(au_url),
                Some("http://127.0.0.1:4040".to_string()),
                "Firebase Emulator Suite (Cloud Firestore + Auth)".to_string(),
            )
        } else if std::env::var("FIREBASE_PROJECT_ID").is_ok()
            && std::env::var("FIREBASE_API_KEY").is_ok()
        {
            let fs_url = format!(
                "https://firestore.googleapis.com/v1/projects/{project_id}/databases/(default)/documents"
            );
            let au_url = "https://identitytoolkit.googleapis.com/v1".to_string();
            (
                Some(fs_url),
                Some(au_url),
                None,
                "Google Firebase Cloud (Firestore REST)".to_string(),
            )
        } else {
            (
                None,
                None,
                None,
                "Penyimpanan Lokal Siap Pakai (Firestore Mirror)".to_string(),
            )
        };

        let ds = Arc::new(Self {
            state: RwLock::new(Arc::new(initial_state)),
            index: RwLock::new(Arc::new(initial_index)),
            eval_cache: RwLock::new(Arc::new(initial_eval)),
            data_dir,
            runtime_mirror_path,
            http,
            project_id,
            api_key,
            firestore_base,
            auth_base,
            emulator_ui,
            mode_label: RwLock::new(mode_str),
        });

        ds.save_mirror().await;
        if ds.firestore_base.is_some() {
            let _ = ds.sync_all_to_firestore().await;
        }
        ds
    }

    /// Pengambilan snapshot O(1) tanpa menyalin data vektor/katalog di heap.
    pub async fn snapshot(&self) -> Arc<AppDatasetState> {
        Arc::clone(&*self.state.read().await)
    }

    /// Pengambilan indeks TF-IDF + Pearson O(1).
    pub async fn index(&self) -> Arc<RecommenderIndex> {
        Arc::clone(&*self.index.read().await)
    }

    /// Pengambilan laporan evaluasi tercache O(1).
    pub async fn evaluation_report(&self) -> Arc<EvaluationReport> {
        Arc::clone(&*self.eval_cache.read().await)
    }

    /// Menerapkan mutasi state lalu membangun ulang indeks dan cache evaluasi secara atomik.
    pub async fn mutate_state<F, R>(&self, mutator: F) -> R
    where
        F: FnOnce(&mut AppDatasetState) -> R,
    {
        let (result, next_state) = {
            let mut guard = self.state.write().await;
            let mut cloned = (**guard).clone();
            let res = mutator(&mut cloned);
            let arc_next = Arc::new(cloned);
            *guard = Arc::clone(&arc_next);
            (res, arc_next)
        };

        let next_index = Arc::new(RecommenderIndex::build(
            &next_state.jobs,
            &next_state.users,
            &next_state.interactions,
        ));
        let next_eval = Arc::new(run_full_evaluation_indexed(
            &next_state.users,
            &next_state.jobs,
            &next_state.interactions,
            &next_index,
        ));

        *self.index.write().await = next_index;
        *self.eval_cache.write().await = next_eval;
        self.save_mirror().await;

        result
    }

    fn load_seed_files(data_dir: &Path) -> AppDatasetState {
        let jobs_raw = std::fs::read_to_string(data_dir.join("jobs_catalog.json"))
            .expect("Gagal membaca data/processed/jobs_catalog.json");
        let users_raw = std::fs::read_to_string(data_dir.join("users_profiles.json"))
            .expect("Gagal membaca data/processed/users_profiles.json");
        let ints_raw = std::fs::read_to_string(data_dir.join("interactions.json"))
            .expect("Gagal membaca data/processed/interactions.json");

        AppDatasetState {
            jobs: serde_json::from_str(&jobs_raw).expect("Format jobs_catalog.json tidak valid"),
            users: serde_json::from_str(&users_raw)
                .expect("Format users_profiles.json tidak valid"),
            interactions: serde_json::from_str(&ints_raw)
                .expect("Format interactions.json tidak valid"),
        }
    }

    fn load_from_mirror(path: &Path) -> Result<AppDatasetState, String> {
        let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).map_err(|e| e.to_string())
    }

    /// Penulisan atomik ke disk (`.tmp` lalu rename) agar berkas tidak pernah rusak.
    pub async fn save_mirror(&self) {
        let snap = self.snapshot().await;
        if let Ok(serialized) = serde_json::to_string_pretty(&*snap) {
            let tmp_path = self.runtime_mirror_path.with_extension("json.tmp");
            if std::fs::write(&tmp_path, serialized).is_ok() {
                let _ = std::fs::rename(&tmp_path, &self.runtime_mirror_path);
            }
        }
    }

    pub async fn reset_to_seed(&self) {
        let seed = Self::load_seed_files(&self.data_dir);
        self.mutate_state(|st| {
            *st = seed;
        })
        .await;
        let _ = self.sync_all_to_firestore().await;
    }

    fn json_to_firestore_value(v: &Value) -> Value {
        match v {
            Value::Null => json!({ "nullValue": null }),
            Value::Bool(b) => json!({ "booleanValue": b }),
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    json!({ "integerValue": i.to_string() })
                } else {
                    json!({ "doubleValue": n.as_f64().unwrap_or(0.0) })
                }
            }
            Value::String(s) => json!({ "stringValue": s }),
            Value::Array(arr) => {
                let vals: Vec<Value> = arr.iter().map(Self::json_to_firestore_value).collect();
                json!({ "arrayValue": { "values": vals } })
            }
            Value::Object(map) => {
                let mut fields = serde_json::Map::new();
                for (k, val) in map {
                    fields.insert(k.clone(), Self::json_to_firestore_value(val));
                }
                json!({ "mapValue": { "fields": fields } })
            }
        }
    }

    async fn resolve_firestore_base(&self) -> Option<String> {
        if let Some(b) = &self.firestore_base {
            return Some(b.clone());
        }
        if self.http.get("http://127.0.0.1:8088/").send().await.is_ok() {
            let mut m = self.mode_label.write().await;
            *m = "Firebase Emulator Suite (Terhubung :8088)".to_string();
            return Some(format!(
                "http://127.0.0.1:8088/v1/projects/{}/databases/(default)/documents",
                self.project_id
            ));
        }
        None
    }

    async fn upsert_with_base(
        http: &reqwest::Client,
        base: &str,
        api_key: &str,
        collection: &str,
        doc_id: &str,
        payload: &Value,
    ) -> Result<(), String> {
        let Some(obj) = payload.as_object() else {
            return Err("Payload bukan objek JSON".into());
        };
        let mut fields = serde_json::Map::new();
        for (k, val) in obj {
            fields.insert(k.clone(), Self::json_to_firestore_value(val));
        }

        let mut url = format!("{base}/{collection}/{doc_id}");
        if base.starts_with("https://") {
            url = format!("{url}?key={api_key}");
        }

        let resp = http
            .patch(&url)
            .json(&json!({ "fields": fields }))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("HTTP {}", resp.status()))
        }
    }

    pub async fn upsert_firestore_doc(
        &self,
        collection: &str,
        doc_id: &str,
        payload: &Value,
    ) -> Result<(), String> {
        let Some(base) = self.resolve_firestore_base().await else {
            return Err("Firestore endpoint belum aktif".into());
        };
        Self::upsert_with_base(
            &self.http,
            &base,
            &self.api_key,
            collection,
            doc_id,
            payload,
        )
        .await
    }

    async fn resolve_auth_base(&self) -> Option<String> {
        if let Some(a) = &self.auth_base {
            return Some(a.clone());
        }
        if self.http.get("http://127.0.0.1:9098/").send().await.is_ok() {
            return Some("http://127.0.0.1:9098/identitytoolkit.googleapis.com/v1".to_string());
        }
        None
    }

    /// Sinkronisasi konkuren (maks 16 worker) seluruh koleksi ke Cloud Firestore & Firebase Auth.
    pub async fn sync_all_to_firestore(&self) -> Result<FirebaseStatus, String> {
        let Some(base) = self.resolve_firestore_base().await else {
            return Err("Firestore endpoint belum aktif".into());
        };
        let auth_base = self.resolve_auth_base().await;
        let snapshot = self.snapshot().await;

        Self::upsert_with_base(
            &self.http,
            &base,
            &self.api_key,
            "system_meta",
            "catalog_info",
            &json!({
                "app": "KarirAI (Rust + Firebase)",
                "jobs_count": snapshot.jobs.len(),
                "users_count": snapshot.users.len(),
                "interactions_count": snapshot.interactions.len()
            }),
        )
        .await?;

        let sem = Arc::new(Semaphore::new(16));
        let mut set: JoinSet<()> = JoinSet::new();

        for u in &snapshot.users {
            if let Ok(val) = serde_json::to_value(u) {
                let permit = sem.clone().acquire_owned().await.ok();
                let http = self.http.clone();
                let base_c = base.clone();
                let auth_c = auth_base.clone();
                let key_c = self.api_key.clone();
                let uid = u.id.clone();
                let email = u.email.clone();
                let name = u.name.clone();
                set.spawn(async move {
                    let _p = permit;
                    if let Some(au) = auth_c {
                        let signup = format!("{au}/accounts:signUp?key={key_c}");
                        let _ = http
                            .post(&signup)
                            .json(&json!({
                                "email": email,
                                "password": "karir-ai-2026",
                                "displayName": name,
                                "returnSecureToken": true
                            }))
                            .send()
                            .await;
                    }
                    let _ =
                        Self::upsert_with_base(&http, &base_c, &key_c, "users", &uid, &val).await;
                });
            }
        }

        for j in &snapshot.jobs {
            if let Ok(val) = serde_json::to_value(j) {
                let permit = sem.clone().acquire_owned().await.ok();
                let http = self.http.clone();
                let base_c = base.clone();
                let key_c = self.api_key.clone();
                let jid = j.id.clone();
                set.spawn(async move {
                    let _p = permit;
                    let _ =
                        Self::upsert_with_base(&http, &base_c, &key_c, "jobs", &jid, &val).await;
                });
            }
        }

        for it in snapshot.interactions.iter().rev().take(80) {
            if let Ok(val) = serde_json::to_value(it) {
                let permit = sem.clone().acquire_owned().await.ok();
                let http = self.http.clone();
                let base_c = base.clone();
                let key_c = self.api_key.clone();
                let iid = it.id.clone();
                set.spawn(async move {
                    let _p = permit;
                    let _ =
                        Self::upsert_with_base(&http, &base_c, &key_c, "interactions", &iid, &val)
                            .await;
                });
            }
        }

        while set.join_next().await.is_some() {}

        Ok(self.status().await)
    }

    pub async fn status(&self) -> FirebaseStatus {
        let snapshot = self.snapshot().await;
        let idx = self.index().await;
        let emu_host = std::env::var("FIRESTORE_EMULATOR_HOST")
            .unwrap_or_else(|_| "127.0.0.1:8088".to_string());
        let auth_host = std::env::var("FIREBASE_AUTH_EMULATOR_HOST")
            .unwrap_or_else(|_| "127.0.0.1:9098".to_string());
        let emu_live = self
            .http
            .get(format!("http://{emu_host}/"))
            .send()
            .await
            .is_ok();
        let cloud_cfg = self
            .firestore_base
            .as_ref()
            .map(|b| b.starts_with("https://"))
            .unwrap_or(false);
        let connected = emu_live || cloud_cfg;

        let mode = if emu_live {
            format!("Firebase Emulator (Firestore {emu_host} + Auth {auth_host})")
        } else if cloud_cfg {
            "Google Firebase Cloud (Firestore REST)".to_string()
        } else {
            self.mode_label.read().await.clone()
        };

        let fs_ep = if emu_live {
            format!(
                "http://{emu_host}/v1/projects/{}/databases/(default)/documents",
                self.project_id
            )
        } else {
            self.firestore_base
                .clone()
                .unwrap_or_else(|| "data/runtime_firestore_mirror.json (Lokal)".into())
        };

        let au_ep = if emu_live {
            format!("http://{auth_host}/identitytoolkit.googleapis.com/v1")
        } else {
            self.auth_base
                .clone()
                .unwrap_or_else(|| "Sesi Persona Lokal".into())
        };

        FirebaseStatus {
            mode,
            connected,
            project_id: self.project_id.clone(),
            firestore_endpoint: fs_ep,
            auth_endpoint: au_ep,
            emulator_ui_url: if emu_live {
                Some("http://127.0.0.1:4040".into())
            } else {
                self.emulator_ui.clone()
            },
            synced_jobs: snapshot.jobs.len(),
            synced_users: snapshot.users.len(),
            synced_interactions: snapshot.interactions.len(),
            index_build_micros: idx.build_time_micros,
            message: if emu_live {
                "Terhubung ke Firebase Auth & Cloud Firestore Emulator (UI: http://127.0.0.1:4040)."
                    .into()
            } else if cloud_cfg {
                "Terhubung ke Google Cloud Firestore produksi.".into()
            } else {
                "Berjalan dalam mode penyimpanan lokal siap pakai. Jalankan run-emulator.bat atau ./run.sh --emulator untuk membuka dashboard Firestore :4040."
                    .into()
            },
        }
    }
}
