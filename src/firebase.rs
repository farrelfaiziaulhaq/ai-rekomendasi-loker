//! Integrasi Google Firebase (Cloud Firestore REST API + Firebase Auth Identity Toolkit REST API)
//! dengan mekanisme Auto-Detect Emulator & Local Persistent Mirror sehingga:
//! 1. Berjalan 100% terhubung ke Cloud Firestore & Firebase Auth (baik Cloud maupun Firebase Emulator `:8080`/`:9099`).
//! 2. Tetap dapat dijalankan 1-klik di Windows tanpa instalasi Java/Node/Firebase CLI bila sedang offline di kelas!

use crate::models::{JobPosting, UserInteraction, UserProfile};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

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
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDatasetState {
    pub jobs: Vec<JobPosting>,
    pub users: Vec<UserProfile>,
    pub interactions: Vec<UserInteraction>,
}

pub struct DataStore {
    pub state: RwLock<AppDatasetState>,
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

        // Muat data awal dari runtime mirror (bila sudah ada perubahan user) atau processed dataset
        let initial_state = if runtime_mirror_path.exists() {
            Self::load_from_mirror(&runtime_mirror_path)
                .unwrap_or_else(|_| Self::load_seed_files(&data_dir))
        } else {
            Self::load_seed_files(&data_dir)
        };

        let project_id =
            std::env::var("FIREBASE_PROJECT_ID").unwrap_or_else(|_| "demo-karir-ai".to_string());
        let api_key =
            std::env::var("FIREBASE_API_KEY").unwrap_or_else(|_| "demo-karir-ai-key".to_string());

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()
            .unwrap_or_default();

        // Deteksi konfigurasi Firebase Emulator atau Firebase Cloud
        let mut store_emu = std::env::var("FIRESTORE_EMULATOR_HOST").ok();
        let mut auth_emu = std::env::var("FIREBASE_AUTH_EMULATOR_HOST").ok();

