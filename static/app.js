// Antarmuka Interaktif KarirAI — Sistem Rekomendasi Lowongan Kerja (Rust + Firebase)

const state = {
  users: [],
  jobs: [],
  activeUserId: "USR-01",
  lastResponse: null,
  firebaseStatus: null,
};

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => Array.from(document.querySelectorAll(sel));

function showToast(msg) {
  const el = $("#toast");
  el.textContent = msg;
  el.classList.remove("hidden");
  clearTimeout(showToast._t);
  showToast._t = setTimeout(() => el.classList.add("hidden"), 3200);
}

function escapeHtml(str) {
  return String(str ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

// ---------------- TEMA TERANG / GELAP ----------------
function initTheme() {
  const saved = localStorage.getItem("karir_ai_theme") || "light";
  document.documentElement.setAttribute("data-theme", saved);
  $("#btn-theme").textContent = saved === "dark" ? "Mode Terang" : "Mode Gelap";

  $("#btn-theme").addEventListener("click", () => {
    const cur = document.documentElement.getAttribute("data-theme");
    const next = cur === "dark" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", next);
    localStorage.setItem("karir_ai_theme", next);
    $("#btn-theme").textContent = next === "dark" ? "Mode Terang" : "Mode Gelap";
  });
}

// ---------------- NAVIGASI TAB ----------------
function initTabs() {
  $$(".tab-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      $$(".tab-btn").forEach((b) => b.classList.remove("active"));
      $$(".tab-panel").forEach((p) => p.classList.remove("active"));
      btn.classList.add("active");
      const target = btn.getAttribute("data-tab");
      document.getElementById(target).classList.add("active");

      if (target === "tab-cf") loadCfAndHistory();
      if (target === "tab-slides") loadLecturerSimulation();
      if (target === "tab-eval") loadEvaluation();
      if (target === "tab-catalog") renderCatalogTable();
    });
  });
}

// ---------------- BOBOT MULTIKRITERIA BAR (SLIDE 11) ----------------
function readWeightsFromSliders() {
  const w1 = Number($("#rng-w1").value);
  const w2 = Number($("#rng-w2").value);
  const w3 = Number($("#rng-w3").value);
  const w4 = Number($("#rng-w4").value);
  const sum = Math.max(w1 + w2 + w3 + w4, 1);
  return {
    raw: { w1, w2, w3, w4 },
    norm: {
      c1_role_clarity: w1 / sum,
      c2_technical_challenge: w2 / sum,
      c3_compensation: w3 / sum,
      c4_flexibility: w4 / sum,
    },
  };
}

function updateWeightVisuals() {
  const { norm } = readWeightsFromSliders();
  const p1 = Math.round(norm.c1_role_clarity * 100);
  const p2 = Math.round(norm.c2_technical_challenge * 100);
  const p3 = Math.round(norm.c3_compensation * 100);
  const p4 = Math.max(0, 100 - p1 - p2 - p3);

  $("#lbl-w1").textContent = `${p1}%`;
  $("#lbl-w2").textContent = `${p2}%`;
  $("#lbl-w3").textContent = `${p3}%`;
  $("#lbl-w4").textContent = `${p4}%`;

  $("#weight-visual-bar").innerHTML = `
    <div class="weight-seg-1" style="width:${p1}%"></div>
    <div class="weight-seg-2" style="width:${p2}%"></div>
    <div class="weight-seg-3" style="width:${p3}%"></div>
    <div class="weight-seg-4" style="width:${p4}%"></div>
  `;
  $("#weight-visual-legend").innerHTML = `
    <span><strong>C1 Kejelasan:</strong> ${p1}%</span>
    <span><strong>C2 Tantangan:</strong> ${p2}%</span>
    <span><strong>C3 Gaji:</strong> ${p3}%</span>
    <span><strong>C4 Fleksibilitas:</strong> ${p4}%</span>
  `;
}

function populateSidebarFromUser(user) {
  if (!user) return;
  $("#persona-caption").innerHTML = `
    <strong>${escapeHtml(user.role_headline)}</strong><br/>
    <span>${escapeHtml(user.persona_tag)} • Pengalaman: ${user.experience_years} thn</span>
  `;

  $("#rng-w1").value = Math.round(user.weights.c1_role_clarity * 100);
  $("#rng-w2").value = Math.round(user.weights.c2_technical_challenge * 100);
  $("#rng-w3").value = Math.round(user.weights.c3_compensation * 100);
  $("#rng-w4").value = Math.round(user.weights.c4_flexibility * 100);
  updateWeightVisuals();

  $("#inp-min-salary").value = user.constraints.min_salary_idr_millions;
  $("#inp-max-exp").value = user.constraints.max_required_experience_years;
  $("#inp-min-c1").value = user.constraints.min_role_clarity;
  $("#inp-min-c2").value = user.constraints.min_technical_challenge;

  const allowed = new Set((user.constraints.allowed_work_modes || []).map((m) => m.toLowerCase()));
  $$(".chk-mode").forEach((chk) => {
    chk.checked = allowed.has(chk.value.toLowerCase());
  });

  $("#inp-skills").value = (user.preferred_skills || []).join(", ");
}

