# Panduan Tanya Jawab (Q&A) Dosen & Skenario Demo Presentasi Proyek AI

Dokumen ini merangkum **daftar pertanyaan kritis yang paling sering dilontarkan dosen pengampu mata kuliah Artificial Intelligence (AI)** saat presentasi/sidang tugas proyek, lengkap dengan **jawaban lugas, rumus kunci, dan lokasi bukti di aplikasi maupun kode Rust**.

---

## Bagian A: Skenario Demo Live 5 Menit di Depan Dosen (Urutan Paling Efektif)

1. **Menit 1 — Tunjukkan Masalah & Kontras Persona (Bukti Slide 01–03 & 11–13):**
   - Buka `http://127.0.0.1:3000`. Tunjukkan pengguna aktif **Dina Rahmawati (Fresh Graduate, bobot Kejelasan 45%)** → Peringkat #1 adalah *AI & Recommender Systems Engineer* (ramah pemula, mentorship tinggi).
   - Ubah dropdown persona ke **Bima Pratama (Systems Engineer, bobot Tantangan Teknis 45% & Gaji 30%)** → dalam `~2 milidetik`, urutan katalog berubah total menjadi *High-Frequency Systems & Rust Backend Engineer* (Rp 22 Jt/bln, tantangan 5.0/5.0).
2. **Menit 2 — Buka Bukti Perhitungan AI / White-Box Inspector (Bukti Ini Proyek AI Asli, Bukan CRUD Biasa!):**
   - Klik tombol **`🔍 Buka Bukti Perhitungan AI (Rumus & Vektor)`** pada kartu Peringkat #1.
   - Tunjukkan ke dosen: angka `dot_product`, `norma vektor`, tabel kata kunci **TF-IDF**, tabel tetangga **Pearson Collaborative Filtering**, jejak rumus **Multi-Criteria**, dan penggabungan **Hybrid**.
3. **Menit 3 — Buktikan Filter Syarat Wajib & Simulasi Materi PDF Dosen (Bukti Slide 09–17):**
   - Gulir ke bawah Tab 1: tunjukkan kotak **"Lowongan Berskor Tinggi yang Gugur Syarat Wajib"** (misalnya gaji kurang atau meminta pengalaman 5 tahun untuk Fresh Graduate).
   - Klik **Tab 3 (`3. Simulasi Materi PDF Dosen`)**: tunjukkan angka `4.4 vs 3.6` dan soal latihan `50%-50% + Syarat Minimal 4.0` yang sama persis dengan slide dosen, lalu geser slider-nya secara live!
4. **Menit 4 — Tunjukkan Bukti Evaluasi Ilmiah AI (Bukti Slide 15):**
   - Klik **Tab 4 (`4. Evaluasi Precision@K & NDCG`)**: tunjukkan peningkatan `Precision@3` dari `47.22%` (Content-Based) dan `33.33%` (Collaborative) menjadi **`76.39%`** dan `NDCG@5 = 95.01%` serta kepatuhan syarat wajib **`100%`** pada metode usulan **HMC-CF**.
5. **Menit 5 — Tunjukkan Integrasi Real-Time Firebase Firestore:**
   - Klik tombol `★ 5 Lamar` pada salah satu lowongan di Tab 1, lalu buka **Tab 5** atau **Firebase Emulator UI (`http://127.0.0.1:4040/firestore`)** untuk memperlihatkan koleksi `jobs`, `users`, `interactions`, dan `recommendation_logs` yang tersimpan secara real-time di Cloud Firestore!

---

## Bagian B: 20 Prediksi Pertanyaan Dosen & Jawaban Siap Pakai

### Kelompok 1: Pertanyaan Seputar Penerapan Materi PDF Perkuliahan

