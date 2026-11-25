#!/usr/bin/env python3
"""
Skrip pengunduh otomatis Dataset Lowongan Kerja Riil & 15 Jurnal Referensi Open-Access.
Menyimpan seluruh berkas secara rapi ke:
  - data/raw/
  - references/journals/
beserta metadata lengkap JSON untuk audit akademis & reproduksibilitas.
"""

import csv
import io
import json
import ssl
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW_DATA_DIR = ROOT / "data" / "raw"
JOURNALS_DIR = ROOT / "references" / "journals"

RAW_DATA_DIR.mkdir(parents=True, exist_ok=True)
JOURNALS_DIR.mkdir(parents=True, exist_ok=True)

CTX = ssl.create_default_context()
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0 Safari/537.36"

PAPERS_15 = [
    {
        "slug": "2021_DeRuijt_Bhulai_Job_Recommender_Systems_Review",
        "doi": "10.48550/arxiv.2111.13576",
        "pdf_urls": ["https://arxiv.org/pdf/2111.13576.pdf"],
        "topic_group": "Taksonomi & Survey Komprehensif Sistem Rekomendasi Kerja",
    },
    {
        "slug": "2017_Yang_KBS_Combining_ContentBased_Collaborative_Job_RecSys",
        "doi": "10.1016/j.knosys.2017.08.017",
        "pdf_urls": ["https://arxiv.org/pdf/1607.01050.pdf"],
        "topic_group": "Hybrid Content-Based & Collaborative Filtering untuk Lowongan Kerja",
    },
    {
        "slug": "2016_Pacuk_RecSysChallenge_Scalable_Hybrid_Job_Recommendations",
        "doi": "10.1145/2987538.2987539",
        "pdf_urls": ["https://biblio.ugent.be/publication/8162090/file/8162096.pdf"],
        "topic_group": "Algoritma Hybrid Berperforma Tinggi pada Rekomendasi Kerja",
    },
    {
        "slug": "2025_JATI_TFIDF_Cosine_Similarity_Rekomendasi_Lowongan_Kerja",
        "doi": "10.36040/jati.v9i1.12406",
        "pdf_urls": [
            "https://ejournal.itn.ac.id/index.php/jati/article/download/12406/7091",
            "https://ejournal.itn.ac.id/jati/article/download/12406/7091",
        ],
        "topic_group": "Content-Based Filtering (TF-IDF & Cosine Similarity) Konteks Indonesia",
    },
    {
        "slug": "2018_Shalaby_IEEEBigData_Graph_Content_Job_Recommendation_Scale",
        "doi": "10.48550/arxiv.1801.00377",
        "pdf_urls": ["https://arxiv.org/pdf/1801.00377.pdf"],
        "topic_group": "Content & Graph-Based Job Recommendation Skala Industri (CareerBuilder)",
    },
    {
        "slug": "2023_Kwiecinski_Benchmarking_Collaborative_Filtering_Job_Recs",
        "doi": "10.48550/arxiv.2301.07946",
        "pdf_urls": ["https://arxiv.org/pdf/2301.07946.pdf"],
        "topic_group": "Benchmarking Collaborative Filtering pada Portal Lowongan Kerja",
    },
    {
        "slug": "2021_Nigam_Embedding_Recommender_Job_Candidate_Matching_Scale",
        "doi": "10.48550/arxiv.2107.00221",
        "pdf_urls": ["https://arxiv.org/pdf/2107.00221.pdf"],
        "topic_group": "Representasi Vektor & Pencocokan Kandidat-Lowongan Skala Besar",
    },
    {
        "slug": "2024_Reciprocal_Recommendations_Job_Search_Best_Of_Both",
        "doi": "10.48550/arxiv.2409.10992",
        "pdf_urls": ["https://arxiv.org/pdf/2409.10992.pdf"],
        "topic_group": "Pencocokan Dua Arah & Preferensi Pencari Kerja",
    },
    {
        "slug": "2016_IJACSA_ItemBased_MultiCriteria_Collaborative_Filtering",
        "doi": "10.14569/ijacsa.2016.070837",
        "pdf_urls": [
            "https://thesai.org/Downloads/Volume7No8/Paper_37-An_Item_based_Multi_Criteria_Collaborative_Filtering_Algorithm.pdf",
            "http://thesai.org/Downloads/Volume7No8/Paper_37-An_Item_based_Multi_Criteria_Collaborative_Filtering_Algorithm.pdf",
        ],
        "topic_group": "Multi-Criteria Collaborative Filtering (MCCF)",
    },
    {
        "slug": "2019_Li_Tuzhilin_Latent_MultiCriteria_Ratings_Recommendations",
        "doi": "10.48550/arxiv.1906.10948",
        "pdf_urls": ["https://arxiv.org/pdf/1906.10948.pdf"],
        "topic_group": "Pemodelan Rating Multikriteria untuk Personalisasi Rekomendasi",
    },
    {
        "slug": "2025_Forouzandeh_Multiview_MultiCriteria_Recommender_Systems",
        "doi": "10.48550/arxiv.2502.19271",
        "pdf_urls": ["https://arxiv.org/pdf/2502.19271.pdf"],
        "topic_group": "Perkembangan Mutakhir Multi-Criteria Recommender Systems (MCRS)",
    },
    {
        "slug": "2025_FrontiersAI_Explainable_Person_Job_Recommendations",
        "doi": "10.3389/frai.2025.1660548",
        "pdf_urls": [
            "https://www.frontiersin.org/journals/artificial-intelligence/articles/10.3389/frai.2025.1660548/pdf"
        ],
        "topic_group": "Explainable AI (XAI) dalam Pencocokan Pencari Kerja & Lowongan",
    },
    {
        "slug": "2025_Schellingerhout_OKRA_Explainable_MultiStakeholder_Job_RecSys",
        "doi": "10.48550/arxiv.2504.07108",
        "pdf_urls": ["https://arxiv.org/pdf/2504.07108.pdf"],
        "topic_group": "Explainable & Constraint-Aware Multi-Stakeholder Job Recommendation",
    },
    {
        "slug": "2020_Xu_Rust_Memory_Safety_Performance_InDepth_Study",
        "doi": "10.48550/arxiv.2003.03296",
        "pdf_urls": ["https://arxiv.org/pdf/2003.03296.pdf"],
        "topic_group": "Keamanan Memori & Performa Sistem Backend Menggunakan Bahasa Rust",
    },
    {
        "slug": "2023_IJEECS_Google_Firebase_Firestore_Optimization_Analysis",
        "doi": "10.11591/ijeecs.v29.i3.pp1719-1728",
        "pdf_urls": [
            "https://ijeecs.iaescore.com/index.php/IJEECS/article/download/30229/17143"
        ],
        "topic_group": "Optimasi Basis Data Cloud Real-Time Google Firebase Firestore",
    },
]