function readConstraintsFromInputs() {
  const modes = $$(".chk-mode")
    .filter((c) => c.checked)
    .map((c) => c.value);
  return {
    min_salary_idr_millions: Number($("#inp-min-salary").value) || 0,
    max_required_experience_years: Number($("#inp-max-exp").value) || 0,
    allowed_work_modes: modes.length ? modes : ["Remote", "Hybrid", "Onsite"],
    min_role_clarity: Number($("#inp-min-c1").value) || 0,
    min_technical_challenge: Number($("#inp-min-c2").value) || 0,
  };
}

// ---------------- PEMANGGILAN MESIN REKOMENDASI RUST ----------------
async function fetchRecommendations() {
  const mode = $("#select-mode").value;
  const applyHard = $("#chk-constraints").checked;
  const { norm } = readWeightsFromSliders();
  const constraints = readConstraintsFromInputs();

  const res = await fetch("/api/recommend", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      user_id: state.activeUserId,
      mode,
      apply_hard_constraints: applyHard,
      top_k: 10,
      exclude_interacted: true,
      override_weights: norm,
      override_constraints: constraints,
    }),
  });
  const data = await res.json();
  state.lastResponse = data;
  renderRecommendations(data);
}

function renderRecommendations(data) {
  $("#kpi-latency").textContent = `${(data.execution_time_micros / 1000).toFixed(2)} ms`;
  $("#kpi-passed").textContent = `${data.passed_constraints_count}`;
  $("#kpi-total-sub").textContent = `dari ${data.total_catalog_jobs} lowongan katalog`;
  $("#kpi-filtered").textContent = `${data.filtered_out_count}`;
  $("#kpi-interacted").textContent = `${data.already_interacted_count}`;

  const formulaMap = {
    hybrid_multicriteria:
      "<strong>Rumus Aktif (Metode Usulan HMC-CF):</strong> <code>S_Final = 0.40 × S_CB(TF-IDF + Skill) + 0.25 × S_CF(Pearson) + 0.35 × S_MC(4 Kriteria)</code> + <strong>Filter Syarat Wajib</strong> (redistribusi bobot otomatis saat Cold-Start).",
    hybrid_classic:
      "<strong>Rumus Aktif (Hybrid Klasik — Slide 08):</strong> <code>S_Final = 0.60 × S_CB + 0.40 × S_CF</code> (menggabungkan isi lowongan &amp; pola pengguna tanpa aspek multikriteria).",
    content_based:
      "<strong>Rumus Aktif (Cara 1 Content-Based — Slide 05):</strong> <code>S_CB = 0.55 × Cosine_TFIDF(u, j) + 0.35 × SkillCoverage + 0.10 × CategoryBonus</code>.",
    collaborative:
      "<strong>Rumus Aktif (Cara 2 Collaborative — Slide 06):</strong> <code>r̂_CF(u, j) = r̄_u + Σ Sim(u,v)·(r_vj - r̄_v) / Σ |Sim(u,v)|</code> berbasis tetangga terdekat.",
  };
  $("#active-formula-banner").innerHTML = formulaMap[data.algorithm_mode] || "";

  const container = $("#recommendations-list");
  if (!data.recommendations.length) {
    container.innerHTML = `
      <div class="card">
        <h3>Tidak ada lowongan yang lolos syarat wajib saat ini</h3>
        <p class="muted">Longgarkan batas Gaji Minimal, Pengalaman Maksimal, atau Skor Minimal pada panel kiri untuk melihat kandidat.</p>
      </div>`;
  } else {
    container.innerHTML = data.recommendations
      .map((rec, idx) => {
        const j = rec.job;
        const matchedSet = new Set(rec.content_based.matched_skills.map((s) => s.toLowerCase()));
        const skillHtml = j.skills
          .map((sk) => {
            const isMatch = matchedSet.has(sk.toLowerCase());
            return `<span class="skill-chip ${isMatch ? "matched" : ""}">${isMatch ? "✓ " : ""}${escapeHtml(sk)}</span>`;
          })
          .join("");

        return `
        <article class="job-card">
          <div class="job-card-top">
            <div>
              <span class="rank-badge">Peringkat #${rec.rank}</span>
              <h3 class="job-title">${escapeHtml(j.title)}</h3>
              <div class="job-meta">
                <strong>${escapeHtml(j.company)}</strong> • ${escapeHtml(j.category)} •
                ${escapeHtml(j.work_mode)} (${escapeHtml(j.location)}) •
                <strong>Rp ${j.salary_idr_millions.toFixed(1)} Jt/bln</strong> • Min ${j.min_experience_years} thn
              </div>
            </div>
            <div class="score-box">
              <div class="score-pct">${rec.match_percentage.toFixed(1)}%</div>
              <div class="score-raw">Skor: ${rec.final_score.toFixed(4)}</div>
            </div>
          </div>

          <div class="subscore-grid">
            <div class="subscore-pill">
              <span>Content-Based (α=${rec.alpha_cb_weight.toFixed(2)})</span>
              <strong>${rec.content_based.final_cb_score.toFixed(3)} <small>(Cos: ${rec.content_based.tfidf_cosine.toFixed(2)})</small></strong>
            </div>
            <div class="subscore-pill">
              <span>Collaborative (β=${rec.beta_cf_weight.toFixed(2)})</span>
              <strong>${rec.collaborative.has_cf_signal ? `${rec.collaborative.normalized_cf_score.toFixed(3)} <small>(${rec.collaborative.predicted_rating_1_to_5.toFixed(1)}★)</small>` : "Cold-Start"}</strong>
            </div>
            <div class="subscore-pill">
              <span>Multi-Criteria (γ=${rec.gamma_mc_weight.toFixed(2)})</span>
              <strong>${rec.multi_criteria.normalized_mc_score.toFixed(3)} <small>(${rec.multi_criteria.raw_weighted_rating_1_to_5.toFixed(2)}/5)</small></strong>
            </div>
          </div>

          <div class="criteria-mini-row">
            <span class="crit-tag">C1 Kejelasan: ${j.criteria.c1_role_clarity.toFixed(1)}/5</span>
            <span class="crit-tag">C2 Tantangan: ${j.criteria.c2_technical_challenge.toFixed(1)}/5</span>
            <span class="crit-tag">C3 Gaji: ${j.criteria.c3_compensation.toFixed(1)}/5</span>
            <span class="crit-tag">C4 Fleksibilitas: ${j.criteria.c4_flexibility.toFixed(1)}/5</span>
          </div>

          <div class="skills-row">${skillHtml}</div>

          <div class="why-box">
            <strong>Alasan Rekomendasi (Explainable AI):</strong> ${escapeHtml(rec.human_explanation)}
          </div>

          <div class="card-actions">
            <button class="btn btn-outline btn-sm btn-inspect" data-idx="${idx}" type="button">
              🔍 Buka Bukti Perhitungan AI (Rumus &amp; Vektor)
            </button>
            <div class="rate-group">
              <span>Simpan Interaksi ke Firebase:</span>
              <button class="btn btn-ghost btn-sm btn-rate" data-job="${j.id}" data-rating="5" data-action="applied" type="button">★ 5 Lamar</button>
              <button class="btn btn-ghost btn-sm btn-rate" data-job="${j.id}" data-rating="4" data-action="bookmarked" type="button">★ 4 Simpan</button>
              <button class="btn btn-ghost btn-sm btn-rate" data-job="${j.id}" data-rating="2" data-action="viewed" type="button">★ 2 Kurang Cocok</button>
            </div>
          </div>
        </article>`;
      })
      .join("");
  }

  // Render bagian kandidat yang gugur karena Hard Constraints (Slide 14)
  const filtSec = $("#filtered-section");
  const filtList = $("#filtered-out-list");
  if (!data.apply_hard_constraints || !data.filtered_out_examples.length) {
    filtList.innerHTML = `<div class="muted">Tidak ada kandidat yang tereliminasi (atau Filter Syarat Wajib sedang dinonaktifkan).</div>`;
  } else {
    filtList.innerHTML = data.filtered_out_examples
      .map(
        (rec) => `
      <div class="filtered-item">
        <div>
          <strong>${escapeHtml(rec.job.title)}</strong> — ${escapeHtml(rec.job.company)}
          • <strong>Skor Potensial: ${(rec.final_score * 100).toFixed(1)}%</strong>
          (${escapeHtml(rec.job.work_mode)}, Rp ${rec.job.salary_idr_millions.toFixed(1)} Jt, Min ${rec.job.min_experience_years} thn)
        </div>
        <div>
          ${rec.constraints.violations.map((v) => `<span class="violation-tag">✕ ${escapeHtml(v)}</span>`).join("")}
        </div>
      </div>`
      )
      .join("");
  }

  // Pasang event listener tombol Inspector & Rating
  $$(".btn-inspect").forEach((btn) => {
    btn.addEventListener("click", () => {
      const idx = Number(btn.getAttribute("data-idx"));
      openInspectorModal(data.recommendations[idx]);
    });
  });

  $$(".btn-rate").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const jobId = btn.getAttribute("data-job");
      const rating = Number(btn.getAttribute("data-rating"));
      const action = btn.getAttribute("data-action");
      await fetch("/api/interactions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          user_id: state.activeUserId,
          job_id: jobId,
          rating,
          action,
        }),
      });
      showToast(`Interaksi ${action.toUpperCase()} (${rating}★) disimpan ke Firebase & memperbarui rekomendasi!`);
      await fetchRecommendations();
      await loadFirebaseStatus();
    });
  });
}