#### Q1: *"Di PDF perkuliahan contohnya memilih modul belajar dengan aspek Kejelasan dan Latihan. Bagaimana kalian menerapkannya pada lowongan kerja?"*
**Jawaban:**
> "Kami mengadaptasi struktur konseptual pada PDF secara langsung ke domain rekrutmen kerja IT. Dua aspek di slide (*Kejelasan* dan *Latihan*) kami translasikan dan perluas menjadi **4 kriteria utama lowongan kerja (skala 1–5)**:
> 1. **C1 Kejelasan Peran & Mentorship** (paralel langsung dengan *Kejelasan* untuk pencari kerja pemula/fresh graduate seperti Dina),
> 2. **C2 Tantangan Teknis & Kompleksitas Proyek** (paralel langsung dengan *Latihan/Uji Kemampuan* untuk engineer seperti Bima),
> 3. **C3 Daya Tarik Kompensasi Gaji**, dan
> 4. **C4 Fleksibilitas Mode Kerja (Remote/Hybrid/Onsite)**.
> Selain pada 60 katalog lowongan riil di Tab 1, kami juga membuat **Tab 3 khusus** yang memverifikasi angka identik Slide 09–17 (`4.4 vs 3.6` dan filter syarat minimal `4.0`) di kode `src/ai/multicriteria.rs`."

#### Q2: *"Kenapa tidak cukup memakai satu rating rata-rata bintang saja untuk merekomendasikan lowongan kerja?"*
**Jawaban:**
> "Sesuai teori **Adomavicius & Kwon (2015)** pada Slide 09–10, rata-rata tunggal menyembunyikan distribusi nilai antar-aspek (*information loss*). Contohnya, Lowongan A (Kejelasan 5, Tantangan 3) dan Lowongan B (Kejelasan 3, Tantangan 5) sama-sama memiliki rata-rata **4.0**. Sistem *single-rating* menganggap keduanya setara, padahal bagi *Fresh Graduate* yang butuh bimbingan mentor (bobot Kejelasan 70%), Lowongan A bernilai **4.4** sedangkan B hanya **3.6**. Sebaliknya bagi *Senior Engineer* yang mengejar tantangan teknis (70%), B bernilai **4.4** dan A **3.6**."

#### Q3: *"Di Slide 14 ada konsep 'Filter Sebelum Memilih'. Bedanya apa dengan pembobotan biasa?"*
**Jawaban:**
> "Pembobotan multikriteria bersifat **kompensatoris** (*soft preference*): nilai rendah pada satu aspek masih bisa ditutupi oleh nilai sangat tinggi pada aspek lain. Sebaliknya, **Filter Sebelum Memilih (*Hard Constraints*)** bersifat **non-kompensatoris / mutlak** (`src/ai/constraints.rs`). Misalnya seorang *Fresh Graduate* (0 tahun pengalaman) mencari lowongan Remote dengan gaji minimal Rp 8 Juta. Jika ada lowongan *Senior Principal* Onsite yang kata kuncinya sangat cocok sehingga skor kemiripannya tinggi, sistem **wajib menggugurkannya** karena melanggar syarat mutlak pengalaman dan mode kerja. Buktinya kami tampilkan secara transparan di panel bawah Tab 1 dan metrik *Constraint Satisfaction Rate* yang naik dari `43.3%` menjadi `100%`."

---

### Kelompok 2: Pertanyaan Mendalam Seputar Algoritma & Logika AI

#### Q4: *"Apakah kalian memakai library AI/ML instan (black-box), atau menulis algoritmanya sendiri? Tunjukkan logikanya di kode!"*
**Jawaban:**
> "Seluruh algoritma AI kami tulis **100% *from scratch* (dari nol) dalam bahasa Rust** tanpa library *black-box*, sehingga setiap baris rumusnya bisa diaudit langsung:
> - **Tokenisasi, TF-IDF, & Cosine Similarity**: `src/ai/tfidf.rs` (baris 22–126)
> - **Content-Based + Ekspansi Profil Rocchio**: `src/ai/content_based.rs` (baris 25–107)
> - **User-Based Collaborative Filtering (Pearson)**: `src/ai/collaborative.rs` (baris 55–160)
> - **Multi-Criteria Scoring**: `src/ai/multicriteria.rs` (baris 15–52)
> - **Hard Constraint Filtering**: `src/ai/constraints.rs` (baris 8–93)
> - **Hybrid Orchestrator & Cold-Start Handling**: `src/ai/hybrid.rs` (baris 40–175)
> - **Evaluasi Precision@K & NDCG@K**: `src/ai/evaluation.rs`."

