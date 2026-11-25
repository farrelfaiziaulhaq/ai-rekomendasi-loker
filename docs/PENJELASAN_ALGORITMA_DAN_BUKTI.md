# Penjelasan Lengkap Metode & Algoritma AI: Logika, Cara Kerja, Lokasi Kode, dan Bukti Nyata

Dokumen ini disusun khusus agar seluruh anggota kelompok dapat **memahami dan menjelaskan ulang setiap metode Artificial Intelligence (AI)** pada proyek **KarirAI** dengan bahasa yang runtut, logis, dan disertai bukti kode maupun hitungan matematis.

---

## 1. Peta Cepat: Algoritma Apa, Logikanya di Mana, Buktinya di Mana?

| Tahap Pipeline AI | Metode / Algoritma | Logika Singkat | Lokasi File & Fungsi Rust | Bukti di Tampilan Web / CLI | Bukti Unit Test |
|---|---|---|---|---|---|
| **Tahap 1A: Vektorisasi Teks** | **Tokenisasi + Smoothed TF-IDF + Cosine Similarity** | Mengubah kata-kata pada judul, skill, dan deskripsi lowongan menjadi vektor angka berbobot, lalu mengukur sudut kemiripan dengan profil minat user | [`src/ai/tfidf.rs`](../src/ai/tfidf.rs) (`TfidfCorpus::fit`, `vectorize`, `cosine_similarity_detailed`) | Tombol **🔍 Buka Bukti Perhitungan AI** pada tiap kartu lowongan (menampilkan tabel token `tfidf_user × tfidf_job`, `dot_product`, & `norma`) | `ai::tfidf::tests::tfidf_cosine_identik_bernilai_satu` |
| **Tahap 1B: Content-Based Filtering** | **Rocchio User Profile + Skill Coverage + TF-IDF** | Menggabungkan skill pilihan user + riwayat lowongan yang pernah disukai user untuk mencari lowongan baru yang isinya mirip | [`src/ai/content_based.rs`](../src/ai/content_based.rs) (`build_user_profile_vector`, `score_content_based`) | Skor `Content-Based` di tiap kartu + chip hijau `✓ skill` yang cocok | Diuji pada evaluasi ablasi `--eval` (`P@3 = 47.22%`) |
| **Tahap 2: Collaborative Filtering** | **User-Based Mean-Centered Pearson Correlation** | Mencari pengguna lain (*neighbors*) yang punya pola suka/rating mirip, lalu memprediksi skor lowongan yang belum pernah dicoba user aktif | [`src/ai/collaborative.rs`](../src/ai/collaborative.rs) (`CollaborativeMatrix::user_similarity`, `predict_for_job`) | **Tab 2 (Pola Pengguna)** & bagian Tahap 2 di modal **Bukti Perhitungan AI** | `ai::collaborative::tests::skenario_slide_06_dina_dan_bima_pola_mirip` |
| **Tahap 3: Multi-Criteria Scoring** | **Weighted Multi-Criteria Utility (Adomavicius & Kwon, 2015)** | Menilai lowongan pada 4 aspek terpisah (*Kejelasan, Tantangan, Gaji, Fleksibilitas*) dikalikan bobot prioritas user (`w1..w4`) | [`src/ai/multicriteria.rs`](../src/ai/multicriteria.rs) (`score_multi_criteria`, `verify_lecturer_slide_case`) | Slider bobot di kiri **Tab 1** + simulasi langsung di **Tab 3 (Simulasi Materi Dosen)** + CLI `--case-study` | `ai::multicriteria::tests::verifikasi_angka_persis_slide_12_13_16_17_dosen` |
| **Tahap 4: Hard Constraint Filter** | **Rule/Constraint-Based Pre-Selection Filter** | Mengeliminasi lowongan yang melanggar syarat mutlak user (gaji kurang, syarat pengalaman berlebih, mode kerja tidak sesuai, skor aspek di bawah batas) meskipun skor totalnya tinggi | [`src/ai/constraints.rs`](../src/ai/constraints.rs) (`check_hard_constraints`) | Kotak oranye **"Lowongan Berskor Tinggi yang Gugur Syarat Wajib"** di bagian bawah Tab 1 | `ai::constraints::tests::filter_syarat_wajib_menggugurkan_item_tidak_memenuhi_syarat` |
| **Tahap 5: Penggabungan & Cold-Start** | **Adaptive Weighted Hybrid Multi-Criteria (HMC-CF)** | Menggabungkan `S_CB`, `S_CF`, dan `S_MC` secara proporsional serta meredistribusi bobot otomatis bila user baru belum punya sinyal CF (*Cold-Start*) | [`src/ai/hybrid.rs`](../src/ai/hybrid.rs) (`recommend_jobs`) | Banner rumus aktif & persentase kecocokan utama (`%`) di Tab 1 | Diuji pada evaluasi ablasi `--eval` (`P@3 = 76.39%`, `NDCG@5 = 95.01%`) |
| **Tahap 6: Evaluasi Kuantitatif AI** | **Precision@K, Recall@K, F1@K, NDCG@K, CSR, Latency** | Mengukur seberapa akurat rekomendasi sistem dibanding *ground-truth test split* pada 5 variasi metode | [`src/ai/evaluation.rs`](../src/ai/evaluation.rs) (`run_full_evaluation`) | **Tab 4 (Evaluasi Precision@K & NDCG)** & perintah terminal `cargo run --release -- --eval` | Reproducible 100% setiap kali dijalankan |