// ---------------- MODAL WHITE-BOX INSPECTOR RUMUS AI ----------------
function openInspectorModal(rec) {
  if (!rec) return;
  $("#modal-job-title").textContent = `${rec.job.title} (${rec.job.company})`;
  const cb = rec.content_based;
  const cf = rec.collaborative;
  const mc = rec.multi_criteria;

  const termsRows = cb.top_matched_terms.length
    ? cb.top_matched_terms
        .map(
          (t) => `
      <tr>
        <td><code>${escapeHtml(t.term)}</code></td>
        <td>${t.tfidf_user.toFixed(4)}</td>
        <td>${t.tfidf_job.toFixed(4)}</td>
        <td><strong>${t.product.toFixed(4)}</strong></td>
      </tr>`
        )
        .join("")
    : `<tr><td colspan="4">Tidak ada irisan kata kunci langsung</td></tr>`;

  const cfRows = cf.neighbors_used.length
    ? cf.neighbors_used
        .map(
          (n) => `
      <tr>
        <td>${escapeHtml(n.neighbor_name)} (<code>${n.neighbor_id}</code>)</td>
        <td><strong>${n.similarity.toFixed(4)}</strong></td>
        <td>${n.shared_jobs_count} lowongan</td>
        <td>${n.neighbor_rating_on_job.toFixed(1)} / 5.0 (r̄=${n.neighbor_mean_rating.toFixed(2)})</td>
      </tr>`
        )
        .join("")
    : `<tr><td colspan="4">Belum ada tetangga mirip yang menilai lowongan ini (otomatis ditangani mekanisme Cold-Start).</td></tr>`;

  $("#modal-body").innerHTML = `
    <div class="proof-block">
      <h4>Tahap 1 — Content-Based Filtering: TF-IDF &amp; Cosine Similarity (Slide 05)</h4>
      <p>
        <code>Cosine(u, j) = (u · j) / (||u|| × ||j||) = ${cb.dot_product.toFixed(4)} / (${cb.user_norm.toFixed(4)} × ${cb.job_norm.toFixed(4)}) = ${cb.tfidf_cosine.toFixed(4)}</code><br/>
        <code>SkillCoverage = ${rec.content_based.matched_skills.length}/${rec.job.skills.length} = ${cb.skill_overlap_ratio.toFixed(4)}</code> •
        <code>Bidang Cocok = ${cb.category_match ? "Ya (1.0)" : "Tidak (0.0)"}</code><br/>
        <strong>Skor Akhir Content-Based (S_CB):</strong> <code>0.55×${cb.tfidf_cosine.toFixed(4)} + 0.35×${cb.skill_overlap_ratio.toFixed(4)} + 0.10×${cb.category_match ? "1.0" : "0.0"} = ${cb.final_cb_score.toFixed(4)}</code>
      </p>
      <table>
        <thead><tr><th>Token Kata Kunci</th><th>TF-IDF Profil User</th><th>TF-IDF Lowongan</th><th>Hasil Kali (u_i × j_i)</th></tr></thead>
        <tbody>${termsRows}</tbody>
      </table>
    </div>

    <div class="proof-block">
      <h4>Tahap 2 — User-Based Collaborative Filtering: Pearson Neighborhood (Slide 06)</h4>
      <p>
        Rata-rata rating pengguna aktif (<code>r̄_u</code>) = <code>${cf.target_user_mean.toFixed(2)}</code> •
        Prediksi Rating (<code>r̂_CF</code>) = <code>${cf.predicted_rating_1_to_5.toFixed(2)} / 5.00</code> •
        <strong>Skor Ternormalisasi (S_CF) = <code>${cf.normalized_cf_score.toFixed(4)}</code></strong>
      </p>
      <table>
        <thead><tr><th>Tetangga Serupa</th><th>Pearson Sim(u,v)</th><th>Irisan Riwayat</th><th>Rating Tetangga pd Lowongan Ini</th></tr></thead>
        <tbody>${cfRows}</tbody>
      </table>
    </div>

    <div class="proof-block">
      <h4>Tahap 3 — Multi-Criteria Recommender System (Slide 09–13 &amp; Adomavicius &amp; Kwon, 2015)</h4>
      <p><code>${escapeHtml(mc.formula_trace)}</code></p>
    </div>

    <div class="proof-block">
      <h4>Tahap 4 — Penggabungan Hybrid &amp; Pemeriksaan Syarat Wajib (Slide 08 &amp; 14)</h4>
      <p>
        <code>S_Final = (${rec.alpha_cb_weight.toFixed(4)} × ${cb.final_cb_score.toFixed(4)}) + (${rec.beta_cf_weight.toFixed(4)} × ${cf.normalized_cf_score.toFixed(4)}) + (${rec.gamma_mc_weight.toFixed(4)} × ${mc.normalized_mc_score.toFixed(4)}) = ${rec.final_score.toFixed(4)} (${rec.match_percentage.toFixed(1)}%)</code>
      </p>
      <p><strong>Pemeriksaan Hard Constraints:</strong> ${rec.constraints.passed_checks.map((c) => `✓ ${escapeHtml(c)}`).join(" • ")}</p>
    </div>
  `;

  $("#inspector-modal").classList.remove("hidden");
}