#### Q5: *"Bagaimana cara kerja TF-IDF dan Cosine Similarity pada Content-Based Filtering kalian?"*
**Jawaban:**
> "Pertama, judul, daftar skill, dan deskripsi tiap lowongan dibersihkan dari *stopwords* lalu dihitung frekuensi katanya ($\text{TF}$). Kedua, dikalikan dengan $\text{IDF}(t) = \ln(\frac{1+N}{1+df(t)}) + 1$ agar kata keahlian spesifik seperti `rust`, `tokio`, atau `tf-idf` memiliki bobot jauh lebih besar daripada kata umum seperti `sistem` atau `pengembangan`. Ketiga, profil pencari kerja juga diubah menjadi vektor TF-IDF (gabungan skill pilihan + rata-rata vektor lowongan yang pernah disukai user). Terakhir, kemiripan keduanya diukur memakai kosinus sudut antar-vektor $\frac{\vec{u} \cdot \vec{j}}{\|\vec{u}\|\|\vec{j}\|} \in [0, 1]$, lalu dipadukan dengan rasio cakupan skill eksplisit."

#### Q6: *"Mengapa pada Collaborative Filtering kalian memakai Mean-Centered Pearson Correlation, bukan Cosine biasa pada rating mentah?"*
**Jawaban:**
> "Karena setiap pengguna memiliki bias skala penilaian (*rating scale bias*) yang berbeda: ada pengguna yang murah hati selalu memberi bintang 4–5, dan ada pengguna kritis yang memberi bintang 3 untuk item yang ia sukai. Dengan mengurangkan rata-rata rating masing-masing pengguna ($r_{u,i} - \bar{r}_u$) pada rumus Pearson di `src/ai/collaborative.rs`, sistem mengukur kesamaan **pola naik-turunnya selera**, bukan sekadar angka mentahnya. Kami juga menangani kasus khusus ketika seluruh rating pada irisan bernilai identik konstan (seperti contoh Dina & Bima sama-sama 'Suka' di Slide 06) agar tetap terdeteksi memiliki kemiripan tinggi."

#### Q7: *"Bagaimana sistem kalian menangani masalah Cold-Start (pengguna baru belum punya riwayat rating, atau lowongan baru belum pernah dinilai siapa pun)?"*
**Jawaban:**
> "Inilah alasan utama kami merancang arsitektur **Adaptive Hybrid Multi-Criteria** di `src/ai/hybrid.rs` (baris 95–104). Ketika suatu lowongan belum memiliki sinyal tetangga pada Collaborative Filtering (`has_cf_signal == false`), sistem tidak memberi nilai nol yang merusak skor akhir, melainkan **meredistribusikan bobot Collaborative ($\beta = 0.25$) secara proporsional** ke komponen Content-Based ($\alpha$ naik dari `0.40` ke `0.5333`) dan Multi-Criteria ($\gamma$ naik dari `0.35` ke `0.4667`). Dengan demikian, pengguna baru tetap langsung memperoleh rekomendasi akurat sejak detik pertama berdasarkan skill dan prioritas kriterianya."

#### Q8: *"Bagaimana kalian membuktikan bahwa metode Hybrid Multi-Criteria + Hard Filter benar-benar lebih baik dibanding Content-Based atau Collaborative saja?"*
**Jawaban:**
> "Kami melakukan pengujian ablasi (*ablation study*) menggunakan pembagian data interaksi `train` (252 interaksi) dan `test hold-out` (84 interaksi) pada `src/ai/evaluation.rs`:
> 1. **Collaborative saja** hanya mencapai `Precision@3 = 33.33%` karena matriks interaksi jarang (*data sparsity*).
> 2. **Content-Based saja** mencapai `Precision@3 = 47.22%`, namun mengabaikan preferensi kualitas aspek dan melanggar syarat wajib pada `56.7%` rekomendasi.
> 3. **Hybrid Klasik (CB + CF)** naik ke `Precision@3 = 63.89%` (`NDCG@5 = 70.65%`).
> 4. **Metode Usulan (HMC-CF)** mencapai **`Precision@3 = 76.39%`**, **`NDCG@5 = 95.01%`**, dan **`100%` kepatuhan syarat wajib**. Artinya lebih dari 3 dari 4 rekomendasi teratas benar-benar relevan dan tidak ada satu pun yang melanggar batasan pengguna."