---

## 2. Cara Kerja Step-by-Step & Rumus Matematis Tiap Algoritma

### 2.1 Tahap 1: Content-Based Filtering (TF-IDF + Cosine Similarity)
**Intuisi sederhana (Slide 05):** *"Jika Dina menyukai Python & Machine Learning, carikan lowongan kerja yang deskripsi dan keahliannya banyak memuat kata kunci tersebut."*

1. **Representasi Dokumen Lowongan (`src/ai/content_based.rs`):**
   Setiap lowongan $j$ digabung teksnya: `judul + kategori + 2×daftar_skill + deskripsi`, lalu dibersihkan dari kata sambung (*stopwords* Indonesia & Inggris) di `src/ai/tfidf.rs`.
2. **Perhitungan Bobot TF-IDF (`src/ai/tfidf.rs`):**
   - *Term Frequency* ($\text{TF}$): seberapa sering kata $t$ muncul pada dokumen $d$:
     $$\text{TF}(t, d) = \frac{\text{jumlah kemunculan } t \text{ di } d}{\text{total kata bersih di } d}$$
   - *Smoothed Inverse Document Frequency* ($\text{IDF}$): memberi bobot tinggi pada kata teknis spesifik (misal `rust`, `tf-idf`, `firestore`) dan bobot rendah pada kata umum:
     $$\text{IDF}(t) = \ln\left(\frac{1 + N}{1 + df(t)}\right) + 1$$
     dengan $N = 60$ jumlah lowongan dan $df(t)$ jumlah lowongan yang mengandung kata $t$.
   - Bobot akhir tiap kata: $w_{t,d} = \text{TF}(t, d) \times \text{IDF}(t)$.
3. **Pembentukan Vektor Profil User dengan Ekspansi Rocchio (`build_user_profile_vector`):**
   Profil user $\vec{u}$ tidak hanya berasal dari input skill manual, tetapi juga diperkaya secara otomatis dari rata-rata vektor lowongan yang pernah diberi rating tinggi ($\ge 3.8$) oleh user tersebut:
   $$\vec{u} = \vec{u}_{\text{eksplisit}} + \frac{0.55}{|I_u^+|} \sum_{i \in I_u^+} \frac{r_{u,i}}{5.0} \vec{j}_i$$