// ---------------- TAB 2: MATRIKS CF & RIWAYAT ----------------
async function loadCfAndHistory() {
  const [cfRes, histRes] = await Promise.all([
    fetch(`/api/cf-matrix/${state.activeUserId}`),
    fetch(`/api/users/${state.activeUserId}/interactions`),
  ]);
  const neighbors = await cfRes.json();
  const history = await histRes.json();

  $("#cf-neighbors-table-wrap").innerHTML = `
    <div class="table-responsive">
      <table>
        <thead>
          <tr><th>Pengguna Tetangga</th><th>Fokus Peran</th><th>Irisan Item</th><th>Kemiripan Pearson</th></tr>
        </thead>
        <tbody>
          ${neighbors
            .map(
              (n, i) => `
            <tr class="${i < 3 && n.similarity > 0.1 ? "highlight-row" : ""}">
              <td><strong>${escapeHtml(n.neighbor_name)}</strong> (<code>${n.neighbor_id}</code>)</td>
              <td>${escapeHtml(n.role_headline)}</td>
              <td>${n.shared_jobs_count} lowongan</td>
              <td>${n.similarity.toFixed(4)}</td>
            </tr>`
            )
            .join("")}
        </tbody>
      </table>
    </div>`;

  $("#user-history-wrap").innerHTML = `
    <div class="table-responsive">
      <table>
        <thead>
          <tr><th>Lowongan</th><th>Bidang</th><th>Aksi</th><th>Rating</th></tr>
        </thead>
        <tbody>
          ${history
            .slice()
            .reverse()
            .map(
              (h) => `
            <tr>
              <td><strong>${escapeHtml(h.job_title)}</strong><br/><small class="muted">${escapeHtml(h.company)}</small></td>
              <td>${escapeHtml(h.category)}</td>
              <td><code>${escapeHtml(h.interaction.action)}</code></td>
              <td><strong>${h.interaction.rating.toFixed(1)} ★</strong></td>
            </tr>`
            )
            .join("")}
        </tbody>
      </table>
    </div>`;
}