#### Q9: *"Apa arti metrik Precision@3 dan NDCG@5 pada tabel evaluasi kalian?"*
**Jawaban:**
> - **Precision@3** (persis konsep evaluasi pada **Slide 15 PDF Dosen**: *'2 dari 3 rekomendasi cocok'* = `66.7%`) mengukur berapa persen dari 3 rekomendasi teratas yang benar-benar relevan bagi pengguna. Nilai `76.39%` berarti rata-rata hampir seluruh item di 3 posisi teratas cocok.
> - **NDCG@5 (*Normalized Discounted Cumulative Gain*)** mengukur **kualitas urutan peringkat**: rekomendasi yang sangat cocok mendapat bobot penalti logaritmik bila ditaruh di urutan bawah (#5) dibanding urutan puncak (#1). Nilai `NDCG@5 = 95.01%` membuktikan item terbaik konsisten ditempatkan di peringkat paling atas."

---

### Kelompok 3: Pertanyaan Seputar Pemilihan Teknologi (Rust + Firebase + Dataset + Windows)

#### Q10: *"Kenapa membangun sistem AI ini menggunakan Rust dan Firebase, bukan Python + MySQL seperti umumnya?"*
**Jawaban:**
> "Ada tiga alasan teknis dan rekayasa yang kuat:
> 1. **Kecepatan Inferensi Real-Time (Rust)**: Saat pengguna menggeser slider bobot kriteria di UI, server harus menghitung ulang vektor TF-IDF, matriks Pearson, skor multikriteria, dan filter syarat wajib secara instan. Dengan kompilasi native Rust tanpa *garbage collector* (didukung studi Xu et al., J14), seluruh pipeline rekomendasi selesai hanya dalam **2.24 milidetik (`2.243 µs`)**.
> 2. **Kemudahan Distribusi Lintas OS (Windows-Friendly)**: Hasil kompilasi Rust adalah **satu file `.exe` mandiri (`karir-ai.exe`)** berukuran kecil di GitHub Releases. Teman kelompok pengguna Windows tinggal klik dua kali `start.bat` tanpa pusing bentrok versi Python, Anaconda, pip, atau library C++.
> 3. **Persistensi Dokumen Fleksibel & Real-Time (Google Firebase Firestore)**: Profil pengguna dengan struktur bersarang (`weights`, `constraints`, `skills`) dan log interaksi real-time sangat alami disimpan sebagai dokumen JSON di Cloud Firestore REST API (didukung studi optimasi Firestore Fajri et al., J15)."

#### Q11: *"Dari mana asal dataset yang kalian gunakan? Apakah datanya valid?"*
**Jawaban:**
> "Kami menggunakan dataset publik nyata yang terverifikasi dan kami sertakan skrip pengunduhnya (`scripts/download_resources.py`) serta berkas mentahnya di `data/raw/`:
> 1. **Tech Job Postings & Salary Dataset** dari HuggingFace (`zalizedata/tech-job-postings-salary-dataset`, 500 baris lowongan ATS Ashby/Greenhouse/Lever).
> 2. **LinkedIn Job Postings Dataset** (`xanderios/linkedin-job-postings`, lisensi MIT, subset terkurasi 291 lowongan IT/Software/Data/Security).
> Data tersebut dibersihkan dan distandarisasi melalui `scripts/preprocess_dataset.py` menjadi 60 katalog lowongan terstruktur lengkap dengan ketertelusuran `source_dataset` dan `source_job_id` pada setiap baris."

#### Q12: *"Apa keterbatasan sistem kalian saat ini dan saran pengembangan ke depannya?"*
**Jawaban:**
> "Saat ini representasi teks Content-Based menggunakan **TF-IDF leksikal**, sehingga sinonim kata yang berbeda bentuk (misalnya *'pengembang piranti lunak'* vs *'software developer'* bila tidak ada irisan kata kunci skill) belum tertangkap secara semantik mendalam seperti pada *transformer embeddings* (BERT/LLM). Namun, penggabungan kamus skill terstandarisasi dan sinyal Collaborative Filtering di metode Hybrid kami sudah sangat efektif memitigasi hal tersebut dengan latensi jauh lebih ringan (`2.2 ms` tanpa GPU). Ke depannya, sistem dapat diperkaya dengan ekstraksi CV PDF otomatis dan pencocokan dua arah (*reciprocal recommendation* antara pelamar dan HRD)."