def fetch_openalex_by_doi(doi: str) -> dict:
    url = f"https://api.openalex.org/works/https://doi.org/{urllib.parse.quote(doi, safe='/')}"
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    try:
        with urllib.request.urlopen(req, timeout=20, context=CTX) as resp:
            w = json.loads(resp.read().decode("utf-8"))
        venue = ((w.get("primary_location") or {}).get("source") or {}).get("display_name") or "arXiv / Open Access"
        authors = [a.get("author", {}).get("display_name", "") for a in w.get("authorships", [])]
        return {
            "title": w.get("title"),
            "year": w.get("publication_year"),
            "venue": venue,
            "authors": authors,
            "cited_by_count": w.get("cited_by_count", 0),
            "openalex_id": w.get("id"),
            "doi_url": f"https://doi.org/{doi}",
        }
    except Exception as e:
        return {
            "title": doi,
            "year": None,
            "venue": "Open Access Repository",
            "authors": [],
            "cited_by_count": 0,
            "openalex_id": None,
            "doi_url": f"https://doi.org/{doi}",
            "meta_error": str(e),
        }


def try_download_pdf(urls, dest: Path) -> str | None:
    for u in urls:
        try:
            req = urllib.request.Request(u, headers={"User-Agent": UA, "Accept": "application/pdf,*/*"})
            with urllib.request.urlopen(req, timeout=35, context=CTX) as resp:
                data = resp.read()
            if data.startswith(b"%PDF-") and len(data) > 10_000:
                dest.write_bytes(data)
                return u
        except Exception as e:
            print(f"    [warn] gagal unduh {u}: {e}")
    return None


