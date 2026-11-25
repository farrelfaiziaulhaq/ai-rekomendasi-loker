#!/usr/bin/env python3
"""
Menghasilkan grafik ilmiah beresolusi tinggi (300 DPI) untuk Karya Ilmiah KarirAI.
"""

from pathlib import Path
import matplotlib.pyplot as plt
import matplotlib.patches as patches
import numpy as np

FIG_DIR = Path("/home/farrel/ai-rekomendasi-loker/paper/figures")
FIG_DIR.mkdir(parents=True, exist_ok=True)

plt.rcParams["font.sans-serif"] = "DejaVu Sans"
plt.rcParams["axes.edgecolor"] = "#CBD5E1"
plt.rcParams["axes.linewidth"] = 0.9


def make_fig1_architecture():
    fig, ax = plt.subplots(figsize=(10.5, 4.6), dpi=300)
    ax.set_xlim(0, 10.5)
    ax.set_ylim(0, 4.6)
    ax.axis("off")

    def box(x, y, w, h, title, body, fc="#EEF5F4", ec="#0F6E72", tc="#0F6E72"):
        rect = patches.FancyBboxPatch(
            (x, y), w, h, boxstyle="round,pad=0.1,rounding_size=0.15",
            facecolor=fc, edgecolor=ec, linewidth=1.6
        )
        ax.add_patch(rect)
        ax.text(x + w / 2, y + h - 0.32, title, ha="center", va="center",
                fontsize=9.2, fontweight="bold", color=tc)
        ax.text(x + w / 2, y + (h - 0.38) / 2, body, ha="center", va="center",
                fontsize=7.8, color="#1E293B", linespacing=1.35)

    # Kolom 1: Data & Cloud
    box(0.25, 2.55, 2.35, 1.65, "1. Dataset & Petunjuk",
        "• LinkedIn & Tech ATS\n  (60 Katalog Lowongan)\n• Rating 1–5, Skill Pilihan,\n  & Riwayat Interaksi",
        fc="#F8FAFC", ec="#475569", tc="#1E293B")
    box(0.25, 0.40, 2.35, 1.65, "2. Google Firebase",
        "• Cloud Firestore REST\n  (jobs, users, interactions)\n• Firebase Auth + Local\n  Mirror Fallback",
        fc="#FDF3EB", ec="#D47432", tc="#B45309")

    # Kolom 2: 3 Komponen AI Paralel di Rust
    box(3.15, 3.15, 2.75, 1.15, "Content-Based (S_CB)",
        "TF-IDF Smoothed + Cosine\n+ Rocchio + Skill Coverage",
        fc="#EEF5F4", ec="#0F6E72", tc="#0F6E72")
    box(3.15, 1.75, 2.75, 1.15, "Collaborative (S_CF)",
        "User-Based Mean-Centered\nPearson Top-K Neighbors",
        fc="#EEF5F4", ec="#0F6E72", tc="#0F6E72")
    box(3.15, 0.35, 2.75, 1.15, "Multi-Criteria (S_MC)",
        "4 Aspek (C1..C4) × Bobot\nPrioritas Dinamis User",
        fc="#EEF5F4", ec="#0F6E72", tc="#0F6E72")

    # Kolom 3: Fusion + Hard Constraint Filter
    box(6.45, 2.35, 1.85, 1.85, "Hybrid & Cold-Start",
        "S = α·S_CB + β·S_CF\n    + γ·S_MC\n(Redistribusi otomatis\nsaat Cold-Start)",
        fc="#EFF6FF", ec="#2563EB", tc="#1D4ED8")
    box(6.45, 0.35, 1.85, 1.65, "Hard Constraints",
        "Syarat Wajib:\nGaji Min, Maks Thn,\nMode Kerja, Skor Min",
        fc="#FEF2F2", ec="#DC2626", tc="#B91C1C")

    # Kolom 4: Output
    box(8.75, 1.15, 1.55, 2.45, "Output White-Box",
        "• Top-K Lowongan\n• Bukti Vektor &\n  Rumus Lengkap\n• Latensi Rust:\n  ~2.24 ms",
        fc="#ECFDF5", ec="#059669", tc="#047857")

    arrow_kw = dict(arrowstyle="-|>,head_width=0.25,head_length=0.4", color="#334155", lw=1.3)
    for target_y in (3.72, 2.32, 0.92):
        ax.annotate("", xy=(3.12, target_y), xytext=(2.65, 3.35 if target_y > 2 else 1.25), arrowprops=arrow_kw)
        ax.annotate("", xy=(6.42, 3.25), xytext=(5.93, target_y), arrowprops=arrow_kw)

    ax.annotate("", xy=(2.60, 2.08), xytext=(2.60, 2.52), arrowprops=dict(arrowstyle="<->", color="#D47432", lw=1.4))
    ax.annotate("", xy=(7.37, 2.03), xytext=(7.37, 2.33), arrowprops=arrow_kw)
    ax.annotate("", xy=(8.72, 2.38), xytext=(8.33, 2.85), arrowprops=arrow_kw)
    ax.annotate("", xy=(8.72, 1.95), xytext=(8.33, 1.20), arrowprops=arrow_kw)

    plt.tight_layout()
    fig.savefig(FIG_DIR / "fig1_pipeline_arsitektur.png", dpi=300, bbox_inches="tight")
    plt.close(fig)