// ---------------- TAB 3: SIMULASI MATERI PDF DOSEN ----------------
async function loadLecturerSimulation() {
  const wClarity = Number($("#sim-w-clarity").value) / 100.0;
  const minPrac = Number($("#sim-min-practice").value);
  $("#sim-w-clarity-lbl").textContent = `${Math.round(wClarity * 100)}%`;
  $("#sim-w-practice-lbl").textContent = `${Math.round((1 - wClarity) * 100)}%`;
  $("#sim-min-practice-lbl").textContent = minPrac.toFixed(1);

  const res = await fetch(
    `/api/lecturer-case-study?w_clarity=${wClarity.toFixed(2)}&min_practice=${minPrac.toFixed(1)}`
  );
  const d = await res.json();

  $("#slide-static-cards").innerHTML = `
    <div class="slide-box">
      <h4>Slide 09: Apakah Satu Rating Cukup?</h4>
      <p><strong>${escapeHtml(d.item_a_name)}</strong><br/>Kejelasan = <b>${d.item_a_clarity}</b>, Tantangan = <b>${d.item_a_practice}</b> → Rata-rata = <b>${d.item_a_simple_avg.toFixed(1)}</b></p>
      <p><strong>${escapeHtml(d.item_b_name)}</strong><br/>Kejelasan = <b>${d.item_b_clarity}</b>, Tantangan = <b>${d.item_b_practice}</b> → Rata-rata = <b>${d.item_b_simple_avg.toFixed(1)}</b></p>
      <small class="muted">Rata-rata identik (4.0), namun karakteristik kedua lowongan bertolak belakang.</small>
    </div>
    <div class="slide-box">
      <h4>Slide 12: Prioritas Dina (70% Kejelasan)</h4>
      <p><code>Skor A = 0.7×5 + 0.3×3 = ${d.dina_70_30_score_a.toFixed(1)}</code></p>
      <p><code>Skor B = 0.7×3 + 0.3×5 = ${d.dina_70_30_score_b.toFixed(1)}</code></p>
      <p>Pilihan Teratas Dina: <strong>${escapeHtml(d.dina_winner)}</strong></p>
    </div>
    <div class="slide-box">
      <h4>Slide 13: Prioritas Bima (70% Tantangan)</h4>
      <p><code>Skor A = 0.3×5 + 0.7×3 = ${d.bima_30_70_score_a.toFixed(1)}</code></p>
      <p><code>Skor B = 0.3×3 + 0.7×5 = ${d.bima_30_70_score_b.toFixed(1)}</code></p>
      <p>Pilihan Teratas Bima: <strong>${escapeHtml(d.bima_winner)}</strong></p>
    </div>
  `;

  $("#sim-exercise-result").innerHTML = `
    <div style="display:flex;justify-content:space-between;flex-wrap:wrap;gap:12px;">
      <div>
        <strong>Skor Berbobot Lowongan A:</strong> <code>${d.pair_exercise_50_50_score_a.toFixed(2)}</code>
        (${d.item_a_passes_requirement ? "✅ Lolos Syarat" : "❌ Gugur Syarat Tantangan < " + d.min_practice_requirement.toFixed(1)})
      </div>
      <div>
        <strong>Skor Berbobot Lowongan B:</strong> <code>${d.pair_exercise_50_50_score_b.toFixed(2)}</code>
        (${d.item_b_passes_requirement ? "✅ Lolos Syarat" : "❌ Gugur Syarat Tantangan < " + d.min_practice_requirement.toFixed(1)})
      </div>
    </div>
    <h3 style="margin:10px 0 4px;color:var(--primary);">Keputusan Akhir Sistem AI: ${escapeHtml(d.pair_exercise_final_choice)}</h3>
    <p style="margin:0;">${escapeHtml(d.pair_exercise_explanation)}</p>
  `;
}