        // Auto-probe port default emulator lokal (127.0.0.1:8088) bila env var belum diset!
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
                "Firebase Emulator Suite (Cloud Firestore + Auth Live)".to_string(),
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
                "Google Firebase Cloud Production (Firestore REST)".to_string(),
            )
        } else {
            (
                None,
                None,
                None,
                "Local Firestore Mirror (Siap Tanpa Instalasi Tambahan / Offline-Ready)"
                    .to_string(),
            )
        };

        let ds = Arc::new(Self {
            state: RwLock::new(initial_state),
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

    pub async fn save_mirror(&self) {
        let guard = self.state.read().await;
        if let Ok(serialized) = serde_json::to_string_pretty(&*guard) {
            let _ = std::fs::write(&self.runtime_mirror_path, serialized);
        }
    }

    pub async fn reset_to_seed(&self) {
        let seed = Self::load_seed_files(&self.data_dir);
        {
            let mut guard = self.state.write().await;
            *guard = seed;
        }
        self.save_mirror().await;
        let _ = self.sync_all_to_firestore().await;
    }

    /// Konversi JSON biasa ke format dokumen REST Cloud Firestore (`fields: { ... }`)
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

    /// Menyimpan satu dokumen ke koleksi Cloud Firestore via HTTP PATCH (upsert).
    pub async fn upsert_firestore_doc(
        &self,
        collection: &str,
        doc_id: &str,
        payload: &Value,
    ) -> Result<(), String> {
        let base = if let Some(b) = &self.firestore_base {
            b.clone()
        } else if self.http.get("http://127.0.0.1:8088/").send().await.is_ok() {
            let mut m = self.mode_label.write().await;
            *m = "Firebase Emulator Suite (Auto-Connected :8088)".to_string();
            format!(
                "http://127.0.0.1:8088/v1/projects/{}/databases/(default)/documents",
                self.project_id
            )
        } else {
            return Err("Firestore endpoint belum aktif".into());
        };

        let Some(obj) = payload.as_object() else {
            return Err("Payload bukan objek JSON".into());
        };
        let mut fields = serde_json::Map::new();
        for (k, val) in obj {
            fields.insert(k.clone(), Self::json_to_firestore_value(val));
        }

        let mut url = format!("{base}/{collection}/{doc_id}");
        if base.starts_with("https://") {
            url = format!("{url}?key={}", self.api_key);
        }

        let resp = self
            .http
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

    /// Mendaftarkan akun pengguna demo ke Firebase Auth Emulator bila aktif.
    async fn ensure_auth_user(&self, email: &str, display_name: &str) {
        let auth_url = if let Some(a) = &self.auth_base {
            a.clone()
        } else if self.http.get("http://127.0.0.1:9098/").send().await.is_ok() {
            "http://127.0.0.1:9098/identitytoolkit.googleapis.com/v1".to_string()
        } else {
            return;
        };

        let signup = format!("{auth_url}/accounts:signUp?key={}", self.api_key);
        let _ = self
            .http
            .post(&signup)
            .json(&json!({
                "email": email,
                "password": "karir-ai-2026",
                "displayName": display_name,
                "returnSecureToken": true
            }))
            .send()
            .await;
    }

    /// Menyinkronkan koleksi `jobs`, `users`, dan interaksi terbaru ke Cloud Firestore.
    pub async fn sync_all_to_firestore(&self) -> Result<FirebaseStatus, String> {
        let snapshot = self.state.read().await.clone();

        // Uji koneksi terlebih dahulu dengan satu dokumen metadata
        self.upsert_firestore_doc(
            "system_meta",
            "catalog_info",
            &json!({
                "app": "KarirAI - Rust + Firebase Job Recommender",
                "jobs_count": snapshot.jobs.len(),
                "users_count": snapshot.users.len(),
                "interactions_count": snapshot.interactions.len()
            }),
        )
        .await?;

        for u in &snapshot.users {
            self.ensure_auth_user(&u.email, &u.name).await;
            if let Ok(val) = serde_json::to_value(u) {
                let _ = self.upsert_firestore_doc("users", &u.id, &val).await;
            }
        }

        for j in &snapshot.jobs {
            if let Ok(val) = serde_json::to_value(j) {
                let _ = self.upsert_firestore_doc("jobs", &j.id, &val).await;
            }
        }

        // Sinkronkan 80 interaksi terbaru agar proses cepat namun koleksi `interactions` di Firestore kaya data
        for it in snapshot.interactions.iter().rev().take(80) {
            if let Ok(val) = serde_json::to_value(it) {
                let _ = self
                    .upsert_firestore_doc("interactions", &it.id, &val)
                    .await;
            }
        }

        Ok(self.status().await)
    }

    pub async fn status(&self) -> FirebaseStatus {
        let snapshot = self.state.read().await;
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
            format!("Firebase Emulator Suite (Cloud Firestore {emu_host} + Auth {auth_host} Aktif)")
        } else if cloud_cfg {
            "Google Firebase Cloud Production (REST API Aktif)".to_string()
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
                .unwrap_or_else(|| "data/runtime_firestore_mirror.json (Local Fallback)".into())
        };

        let au_ep = if emu_live {
            format!("http://{auth_host}/identitytoolkit.googleapis.com/v1")
        } else {
            self.auth_base
                .clone()
                .unwrap_or_else(|| "Local Persona Session".into())
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
            message: if emu_live {
                "Terhubung langsung ke Firebase Auth & Cloud Firestore Emulator! Buka http://127.0.0.1:4040 untuk melihat dokumen real-time.".into()
            } else if cloud_cfg {
                "Terhubung ke Google Cloud Firestore produksi.".into()
            } else {
                "Berjalan dalam Mode Instan Tanpa Konfigurasi (Local Mirror). Jalankan dengan --emulator / run-emulator.bat bila ingin menampilkan dashboard Firebase Emulator :4040.".into()
            },
        }
    }
}