def make_fig2_ablation():
    methods = [
        "Content-Based\n(TF-IDF)",
        "Collaborative\n(Pearson)",
        "Hybrid Klasik\n(CB + CF)",
        "Hybrid MCRS\n(Tanpa Filter)",
        "Usulan HMC-CF\n(MCRS + Filter)",
    ]
    p3 = [47.22, 33.33, 63.89, 63.89, 76.39]
    f1 = [51.02, 30.44, 58.47, 55.82, 76.36]
    ndcg5 = [64.53, 38.87, 70.65, 72.70, 95.01]
    csr = [43.33, 43.33, 50.00, 53.33, 100.00]

    x = np.arange(len(methods))
    w = 0.19

    fig, ax = plt.subplots(figsize=(9.6, 4.5), dpi=300)
    b1 = ax.bar(x - 1.5 * w, p3, w, label="Precision@3 (%)", color="#0F6E72")
    b2 = ax.bar(x - 0.5 * w, f1, w, label="F1-Score@5 (%)", color="#2C6FBB")
    b3 = ax.bar(x + 0.5 * w, ndcg5, w, label="NDCG@5 (%)", color="#D47432")
    b4 = ax.bar(x + 1.5 * w, csr, w, label="Kepatuhan Syarat Wajib / CSR (%)", color="#1B7F5A")

    ax.set_ylabel("Persentase Capaian (%)", fontsize=9.5, fontweight="bold")
    ax.set_title("Perbandingan Performa Pengujian Ablasi 5 Metode pada Dataset Uji (12 Persona, 60 Lowongan)",
                 fontsize=10.5, fontweight="bold", pad=12)
    ax.set_xticks(x)
    ax.set_xticklabels(methods, fontsize=8.5)
    ax.set_ylim(0, 114)
    ax.grid(axis="y", linestyle="--", alpha=0.45)
    ax.legend(loc="upper left", ncol=2, fontsize=8.2, frameon=True)

    for bars in (b1, b3, b4):
        for bar in bars:
            h = bar.get_height()
            ax.text(bar.get_x() + bar.get_width() / 2, h + 1.4, f"{h:.1f}",
                    ha="center", va="bottom", fontsize=6.8, fontweight="bold")

    plt.tight_layout()
    fig.savefig(FIG_DIR / "fig2_ablation_precision_ndcg.png", dpi=300, bbox_inches="tight")
    plt.close(fig)


def make_fig3_sensitivity():
    w_clarity = np.linspace(0, 1, 11)
    score_a = w_clarity * 5.0 + (1 - w_clarity) * 3.0
    score_b = w_clarity * 3.0 + (1 - w_clarity) * 5.0

    fig, ax = plt.subplots(figsize=(7.6, 4.0), dpi=300)
    ax.plot(w_clarity * 100, score_a, marker="o", lw=2.3, color="#0F6E72",
            label="Lowongan A: Junior Mentorship (Kejelasan=5, Tantangan=3)")
    ax.plot(w_clarity * 100, score_b, marker="s", lw=2.3, color="#D47432",
            label="Lowongan B: Systems Lab (Kejelasan=3, Tantangan=5)")

    ax.axvline(70, color="#0F6E72", linestyle=":", lw=1.3)
    ax.annotate("Prioritas Dina (70%)\nA=4.4 > B=3.6", xy=(70, 4.4), xytext=(74, 4.55),
                fontsize=8, fontweight="bold", color="#0F6E72",
                arrowprops=dict(arrowstyle="->", color="#0F6E72"))

    ax.axvline(30, color="#D47432", linestyle=":", lw=1.3)
    ax.annotate("Prioritas Bima (30%)\nB=4.4 > A=3.6", xy=(30, 4.4), xytext=(6, 4.55),
                fontsize=8, fontweight="bold", color="#B45309",
                arrowprops=dict(arrowstyle="->", color="#D47432"))

    ax.scatter([50], [4.0], s=70, color="#DC2626", zorder=5)
    ax.annotate("Titik Seri 50%-50% (Skor=4.0)\nDiselesaikan Hard Filter (C2 >= 4 -> B)",
                xy=(50, 4.0), xytext=(33, 3.18),
                fontsize=7.8, fontweight="bold", color="#B91C1C",
                arrowprops=dict(arrowstyle="->", color="#DC2626"))

    ax.set_xlabel("Bobot Kepentingan Aspek Kejelasan Peran (%)", fontsize=9.2, fontweight="bold")
    ax.set_ylabel("Skor Utilitas Multikriteria (Skala 1–5)", fontsize=9.2, fontweight="bold")
    ax.set_title("Analisis Sensitivitas Bobot Multikriteria & Pemecahan Seri dengan Hard Filter",
                 fontsize=10, fontweight="bold")
    ax.set_ylim(2.8, 5.15)
    ax.grid(True, linestyle="--", alpha=0.45)
    ax.legend(loc="lower center", fontsize=8)

    plt.tight_layout()
    fig.savefig(FIG_DIR / "fig3_kontras_multikriteria.png", dpi=300, bbox_inches="tight")
    plt.close(fig)


if __name__ == "__main__":
    make_fig1_architecture()
    make_fig2_ablation()
    make_fig3_sensitivity()
    print("[OK] 3 grafik ilmiah berhasil dibuat di paper/figures/")