// ---------------- TAB 4: EVALUASI METRIK ILMIAH ----------------
async function loadEvaluation() {
  const res = await fetch("/api/evaluate");
  const rep = await res.json();

  $("#eval-desc").textContent = `Diuji secara live pada ${rep.evaluated_users_count} profil pencari kerja, ${rep.catalog_jobs_count} katalog lowongan riil, dan ${rep.total_interactions_count} interaksi (${rep.train_interactions_count} train / ${rep.test_interactions_count} test hold-out). Definisi relevan: ${rep.relevance_definition}`;

  $("#eval-bars-chart").innerHTML = rep.methods
    .map(
      (m) => `
    <div class="eval-bar-row">
      <div class="eval-bar-header">
        <span>${escapeHtml(m.method_label)}</span>
        <span>Latensi Rust: <code>${(m.avg_latency_micros / 1000).toFixed(2)} ms</code></span>
      </div>
      <div class="eval-bars-track">
        <div class="metric-mini-bar">
          Precision@3 (Slide 15): <strong>${(m.precision_at_3 * 100).toFixed(1)}%</strong>
          <div class="bar-outer"><div class="bar-fill" style="width:${(m.precision_at_3 * 100).toFixed(1)}%"></div></div>
        </div>
        <div class="metric-mini-bar">
          NDCG@5 (Kualitas Urutan): <strong>${(m.ndcg_at_5 * 100).toFixed(1)}%</strong>
          <div class="bar-outer"><div class="bar-fill ndcg" style="width:${(m.ndcg_at_5 * 100).toFixed(1)}%"></div></div>
        </div>
        <div class="metric-mini-bar">
          Kepatuhan Syarat Wajib: <strong>${(m.constraint_satisfaction_rate * 100).toFixed(1)}%</strong>
          <div class="bar-outer"><div class="bar-fill csr" style="width:${(m.constraint_satisfaction_rate * 100).toFixed(1)}%"></div></div>
        </div>
      </div>
    </div>`
    )
    .join("");

  $("#eval-table-wrap").innerHTML = `
    <table>
      <thead>
        <tr>
          <th>Metode Algoritma</th>
          <th>Precision@3</th>
          <th>Precision@5</th>
          <th>Recall@5</th>
          <th>F1-Score@5</th>
          <th>NDCG@5</th>
          <th>HitRate@5</th>
          <th>Lolos Syarat Wajib</th>
          <th>Latensi (µs)</th>
        </tr>
      </thead>
      <tbody>
        ${rep.methods
          .map(
            (m) => `
          <tr class="${m.method_id === "proposed_hmc_cf" ? "highlight-row" : ""}">
            <td>${escapeHtml(m.method_label)}</td>
            <td>${(m.precision_at_3 * 100).toFixed(2)}%</td>
            <td>${(m.precision_at_5 * 100).toFixed(2)}%</td>
            <td>${(m.recall_at_5 * 100).toFixed(2)}%</td>
            <td>${(m.f1_at_5 * 100).toFixed(2)}%</td>
            <td>${(m.ndcg_at_5 * 100).toFixed(2)}%</td>
            <td>${(m.hit_rate_at_5 * 100).toFixed(1)}%</td>
            <td>${(m.constraint_satisfaction_rate * 100).toFixed(1)}%</td>
            <td><code>${m.avg_latency_micros} µs</code></td>
          </tr>`
          )
          .join("")}
      </tbody>
    </table>`;
}

