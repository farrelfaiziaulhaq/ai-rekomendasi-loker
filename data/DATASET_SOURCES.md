# Dokumentasi Sumber Dataset & Metodologi Prapemrosesan Data

Seluruh berkas dataset mentah (*raw*) maupun hasil prapemrosesan (*processed*) telah diunduh dan disusun secara rapi di dalam direktori `data/` agar dapat langsung diaudit, dibuka di Microsoft Excel (`.csv`), maupun direproduksi ulang kapan saja melalui skrip `scripts/download_resources.py` dan `scripts/preprocess_dataset.py`.

---

## 1. Daftar Sumber Dataset Publik & Tautan Resmi

| Kode | Nama Dataset | Repositori & Tautan Resmi | Lisensi | Berkas Lokal (`data/raw/`) | Peran dalam Sistem |
|---|---|---|---|---|---|
| **DS-01** | **Tech Job Postings & Salary Dataset (Multi-ATS)** | HuggingFace Datasets: https://huggingface.co/datasets/zalizedata/tech-job-postings-salary-dataset <br/>Direct CSV: https://huggingface.co/datasets/zalizedata/tech-job-postings-salary-dataset/resolve/main/sample.csv | CC-BY-NC 4.0 | [`data/raw/tech_job_postings_salary_sample.csv`](raw/tech_job_postings_salary_sample.csv) (500 baris, 184 KB) | Sumber utama lowongan perusahaan teknologi global (Ashby, Greenhouse, Lever) lengkap dengan `skills`, `is_remote`, `department`, dan `salary_min/max`. |
| **DS-02** | **LinkedIn Job Postings Dataset** | HuggingFace Datasets: https://huggingface.co/datasets/xanderios/linkedin-job-postings <br/>Mirror Kaggle: https://www.kaggle.com/datasets/arshkon/linkedin-job-postings | MIT License | [`data/raw/linkedin_tech_jobs_subset.csv`](raw/linkedin_tech_jobs_subset.csv) (291 baris terkurasi IT/Tech, 528 KB) | Sumber deskripsi lowongan mendalam (`description`), level senioritas (`formatted_experience_level`), mode kerja (`remote_allowed`), dan rentang kompensasi. |
| **DS-03** | **Indonesian Tech & Fresh-Graduate Benchmark Subset** | Dikurasi di dalam [`scripts/preprocess_dataset.py`](../scripts/preprocess_dataset.py) | MIT License | Tergabung pada [`data/processed/jobs_catalog.json`](processed/jobs_catalog.json) (`JOB-053` s.d. `JOB-060`) | Menambahkan posisi ekosistem IT Indonesia (Jakarta, Bandung, Malang, Yogyakarta, Surabaya) seperti *Rust + Firebase Engineer*, *Junior Python Mentorship*, dan *AI/NLP Engineer* untuk pengujian dwibahasa & studi kasus kontras multikriteria. |

---

## 2. Struktur Berkas Terproses (`data/processed/`)

Skrip `python3 scripts/preprocess_dataset.py` menggabungkan, membersihkan teks non-ASCII, mengekstrak kamus keahlian teknis (*skill taxonomy*), dan menghasilkan 5 berkas siap pakai:

1. **[`data/processed/jobs_catalog.json`](processed/jobs_catalog.json)** & **[`jobs_catalog.csv`](processed/jobs_catalog.csv)** — **60 lowongan kerja bidang IT/Tech** dengan atribut:
   - `id`, `source_dataset`, `source_job_id` (ketertelusuran penuh ke dataset asli)
   - `title`, `company`, `category`, `level`, `min_experience_years`
   - `work_mode` (`Remote`, `Hybrid`, `Onsite`), `location`
   - `salary_idr_millions` (ekuivalen gaji bulanan Juta Rupiah) & `salary_usd_annual_ref`
   - `skills` (array keahlian teknis terstandarisasi untuk pencocokan *Content-Based*)
   - `description` (teks deskripsi pekerjaan untuk vektorisasi **TF-IDF**)
   - `criteria`: skor 4 aspek multikriteria skala `1.0 – 5.0`:
     - `c1_role_clarity`: Kejelasan deskripsi peran, struktur onboarding, & mentorship (*paralel aspek Kejelasan pada Slide 09–13 PDF Dosen*)
     - `c2_technical_challenge`: Intensitas tantangan teknis & kedalaman arsitektur (*paralel aspek Latihan pada Slide 09–13 PDF Dosen*)
     - `c3_compensation`: Daya tarik kompensasi gaji terhadap level jabatan
     - `c4_flexibility`: Fleksibilitas mode kerja (`Remote` > `Hybrid` > `Onsite`)
2. **[`data/processed/users_profiles.json`](processed/users_profiles.json)** — **12 persona pencari kerja** (dimulai dari `USR-01 Dina Rahmawati` dan `USR-02 Bima Pratama` selaras materi perkuliahan, dilengkapi preferensi skill, bobot 4 kriteria, serta *Hard Constraints* masing-masing).
3. **[`data/processed/interactions.json`](processed/interactions.json)** & **[`interactions.csv`](processed/interactions.csv)** — **336 baris interaksi historis Pengguna × Lowongan** (`252 train` + `84 test hold-out`), memuat rating eksplisit (`1.0 – 5.0`) dan sinyal implisit (`viewed`, `bookmarked`, `applied`) untuk *User-Based Collaborative Filtering* dan evaluasi *Precision@K / NDCG@K*.

---

## 3. Cara Mengunduh Ulang & Memproses Ulang Dataset dari Nol

```bash
python3 scripts/download_resources.py
python3 scripts/preprocess_dataset.py
```