4. **Cosine Similarity & Skor Akhir Content-Based:**
   $$\text{Cosine}(\vec{u}, \vec{j}) = \frac{\sum_t u_t \cdot j_t}{\sqrt{\sum_t u_t^2} \cdot \sqrt{\sum_t j_t^2}}$$
   $$S_{CB}(u, j) = 0.55 \cdot \text{Cosine}(\vec{u}, \vec{j}) + 0.35 \cdot \frac{|K_u \cap K_j|}{|K_j|} + 0.10 \cdot \mathbb{I}(\text{Kategori Cocok})$$

**Contoh Bukti Nyata di Aplikasi (Dina → Lowongan `AI & Recommender Systems Engineer`):**
- `dot_product (u · j)` = `0.2601`, `||u||` = `0.7530`, `||j||` = `0.8282` → `Cosine` = `0.2601 / (0.7530 × 0.8282) = 0.4172`
- `SkillCoverage` = `4 / 7 skill cocok` (`python, machine learning, tf-idf, sql`) = `0.5714`
- `Bidang Cocok` = `Ya (1.0)`
- `S_CB` = `0.55(0.4172) + 0.35(0.5714) + 0.10(1.0) = 0.5294` *(terbukti persis sama sampai 4 desimal pada modal Inspector!)*.

---

### 2.2 Tahap 2: User-Based Collaborative Filtering (Pearson Neighborhood)
**Intuisi sederhana (Slide 06):** *"Dina dan Arif sama-sama menyukai 6 lowongan Python/Data yang serupa. Karena Arif memberi bintang 5.0 pada lowongan AI Engineer yang belum pernah dicoba Dina, maka lowongan tersebut direkomendasikan untuk Dina."*

1. **Kemiripan Antar-Pengguna (`user_similarity` di `src/ai/collaborative.rs`):**
   Menggunakan korelasi Pearson terpusat rata-rata (*mean-centered*) agar perbedaan gaya memberi bintang (user pelit bintang vs murah bintang) ternormalisasi secara adil:
   $$\text{Sim}(u, v) = \frac{\sum_{i \in I_{uv}} (r_{u,i} - \bar{r}_u)(r_{v,i} - \bar{r}_v)}{\sqrt{\sum_{i \in I_{uv}} (r_{u,i} - \bar{r}_u)^2} \sqrt{\sum_{i \in I_{uv}} (r_{v,i} - \bar{r}_v)^2}} \times \min\left(1, \frac{|I_{uv}|}{3}\right)$$
2. **Prediksi Rating dari Top-5 Tetangga Terdekat (`predict_for_job`):**
   $$\hat{r}_{CF}(u, j) = \bar{r}_u + \frac{\sum_{v \in N_5(u;j)} \text{Sim}(u, v) \cdot (r_{v,j} - \bar{r}_v)}{\sum_{v \in N_5(u;j)} |\text{Sim}(u, v)|}$$
   Lalu dinormalisasi ke skala $[0, 1]$: $S_{CF}(u, j) = (\hat{r}_{CF}(u, j) - 1) / 4$.

**Contoh Bukti Nyata di Aplikasi:**
- Tetangga paling mirip dengan Dina (`USR-01`) adalah **Arif Nugroho (`USR-04`)** dengan `Pearson Sim = 0.6523` (6 lowongan beririsan).
- Rata-rata rating Dina $\bar{r}_u = 3.74$; prediksi $\hat{r}_{CF} = 4.91 / 5.00$ → $S_{CF} = (4.91 - 1)/4 = 0.9768$.

---

### 2.3 Tahap 3: Multi-Criteria Recommender System (MCRS — Adomavicius & Kwon, 2015)
**Intuisi sederhana (Slide 09–13):** *"Satu nilai rata-rata bintang menipu. Dua pekerjaan sama-sama rata-rata 4.0, tetapi Pekerjaan A unggul di bimbingan mentor (cocok untuk Fresh Graduate), sedangkan Pekerjaan B unggul di gaji & tantangan sistem (cocok untuk Senior Engineer)."*