// ---------------- TAB 5: STATUS FIREBASE & KATALOG DATASET ----------------
async function loadFirebaseStatus() {
  const res = await fetch("/api/firebase/status");
  const st = await res.json();
  state.firebaseStatus = st;

  const badge = $("#firebase-badge");
  badge.classList.toggle("connected", st.connected);
  $("#firebase-badge-text").textContent = st.connected
    ? `Firebase Aktif (${st.project_id})`
    : "Mode Lokal Siap Pakai (Mirror)";

  $("#firebase-detail-box").innerHTML = `
    <div class="subscore-pill"><span>Mode Persistensi</span><strong>${escapeHtml(st.mode)}</strong></div>
    <div class="subscore-pill"><span>Project ID Firebase</span><strong><code>${escapeHtml(st.project_id)}</code></strong></div>
    <div class="subscore-pill"><span>Endpoint Cloud Firestore</span><strong><code>${escapeHtml(st.firestore_endpoint)}</code></strong></div>
    <div class="subscore-pill"><span>Dokumen Tersinkron</span><strong>${st.synced_jobs} Lowongan • ${st.synced_users} User • ${st.synced_interactions} Interaksi</strong></div>
  `;
}

function renderCatalogTable() {
  const q = ($("#inp-search-catalog").value || "").toLowerCase().trim();
  const cat = $("#sel-cat-filter").value;

  const filtered = state.jobs.filter((j) => {
    if (cat && j.category !== cat) return false;
    if (!q) return true;
    const hay = `${j.id} ${j.title} ${j.company} ${j.skills.join(" ")} ${j.work_mode}`.toLowerCase();
    return hay.includes(q);
  });

  $("#catalog-table-wrap").innerHTML = `
    <table>
      <thead>
        <tr>
          <th>ID</th>
          <th>Lowongan &amp; Perusahaan</th>
          <th>Kategori &amp; Mode</th>
          <th>Gaji &amp; Syarat</th>
          <th>Skor 4 Aspek (C1/C2/C3/C4)</th>
          <th>Sumber Dataset</th>
        </tr>
      </thead>
      <tbody>
        ${filtered
          .map(
            (j) => `
          <tr>
            <td><code>${j.id}</code></td>
            <td><strong>${escapeHtml(j.title)}</strong><br/><small class="muted">${escapeHtml(j.company)} • ${escapeHtml(j.skills.slice(0, 5).join(", "))}</small></td>
            <td>${escapeHtml(j.category)}<br/><small>${escapeHtml(j.work_mode)} (${escapeHtml(j.location)})</small></td>
            <td>Rp ${j.salary_idr_millions.toFixed(1)} Jt/bln<br/><small>Min ${j.min_experience_years} thn (${escapeHtml(j.level)})</small></td>
            <td><code>${j.criteria.c1_role_clarity.toFixed(1)} | ${j.criteria.c2_technical_challenge.toFixed(1)} | ${j.criteria.c3_compensation.toFixed(1)} | ${j.criteria.c4_flexibility.toFixed(1)}</code></td>
            <td><small><code>${escapeHtml(j.source_dataset)}</code></small></td>
          </tr>`
          )
          .join("")}
      </tbody>
    </table>`;
}