def download_datasets():
    print("\n=== 1. MENGUNDUH DATASET LOWONGAN KERJA RIIL ===")
    ds1_url = "https://huggingface.co/datasets/zalizedata/tech-job-postings-salary-dataset/resolve/main/sample.csv"
    ds1_path = RAW_DATA_DIR / "tech_job_postings_salary_sample.csv"
    req = urllib.request.Request(ds1_url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=30, context=CTX) as resp:
        raw_bytes = resp.read()
    ds1_path.write_bytes(raw_bytes)
    print(f"  [OK] {ds1_path.name} ({len(raw_bytes):,} bytes)")

    ds2_url = "https://huggingface.co/datasets/xanderios/linkedin-job-postings/resolve/main/job_postings.csv"
    ds2_path = RAW_DATA_DIR / "linkedin_tech_jobs_subset.csv"
    req2 = urllib.request.Request(ds2_url, headers={"Range": "bytes=0-8388608", "User-Agent": UA})
    with urllib.request.urlopen(req2, timeout=45, context=CTX) as resp:
        chunk = resp.read().decode("utf-8", errors="replace")

    last_nl = chunk.rfind("\n")
    if last_nl > 0:
        chunk = chunk[:last_nl]

    tech_keywords = (
        "software", "engineer", "developer", "data", "cloud", "devops", "security",
        "cyber", "rust", "python", "backend", "frontend", "full stack", "fullstack",
        "machine learning", "ai ", "artificial intelligence", "systems", "network",
        "database", "analyst", "qa ", "architect", "sre", "platform", "it "
    )
    reader = csv.DictReader(io.StringIO(chunk))
    selected_rows = []
    fieldnames = reader.fieldnames or []
    for row in reader:
        title_l = (row.get("title") or "").lower()
        desc_l = (row.get("description") or "")
        if len(desc_l) < 120:
            continue
        if any(k in title_l for k in tech_keywords):
            row["description"] = " ".join(desc_l.split())[:1500]
            selected_rows.append(row)
            if len(selected_rows) >= 350:
                break

    with ds2_path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(selected_rows)
    print(f"  [OK] {ds2_path.name} ({len(selected_rows)} baris IT/Tech terkurasi, {ds2_path.stat().st_size:,} bytes)")


def download_journals():
    print("\n=== 2. MENGUNDUH 15 JURNAL REFERENSI OPEN-ACCESS ===")
    for old in JOURNALS_DIR.glob("*.pdf"):
        old.unlink()

    downloaded = []
    for idx, item in enumerate(PAPERS_15, start=1):
        meta = fetch_openalex_by_doi(item["doi"])
        fname = f"J{idx:02d}_{item['slug']}.pdf"
        dest = JOURNALS_DIR / fname
        used_url = try_download_pdf(item["pdf_urls"], dest)
        if not used_url:
            raise RuntimeError(f"Gagal mengunduh PDF jurnal ke-{idx}: {item['doi']}")
        size_kb = dest.stat().st_size / 1024
        print(f"  [OK #{idx:02d}] {fname} ({size_kb:.1f} KB) | {meta['year']} | {meta['title']}")
        downloaded.append({
            "id": f"J{idx:02d}",
            "filename": fname,
            "doi": item["doi"],
            "doi_url": meta["doi_url"],
            "downloaded_pdf_url": used_url,
            "topic_group": item["topic_group"],
            **meta,
        })

    handbook_meta = fetch_openalex_by_doi("10.1007/978-1-4899-7637-6_25")
    manifest = {
        "lecturer_slide_reference": {
            "id": "REF_DOSEN_SLIDE_18",
            "doi": "10.1007/978-1-4899-7637-6_25",
            "google_devs_url": "https://developers.google.com/machine-learning/recommendation/overview",
            "note": "Referensi utama Slide 18 AI_Sistem_Rekomendasi_Visual.pdf: Adomavicius & Kwon (2015) Multi-Criteria Recommender Systems, Recommender Systems Handbook, hlm. 847-880, Springer.",
            **handbook_meta,
        },
        "journals_count": len(downloaded),
        "journals": downloaded,
    }
    manifest_path = JOURNALS_DIR / "journals_manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\nSUKSES PENUH: {len(downloaded)} / 15 PDF jurnal terunduh & terverifikasi di {JOURNALS_DIR}")


if __name__ == "__main__":
    download_datasets()
    download_journals()