Setiap lowongan dinilai pada 4 aspek ($C_1$ Kejelasan Peran, $C_2$ Tantangan Teknis, $C_3$ Kompensasi Gaji, $C_4$ Fleksibilitas Kerja). Bobot input pengguna dinormalisasi sehingga $\sum_{k=1}^4 w_k = 1$:
$$R_{MC}(u, j) = w_1 C_{j,1} + w_2 C_{j,2} + w_3 C_{j,3} + w_4 C_{j,4}, \quad S_{MC}(u, j) = \frac{R_{MC}(u, j) - 1}{4}$$

**Contoh Hitung Manual vs Output Rust (Dina pada Lowongan `AI & Recommender Systems Engineer`):**
- Bobot Dina: $w = [0.45, 0.15, 0.15, 0.25]$
- Skor Kriteria Lowongan: $C = [4.8, 4.4, 3.6, 4.2]$
- Hitung manual: $(0.45 \times 4.8) + (0.15 \times 4.4) + (0.15 \times 3.6) + (0.25 \times 4.2) = 2.16 + 0.66 + 0.54 + 1.05 = \mathbf{4.41 / 5.00}$
- Normalisasi: $(4.41 - 1.0) / 4.0 = \mathbf{0.8525}$ *(sama persis dengan angka di UI KarirAI!)*.

---

### 2.4 Tahap 4 & 5: Penggabungan Hybrid Adaptif + Filter Syarat Wajib (Hard Constraints)
1. **Pemeriksaan Hard Constraints (`src/ai/constraints.rs` — Slide 14):**
   Memeriksa 5 syarat mutlak: `Gaji >= Gaji_Min`, `Syarat_Pengalaman <= Maks_Pengalaman_User`, `Mode_Kerja ∈ Diizinkan`, `C1 >= Min_C1`, dan `C2 >= Min_C2`. Jika gagal satu saja, lowongan dipisahkan ke daftar **Gugur Syarat Wajib** beserta alasan spesifiknya.
2. **Penggabungan Skor Akhir (`src/ai/hybrid.rs`):**
   - Normal: $S_{\text{Final}} = 0.40 \cdot S_{CB} + 0.25 \cdot S_{CF} + 0.35 \cdot S_{MC}$
   - Saat *Cold-Start* (belum ada tetangga CF yang menilai lowongan tersebut): bobot $\beta = 0.25$ dibagi secara proporsional ke $\alpha$ dan $\gamma$ menjadi $\alpha = 0.5333, \gamma = 0.4667$ sehingga lowongan baru yang bagus tidak dirugikan.
   - Bukti hitung contoh Dina di atas:
     $$S_{\text{Final}} = 0.40(0.5294) + 0.25(0.9768) + 0.35(0.8525) = 0.2118 + 0.2442 + 0.2984 = \mathbf{0.7543 \ (75.4\%)}$$

---

### 2.5 Tahap 6: Bukti Evaluasi Ilmiah (`Precision@K`, `NDCG@K`, & `CSR`)
Jalankan perintah berikut kapan saja di terminal Windows maupun Linux untuk melihat bukti kuantitatif langsung dari biner Rust:

```bash
cargo run --release -- --eval
```

Hasil terukur pada 12 persona, 60 lowongan riil, dan 336 interaksi (`252 train / 84 test`):
- **Content-Based murni**: `Precision@3 = 47.22%`, `NDCG@5 = 64.53%`, `Lolos Syarat Wajib = 43.3%`
- **Collaborative murni**: `Precision@3 = 33.33%`, `NDCG@5 = 38.87%`, `Lolos Syarat Wajib = 43.3%`
- **Hybrid Klasik (CB + CF)**: `Precision@3 = 63.89%`, `NDCG@5 = 70.65%`, `Lolos Syarat Wajib = 50.0%`
- **Metode Usulan (HMC-CF)**: **`Precision@3 = 76.39%`**, **`NDCG@5 = 95.01%`**, **`Lolos Syarat Wajib = 100.0%`**, dalam waktu rata-rata **`2.24 milidetik`**!