// ---------------- INISIALISASI UTAMA ----------------
async function bootstrap() {
  initTheme();
  initTabs();

  const [usersRes, jobsRes] = await Promise.all([fetch("/api/users"), fetch("/api/jobs")]);
  state.users = await usersRes.json();
  state.jobs = await jobsRes.json();

  // Isi dropdown persona user
  const selUser = $("#select-user");
  selUser.innerHTML = state.users
    .map((u) => `<option value="${u.id}">${escapeHtml(u.name)} — ${escapeHtml(u.role_headline)}</option>`)
    .join("");
  state.activeUserId = state.users[0]?.id || "USR-01";
  populateSidebarFromUser(state.users[0]);

  // Isi kategori filter katalog
  const cats = [...new Set(state.jobs.map((j) => j.category))].sort();
  $("#sel-cat-filter").innerHTML =
    `<option value="">Semua Kategori Bidang (${state.jobs.length})</option>` +
    cats.map((c) => `<option value="${escapeHtml(c)}">${escapeHtml(c)}</option>`).join("");

  // Event perubahan persona, metode, slider, dan filter
  selUser.addEventListener("change", () => {
    state.activeUserId = selUser.value;
    const u = state.users.find((x) => x.id === state.activeUserId);
    populateSidebarFromUser(u);
    fetchRecommendations();
  });

  $("#select-mode").addEventListener("change", fetchRecommendations);
  $("#chk-constraints").addEventListener("change", fetchRecommendations);

  ["#rng-w1", "#rng-w2", "#rng-w3", "#rng-w4"].forEach((id) => {
    $(id).addEventListener("input", () => {
      updateWeightVisuals();
      fetchRecommendations();
    });
  });

  ["#inp-min-salary", "#inp-max-exp", "#inp-min-c1", "#inp-min-c2"].forEach((id) => {
    $(id).addEventListener("change", fetchRecommendations);
  });
  $$(".chk-mode").forEach((c) => c.addEventListener("change", fetchRecommendations));

  $("#btn-save-prefs").addEventListener("click", async () => {
    const u = state.users.find((x) => x.id === state.activeUserId);
    if (!u) return;
    const skills = $("#inp-skills")
      .value.split(",")
      .map((s) => s.trim().toLowerCase())
      .filter(Boolean);
    const { norm } = readWeightsFromSliders();
    const constraints = readConstraintsFromInputs();

    const res = await fetch(`/api/users/${state.activeUserId}/preferences`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        preferred_skills: skills,
        preferred_categories: u.preferred_categories,
        experience_years: u.experience_years,
        weights: norm,
        constraints,
      }),
    });
    if (res.ok) {
      const updated = await res.json();
      const idx = state.users.findIndex((x) => x.id === updated.id);
      if (idx >= 0) state.users[idx] = updated;
      showToast("Preferensi, bobot multikriteria, & syarat wajib berhasil disimpan ke Firebase!");
      await fetchRecommendations();
    }
  });

  $("#btn-reset-demo").addEventListener("click", async () => {
    await fetch("/api/reset", { method: "POST" });
    const usersRes2 = await fetch("/api/users");
    state.users = await usersRes2.json();
    const cur = state.users.find((x) => x.id === state.activeUserId) || state.users[0];
    populateSidebarFromUser(cur);
    await fetchRecommendations();
    await loadFirebaseStatus();
    showToast("Data berhasil dikembalikan ke kondisi awal dataset.");
  });

  $("#btn-sync-firebase").addEventListener("click", async () => {
    const res = await fetch("/api/firebase/sync", { method: "POST" });
    if (res.ok) {
      await loadFirebaseStatus();
      showToast("Seluruh koleksi berhasil disinkronkan ke Cloud Firestore!");
    } else {
      showToast("Emulator Firestore (:8088) belum dinyalakan. Jalankan dengan --emulator / run-emulator.bat.");
    }
  });

  $("#btn-close-modal").addEventListener("click", () => $("#inspector-modal").classList.add("hidden"));
  $("#inspector-modal").addEventListener("click", (e) => {
    if (e.target.id === "inspector-modal") $("#inspector-modal").classList.add("hidden");
  });

  $("#sim-w-clarity").addEventListener("input", loadLecturerSimulation);
  $("#sim-min-practice").addEventListener("input", loadLecturerSimulation);
  $("#inp-search-catalog").addEventListener("input", renderCatalogTable);
  $("#sel-cat-filter").addEventListener("change", renderCatalogTable);

  await Promise.all([loadFirebaseStatus(), fetchRecommendations()]);
}

document.addEventListener("DOMContentLoaded", bootstrap);
