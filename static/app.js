// KarirAI: Antarmuka Pencarian & Rekomendasi Lowongan Kerja (Rust + Google Firebase)

const state = {
  users: [],
  jobs: [],
  activeUserId: "USR-01",
  quickMode: "all",
  lastResponse: null,
  activeModalRec: null,
  lastFocusedEl: null,
  evalLoaded: false,
};

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => Array.from(document.querySelectorAll(sel));

function esc(str) {
  return String(str ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function initials(name) {
  const words = String(name || "KA")
    .replace(/[^a-zA-Z0-9\s]/g, "")
    .trim()
    .split(/\s+/)
    .filter(Boolean);
  if (words.length >= 2) return (words[0][0] + words[1][0]).toUpperCase();
  return (words[0] || "KA").slice(0, 2).toUpperCase();
}

function showToast(msg) {
  const el = $("#toast");
  el.textContent = msg;
  el.classList.remove("hidden");
  clearTimeout(showToast._timer);
  showToast._timer = setTimeout(() => el.classList.add("hidden"), 3000);
}

// ---------------- TEMA TERANG / GELAP (R-21 & R-34) ----------------
function applyTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("karir_ai_theme", theme);
  const isDark = theme === "dark";
  const btn = $("#btn-theme");
  btn.setAttribute("aria-pressed", String(isDark));
  btn.setAttribute("aria-label", isDark ? "Ubah ke mode terang" : "Ubah ke mode gelap");
  $("#icon-moon").classList.toggle("hidden", isDark);
  $("#icon-sun").classList.toggle("hidden", !isDark);
}

function initTheme() {
  const saved = localStorage.getItem("karir_ai_theme") || "light";
  applyTheme(saved);
  $("#btn-theme").addEventListener("click", () => {
    const cur = document.documentElement.getAttribute("data-theme");
    applyTheme(cur === "dark" ? "light" : "dark");
  });
}

// ---------------- NAVIGASI 3 HALAMAN UTAMA ----------------
function switchView(viewId) {
  $$(".nav-link").forEach((btn) => {
    const isTarget = btn.getAttribute("data-view") === viewId;
    btn.classList.toggle("active", isTarget);
    if (isTarget) {
      btn.setAttribute("aria-current", "page");
    } else {
      btn.removeAttribute("aria-current");
    }
  });

  $$(".view-section").forEach((sec) => {
    sec.classList.toggle("active", sec.id === viewId);
  });

  if (viewId === "view-activity") {
    loadActivityView();
  } else if (viewId === "view-lab") {
    loadLabView();
  }
}

function initNavigation() {
  $$(".nav-link").forEach((btn) => {
    btn.addEventListener("click", () => switchView(btn.getAttribute("data-view")));
  });

  const wordmark = $(".wordmark");
  if (wordmark) {
    wordmark.addEventListener("click", (e) => {
      e.preventDefault();
      switchView("view-jobs");
    });
  }

  const toggleBtn = $("#btn-toggle-filters");
  const filterPanel = $("#filter-panel");
  toggleBtn.addEventListener("click", () => {
    const nextOpen = !filterPanel.classList.contains("open");
    filterPanel.classList.toggle("open", nextOpen);
    toggleBtn.setAttribute("aria-expanded", String(nextOpen));
  });
}

// ---------------- BOBOT & FILTER ----------------
function readNormalizedWeights() {
  const w1 = Number($("#rng-w1").value);
  const w2 = Number($("#rng-w2").value);
  const w3 = Number($("#rng-w3").value);
  const w4 = Number($("#rng-w4").value);
  const sum = Math.max(w1 + w2 + w3 + w4, 1);
  return {
    c1_role_clarity: w1 / sum,
    c2_technical_challenge: w2 / sum,
    c3_compensation: w3 / sum,
    c4_flexibility: w4 / sum,
  };
}

function renderWeightBar() {
  const w = readNormalizedWeights();
  const p1 = Math.round(w.c1_role_clarity * 100);
  const p2 = Math.round(w.c2_technical_challenge * 100);
  const p3 = Math.round(w.c3_compensation * 100);
  const p4 = Math.max(0, 100 - p1 - p2 - p3);

  $("#lbl-w1").textContent = `${p1}%`;
  $("#lbl-w2").textContent = `${p2}%`;
  $("#lbl-w3").textContent = `${p3}%`;
  $("#lbl-w4").textContent = `${p4}%`;

  $("#weight-bar").innerHTML = `
    <div class="wb-1" style="width:${p1}%"></div>
    <div class="wb-2" style="width:${p2}%"></div>
    <div class="wb-3" style="width:${p3}%"></div>
    <div class="wb-4" style="width:${p4}%"></div>
  `;
}

function readConstraints() {
  const user = state.users.find((u) => u.id === state.activeUserId);
  let allowedModes = user?.constraints?.allowed_work_modes || ["Remote", "Hybrid", "Onsite"];
  if (state.quickMode !== "all") {
    allowedModes = [state.quickMode];
  }
  return {
    min_salary_idr_millions: Number($("#inp-min-salary").value) || 0,
    max_required_experience_years: Number($("#inp-max-exp").value) || 0,
    allowed_work_modes: allowedModes,
    min_role_clarity: Number($("#inp-min-c1").value) || 0,
    min_technical_challenge: Number($("#inp-min-c2").value) || 0,
  };
}

function syncQuickModeButtons(mode) {
  state.quickMode = mode;
  $$("[data-quick-mode]").forEach((btn) => {
    btn.classList.toggle("active", btn.getAttribute("data-quick-mode") === mode);
  });
}

function populateUserControls(user) {
  if (!user) return;
  $("#active-profile-line").textContent =
    `${user.name} (${user.role_headline}, pengalaman ${user.experience_years} tahun)`;

  $("#rng-w1").value = Math.round(user.weights.c1_role_clarity * 100);
  $("#rng-w2").value = Math.round(user.weights.c2_technical_challenge * 100);
  $("#rng-w3").value = Math.round(user.weights.c3_compensation * 100);
  $("#rng-w4").value = Math.round(user.weights.c4_flexibility * 100);
  renderWeightBar();

  $("#inp-min-salary").value = user.constraints.min_salary_idr_millions;
  $("#inp-max-exp").value = user.constraints.max_required_experience_years;
  $("#inp-min-c1").value = user.constraints.min_role_clarity;
  $("#inp-min-c2").value = user.constraints.min_technical_challenge;
  $("#inp-skills").value = (user.preferred_skills || []).join(", ");

  const modes = user.constraints.allowed_work_modes || [];
  if (modes.length === 1) {
    syncQuickModeButtons(modes[0]);
  } else {
    syncQuickModeButtons("all");
  }
}

// ---------------- FETCH & RENDER LOWONGAN (WITH LOADING / ERROR / EMPTY STATES R-27) ----------------
async function fetchRecommendations() {
  const loadingEl = $("#feed-loading");
  const errorEl = $("#feed-error");
  const listEl = $("#jobs-list");

  errorEl.classList.add("hidden");
  loadingEl.classList.remove("hidden");
  listEl.innerHTML = "";

  try {
    const res = await fetch("/api/recommend", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        user_id: state.activeUserId,
        mode: $("#select-mode").value,
        apply_hard_constraints: $("#chk-constraints").checked,
        top_k: 12,
        exclude_interacted: true,
        search_query: $("#inp-query").value.trim() || null,
        category_filter: $("#sel-category").value || null,
        override_weights: readNormalizedWeights(),
        override_constraints: readConstraints(),
      }),
    });

    if (!res.ok) {
      throw new Error(`Server mengembalikan status ${res.status}`);
    }

    const data = await res.json();
    state.lastResponse = data;
    renderJobFeed(data);
  } catch (err) {
    $("#feed-error-text").textContent = `Gagal memuat daftar lowongan: ${err.message}`;
    errorEl.classList.remove("hidden");
    $("#results-count").textContent = "Tidak dapat terhubung ke server.";
  } finally {
    loadingEl.classList.add("hidden");
  }
}

function renderJobFeed(data) {
  const count = data.recommendations.length;
  const q = $("#inp-query").value.trim();
  const queryLabel = q ? ` untuk "${q}"` : "";
  $("#results-count").textContent =
    count > 0
      ? `Menampilkan ${count} lowongan cocok${queryLabel}`
      : `Tidak ada lowongan yang cocok${queryLabel}`;

  const ms = (data.execution_time_micros / 1000).toFixed(2);
  $("#latency-readout").textContent = `Komputasi Rust ${ms} ms`;

  const listEl = $("#jobs-list");
  if (!count) {
    listEl.innerHTML = `
      <div class="empty-state">
        <h3>Belum ada lowongan yang sesuai filter saat ini</h3>
        <p>Coba hapus kata kunci pencarian atau longgarkan batas syarat wajib (gaji minimal, pengalaman, atau mode kerja).</p>
        <button type="button" id="btn-relax-filter" class="btn btn-outline">Tampilkan tanpa syarat ketat</button>
      </div>`;
    $("#btn-relax-filter").addEventListener("click", () => {
      $("#inp-query").value = "";
      $("#sel-category").value = "";
      $("#chk-constraints").checked = false;
      syncQuickModeButtons("all");
      fetchRecommendations();
    });
  } else {
    listEl.innerHTML = data.recommendations
      .map((rec, idx) => {
        const j = rec.job;
        const matched = new Set(rec.content_based.matched_skills.map((s) => s.toLowerCase()));
        const skillTags = j.skills
          .slice(0, 5)
          .map((sk) => {
            const cls = matched.has(sk.toLowerCase()) ? "skill-tag matched" : "skill-tag";
            return `<span class="${cls}">${esc(sk)}</span>`;
          })
          .join("");

        return `
        <article class="job-item">
          <div class="job-item-head">
            <div class="job-head-left">
              <div class="company-monogram" aria-hidden="true">${esc(initials(j.company))}</div>
              <div>
                <button type="button" class="job-title-btn btn-open-detail" data-idx="${idx}" data-tab="overview">
                  ${esc(j.title)}
                </button>
                <p class="job-company">${esc(j.company)} &middot; ${esc(j.location)}</p>
              </div>
            </div>

            <div class="job-head-right">
              <span class="match-label">${Math.round(rec.match_percentage)}% cocok</span>
              <div class="salary-label">Rp ${j.salary_idr_millions.toFixed(1)} Jt/bln</div>
            </div>
          </div>

          <div class="job-facts">
            <span class="fact-dot">${esc(j.work_mode)}</span>
            <span class="fact-dot">${esc(j.category)}</span>
            <span class="fact-dot">Min ${j.min_experience_years} thn pengalaman</span>
            <span class="fact-dot">Kejelasan ${j.criteria.c1_role_clarity.toFixed(1)}/5</span>
            <span class="fact-dot">Tantangan ${j.criteria.c2_technical_challenge.toFixed(1)}/5</span>
          </div>

          <div class="job-skills">${skillTags}</div>

          <p class="job-reason">${esc(rec.human_explanation)}.</p>

          <div class="job-actions">
            <button type="button" class="btn btn-text btn-open-detail" data-idx="${idx}" data-tab="math">
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="12" y1="16" x2="12" y2="12"></line>
                <line x1="12" y1="8" x2="12.01" y2="8"></line>
              </svg>
              Lihat hitungan AI
            </button>

            <div class="job-actions-right">
              <button type="button" class="btn btn-outline btn-sm btn-act" data-job="${j.id}" data-rating="4" data-action="bookmarked">
                Simpan
              </button>
              <button type="button" class="btn btn-primary btn-sm btn-act" data-job="${j.id}" data-rating="5" data-action="applied">
                Lamar
              </button>
            </div>
          </div>
        </article>`;
      })
      .join("");
  }

  // Accordion lowongan yang gugur syarat wajib (bukti Slide 14 tetap mudah diakses tanpa mengotori feed)
  const drawer = $("#filtered-drawer");
  if (data.apply_hard_constraints && data.filtered_out_count > 0) {
    drawer.classList.remove("hidden");
    $("#filtered-summary-label").textContent =
      `Lihat ${data.filtered_out_count} lowongan yang tidak memenuhi syarat wajib Anda`;
    $("#filtered-out-list").innerHTML = data.filtered_out_examples
      .map(
        (rec) => `
      <div class="filtered-row">
        <div class="filtered-row-title">
          ${esc(rec.job.title)} (${esc(rec.job.company)}, Rp ${rec.job.salary_idr_millions.toFixed(1)} Jt/bln, ${esc(rec.job.work_mode)})
          <span class="meta-text">Skor potensial ${Math.round(rec.match_percentage)}%</span>
        </div>
        <p class="violation-list">Alasan disaring: ${esc(rec.constraints.violations.join("; "))}</p>
      </div>`
      )
      .join("");
  } else {
    drawer.classList.add("hidden");
  }

  $$(".btn-open-detail").forEach((btn) => {
    btn.addEventListener("click", () => {
      const idx = Number(btn.getAttribute("data-idx"));
      const tab = btn.getAttribute("data-tab") || "overview";
      openDetailModal(data.recommendations[idx], tab, btn);
    });
  });

  $$(".btn-act").forEach((btn) => {
    btn.addEventListener("click", () => {
      submitInteraction(
        btn.getAttribute("data-job"),
        Number(btn.getAttribute("data-rating")),
        btn.getAttribute("data-action")
      );
    });
  });
}

async function submitInteraction(jobId, rating, action) {
  try {
    const res = await fetch("/api/interactions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        user_id: state.activeUserId,
        job_id: jobId,
        rating,
        action,
      }),
    });
    if (!res.ok) throw new Error("Gagal menyimpan interaksi");
    state.evalLoaded = false;
    const label = action === "applied" ? "Lamaran tercatat" : "Lowongan disimpan";
    showToast(`${label} dan rekomendasi diperbarui.`);
    closeDetailModal();
    await fetchRecommendations();
    await loadFirebaseStatus();
  } catch (err) {
    showToast(`Terjadi kesalahan: ${err.message}`);
  }
}

// ---------------- MODAL DETAIL & WHITE-BOX HITUNGAN AI (R-32 ESC + FOCUS RETURN) ----------------
function setModalTab(tabName) {
  $$(".modal-tab").forEach((btn) => {
    const active = btn.getAttribute("data-mtab") === tabName;
    btn.classList.toggle("active", active);
    btn.setAttribute("aria-selected", String(active));
  });
  $("#modal-panel-overview").classList.toggle("hidden", tabName !== "overview");
  $("#modal-panel-math").classList.toggle("hidden", tabName !== "math");
}

function openDetailModal(rec, initialTab, triggerEl) {
  if (!rec) return;
  state.activeModalRec = rec;
  state.lastFocusedEl = triggerEl || document.activeElement;

  const j = rec.job;
  const cb = rec.content_based;
  const cf = rec.collaborative;
  const mc = rec.multi_criteria;

  $("#modal-title").textContent = j.title;
  $("#modal-subtitle").textContent =
    `${j.company} | ${j.location} (${j.work_mode}) | Rp ${j.salary_idr_millions.toFixed(1)} Jt/bln | Kecocokan ${Math.round(rec.match_percentage)}%`;

  const matchedSet = new Set(cb.matched_skills.map((s) => s.toLowerCase()));
  const skillsMarkup = j.skills
    .map((sk) => {
      const cls = matchedSet.has(sk.toLowerCase()) ? "skill-tag matched" : "skill-tag";
      return `<span class="${cls}">${esc(sk)}</span>`;
    })
    .join("");

  $("#modal-panel-overview").innerHTML = `
    <div class="detail-section">
      <h3>Deskripsi pekerjaan</h3>
      <p>${esc(j.description)}</p>
    </div>

    <div class="detail-section">
      <h3>Keahlian dibutuhkan (${cb.matched_skills.length} dari ${j.skills.length} sesuai profil Anda)</h3>
      <div class="job-skills">${skillsMarkup}</div>
    </div>

    <div class="detail-section">
      <h3>Penilaian 4 aspek lowongan (skala 1 sampai 5)</h3>
      <div class="criteria-grid">
        <div class="criteria-box">Kejelasan peran<strong>${j.criteria.c1_role_clarity.toFixed(1)}</strong></div>
        <div class="criteria-box">Tantangan teknis<strong>${j.criteria.c2_technical_challenge.toFixed(1)}</strong></div>
        <div class="criteria-box">Kompensasi gaji<strong>${j.criteria.c3_compensation.toFixed(1)}</strong></div>
        <div class="criteria-box">Fleksibilitas<strong>${j.criteria.c4_flexibility.toFixed(1)}</strong></div>
      </div>
    </div>

    <div class="detail-section">
      <h3>Pemeriksaan syarat wajib Anda</h3>
      <p class="muted-text">${esc(rec.constraints.passed_checks.join(" | "))}</p>
    </div>
  `;

  const termRows = cb.top_matched_terms.length
    ? cb.top_matched_terms
        .map(
          (t) => `
      <tr>
        <td><code>${esc(t.term)}</code></td>
        <td>${t.tfidf_user.toFixed(4)}</td>
        <td>${t.tfidf_job.toFixed(4)}</td>
        <td><strong>${t.product.toFixed(4)}</strong></td>
      </tr>`
        )
        .join("")
    : `<tr><td colspan="4">Tidak ada irisan kata kunci langsung.</td></tr>`;

  const cfRows = cf.neighbors_used.length
    ? cf.neighbors_used
        .map(
          (n) => `
      <tr>
        <td>${esc(n.neighbor_name)} (${n.neighbor_id})</td>
        <td><strong>${n.similarity.toFixed(4)}</strong></td>
        <td>${n.shared_jobs_count} lowongan</td>
        <td>${n.neighbor_rating_on_job.toFixed(1)} / 5.0</td>
      </tr>`
        )
        .join("")
    : `<tr><td colspan="4">Belum ada tetangga mirip yang menilai lowongan ini (otomatis ditangani bobot Cold-Start).</td></tr>`;

  $("#modal-panel-math").innerHTML = `
    <div class="detail-section">
      <h3>1. Content-Based Filtering (TF-IDF &amp; Cosine Similarity)</h3>
      <div class="formula-line">Cosine(u, j) = ${cb.dot_product.toFixed(4)} / (${cb.user_norm.toFixed(4)} * ${cb.job_norm.toFixed(4)}) = ${cb.tfidf_cosine.toFixed(4)} | S_CB = ${cb.final_cb_score.toFixed(4)}</div>
      <div class="table-scroll">
        <table>
          <thead><tr><th>Kata kunci</th><th>TF-IDF Profil</th><th>TF-IDF Lowongan</th><th>Kontribusi</th></tr></thead>
          <tbody>${termRows}</tbody>
        </table>
      </div>
    </div>

    <div class="detail-section">
      <h3>2. Collaborative Filtering (User-Based Pearson)</h3>
      <div class="formula-line">Rata-rata user = ${cf.target_user_mean.toFixed(2)} | Prediksi rating = ${cf.predicted_rating_1_to_5.toFixed(2)} / 5.00 | S_CF = ${cf.normalized_cf_score.toFixed(4)}</div>
      <div class="table-scroll">
        <table>
          <thead><tr><th>Pengguna serupa</th><th>Kemiripan Pearson</th><th>Irisan riwayat</th><th>Rating pada lowongan ini</th></tr></thead>
          <tbody>${cfRows}</tbody>
        </table>
      </div>
    </div>

    <div class="detail-section">
      <h3>3. Skor Multikriteria (4 Aspek Berbobot)</h3>
      <div class="formula-line">${esc(mc.formula_trace)}</div>
    </div>

    <div class="detail-section">
      <h3>4. Skor Akhir Hybrid</h3>
      <div class="formula-line">S_Final = (${rec.alpha_cb_weight.toFixed(4)} * ${cb.final_cb_score.toFixed(4)}) + (${rec.beta_cf_weight.toFixed(4)} * ${cf.normalized_cf_score.toFixed(4)}) + (${rec.gamma_mc_weight.toFixed(4)} * ${mc.normalized_mc_score.toFixed(4)}) = ${rec.final_score.toFixed(4)} (${rec.match_percentage.toFixed(1)}%)</div>
    </div>
  `;

  setModalTab(initialTab);
  $("#detail-modal").classList.remove("hidden");
  $("#btn-close-modal").focus();
}

function closeDetailModal() {
  const modal = $("#detail-modal");
  if (modal.classList.contains("hidden")) return;
  modal.classList.add("hidden");
  if (state.lastFocusedEl && typeof state.lastFocusedEl.focus === "function") {
    state.lastFocusedEl.focus();
  }
}

// ---------------- VIEW 2: AKTIVITAS & COLLABORATIVE ----------------
async function loadActivityView() {
  const [histRes, cfRes] = await Promise.all([
    fetch(`/api/users/${state.activeUserId}/interactions`),
    fetch(`/api/cf-matrix/${state.activeUserId}`),
  ]);
  const history = await histRes.json();
  const neighbors = await cfRes.json();

  $("#history-count").textContent = `${history.length} aktivitas`;
  if (!history.length) {
    $("#user-history-wrap").innerHTML = `<p class="muted-text">Belum ada lamaran atau lowongan yang disimpan.</p>`;
  } else {
    $("#user-history-wrap").innerHTML = `
      <div class="table-scroll">
        <table>
          <thead><tr><th>Posisi &amp; perusahaan</th><th>Status</th><th>Rating</th></tr></thead>
          <tbody>
            ${history
              .slice()
              .reverse()
              .map((h) => {
                const act = h.interaction.action;
                const actLabel = act === "applied" ? "Dilamar" : act === "bookmarked" ? "Disimpan" : "Dilihat";
                return `
                <tr>
                  <td><strong>${esc(h.job_title)}</strong><br/><span class="muted-text">${esc(h.company)} (${esc(h.work_mode)})</span></td>
                  <td><span class="status-chip ${esc(act)}">${actLabel}</span></td>
                  <td><strong>${h.interaction.rating.toFixed(1)}</strong> / 5</td>
                </tr>`;
              })
              .join("")}
          </tbody>
        </table>
      </div>`;
  }

  $("#cf-neighbors-wrap").innerHTML = `
    <div class="table-scroll">
      <table>
        <thead><tr><th>Pengguna</th><th>Irisan</th><th>Pearson</th></tr></thead>
        <tbody>
          ${neighbors
            .map(
              (n, i) => `
            <tr class="${i < 3 && n.similarity > 0.1 ? "row-accent" : ""}">
              <td><strong>${esc(n.neighbor_name)}</strong><br/><span class="muted-text">${esc(n.role_headline)}</span></td>
              <td>${n.shared_jobs_count} item</td>
              <td><strong>${n.similarity.toFixed(4)}</strong></td>
            </tr>`
            )
            .join("")}
        </tbody>
      </table>
    </div>`;
}

// ---------------- VIEW 3: LAB EVALUASI AI ----------------
async function loadLabView() {
  await Promise.all([loadEvaluation(), loadLecturerSimulation(), loadFirebaseStatus()]);
  renderCatalogTable();
}

async function loadEvaluation() {
  const res = await fetch("/api/evaluate");
  const rep = await res.json();
  state.evalLoaded = true;

  $("#eval-meta").textContent = `Indeks dibangun dalam ${(rep.index_build_micros / 1000).toFixed(2)} ms`;
  $("#eval-desc").textContent =
    `Pengujian pada ${rep.evaluated_users_count} profil pencari kerja, ${rep.catalog_jobs_count} lowongan riil, dan ${rep.total_interactions_count} interaksi (${rep.train_interactions_count} train / ${rep.test_interactions_count} test hold-out).`;

  $("#eval-bars").innerHTML = rep.methods
    .map(
      (m) => `
    <div class="eval-bar-item">
      <div class="eval-bar-top">
        <span>${esc(m.method_label)}</span>
        <span class="meta-text">${m.avg_latency_micros} us (${(m.avg_latency_micros / 1000).toFixed(2)} ms)</span>
      </div>
      <div class="eval-bar-tracks">
        <div>
          Precision@3: <strong>${(m.precision_at_3 * 100).toFixed(1)}%</strong>
          <div class="track-bg"><div class="track-fill" style="width:${(m.precision_at_3 * 100).toFixed(1)}%"></div></div>
        </div>
        <div>
          NDCG@5: <strong>${(m.ndcg_at_5 * 100).toFixed(1)}%</strong>
          <div class="track-bg"><div class="track-fill" style="width:${(m.ndcg_at_5 * 100).toFixed(1)}%"></div></div>
        </div>
        <div>
          Lolos syarat wajib: <strong>${(m.constraint_satisfaction_rate * 100).toFixed(1)}%</strong>
          <div class="track-bg"><div class="track-fill green" style="width:${(m.constraint_satisfaction_rate * 100).toFixed(1)}%"></div></div>
        </div>
      </div>
    </div>`
    )
    .join("");

  $("#eval-table-wrap").innerHTML = `
    <table>
      <thead>
        <tr>
          <th>Metode</th>
          <th>P@3</th>
          <th>P@5</th>
          <th>Recall@5</th>
          <th>F1@5</th>
          <th>NDCG@5</th>
          <th>Syarat wajib</th>
          <th>Latensi</th>
        </tr>
      </thead>
      <tbody>
        ${rep.methods
          .map(
            (m) => `
          <tr class="${m.method_id === "proposed_hmc_cf" ? "row-accent" : ""}">
            <td>${esc(m.method_label)}</td>
            <td>${(m.precision_at_3 * 100).toFixed(2)}%</td>
            <td>${(m.precision_at_5 * 100).toFixed(2)}%</td>
            <td>${(m.recall_at_5 * 100).toFixed(2)}%</td>
            <td>${(m.f1_at_5 * 100).toFixed(2)}%</td>
            <td>${(m.ndcg_at_5 * 100).toFixed(2)}%</td>
            <td>${(m.constraint_satisfaction_rate * 100).toFixed(1)}%</td>
            <td><code>${m.avg_latency_micros} us</code></td>
          </tr>`
          )
          .join("")}
      </tbody>
    </table>`;
}

async function loadLecturerSimulation() {
  const wClarity = Number($("#sim-w-clarity").value) / 100;
  const minPrac = Number($("#sim-min-practice").value);
  $("#sim-w-clarity-lbl").textContent = `${Math.round(wClarity * 100)}%`;
  $("#sim-w-practice-lbl").textContent = `${Math.round((1 - wClarity) * 100)}%`;
  $("#sim-min-practice-lbl").textContent = minPrac.toFixed(1);

  const res = await fetch(
    `/api/lecturer-case-study?w_clarity=${wClarity.toFixed(2)}&min_practice=${minPrac.toFixed(1)}`
  );
  const d = await res.json();

  $("#slide-cards").innerHTML = `
    <div class="case-card">
      <h4>Dua lowongan rata-rata sama (4.0)</h4>
      <p><strong>Lowongan A:</strong> Kejelasan ${d.item_a_clarity}, Tantangan ${d.item_a_practice}</p>
      <p><strong>Lowongan B:</strong> Kejelasan ${d.item_b_clarity}, Tantangan ${d.item_b_practice}</p>
    </div>
    <div class="case-card">
      <h4>Prioritas Dina (70% Kejelasan)</h4>
      <p>Skor A = <strong>${d.dina_70_30_score_a.toFixed(1)}</strong>, Skor B = ${d.dina_70_30_score_b.toFixed(1)}</p>
      <p>Terpilih: <strong>${esc(d.dina_winner)}</strong></p>
    </div>
    <div class="case-card">
      <h4>Prioritas Bima (70% Tantangan)</h4>
      <p>Skor A = ${d.bima_30_70_score_a.toFixed(1)}, Skor B = <strong>${d.bima_30_70_score_b.toFixed(1)}</strong></p>
      <p>Terpilih: <strong>${esc(d.bima_winner)}</strong></p>
    </div>
  `;

  $("#sim-result").innerHTML = `
    <p style="margin:0 0 6px;">
      <strong>Lowongan A:</strong> ${d.pair_exercise_50_50_score_a.toFixed(2)} (${d.item_a_passes_requirement ? "Lolos syarat" : "Gugur syarat"}) &middot;
      <strong>Lowongan B:</strong> ${d.pair_exercise_50_50_score_b.toFixed(2)} (${d.item_b_passes_requirement ? "Lolos syarat" : "Gugur syarat"})
    </p>
    <p style="margin:0;"><strong>Keputusan: ${esc(d.pair_exercise_final_choice)}.</strong> ${esc(d.pair_exercise_explanation)}</p>
  `;
}

async function loadFirebaseStatus() {
  const res = await fetch("/api/firebase/status");
  const st = await res.json();

  const footerStatus = $("#footer-fb-status");
  footerStatus.textContent = st.connected
    ? `Firebase terhubung (${st.project_id})`
    : "Penyimpanan lokal aktif";
  footerStatus.classList.toggle("connected", st.connected);

  $("#firebase-info-grid").innerHTML = `
    <div class="info-cell"><span>Mode penyimpanan</span><strong>${esc(st.mode)}</strong></div>
    <div class="info-cell"><span>Project ID</span><strong><code>${esc(st.project_id)}</code></strong></div>
    <div class="info-cell"><span>Endpoint Cloud Firestore</span><strong><code>${esc(st.firestore_endpoint)}</code></strong></div>
    <div class="info-cell"><span>Data tersinkron</span><strong>${st.synced_jobs} lowongan, ${st.synced_users} pengguna, ${st.synced_interactions} interaksi</strong></div>
  `;
}

function renderCatalogTable() {
  const q = ($("#inp-catalog-search").value || "").toLowerCase().trim();
  const rows = state.jobs.filter((j) => {
    if (!q) return true;
    return `${j.id} ${j.title} ${j.company} ${j.skills.join(" ")} ${j.source_dataset}`.toLowerCase().includes(q);
  });

  $("#catalog-table-wrap").innerHTML = `
    <table>
      <thead>
        <tr>
          <th>ID</th>
          <th>Posisi &amp; perusahaan</th>
          <th>Mode &amp; gaji</th>
          <th>Skor (C1 / C2 / C3 / C4)</th>
          <th>Sumber dataset</th>
        </tr>
      </thead>
      <tbody>
        ${rows
          .map(
            (j) => `
          <tr>
            <td><code>${j.id}</code></td>
            <td><strong>${esc(j.title)}</strong><br/><span class="muted-text">${esc(j.company)}</span></td>
            <td>${esc(j.work_mode)}, Rp ${j.salary_idr_millions.toFixed(1)} Jt</td>
            <td><code>${j.criteria.c1_role_clarity.toFixed(1)} / ${j.criteria.c2_technical_challenge.toFixed(1)} / ${j.criteria.c3_compensation.toFixed(1)} / ${j.criteria.c4_flexibility.toFixed(1)}</code></td>
            <td><code>${esc(j.source_dataset)}</code></td>
          </tr>`
          )
          .join("")}
      </tbody>
    </table>`;
}

// ---------------- BOOTSTRAP ----------------
async function bootstrap() {
  initTheme();
  initNavigation();

  try {
    const [usersRes, jobsRes] = await Promise.all([fetch("/api/users"), fetch("/api/jobs")]);
    state.users = await usersRes.json();
    state.jobs = await jobsRes.json();
  } catch (err) {
    $("#feed-error-text").textContent = "Server lokal belum siap. Pastikan karir-ai sedang berjalan.";
    $("#feed-error").classList.remove("hidden");
    return;
  }

  const selUser = $("#select-user");
  selUser.innerHTML = state.users
    .map((u) => `<option value="${u.id}">${esc(u.name)} (${u.experience_years} thn)</option>`)
    .join("");
  state.activeUserId = state.users[0]?.id || "USR-01";
  populateUserControls(state.users[0]);

  const cats = [...new Set(state.jobs.map((j) => j.category))].sort();
  $("#sel-category").innerHTML =
    `<option value="">Semua bidang (${state.jobs.length})</option>` +
    cats.map((c) => `<option value="${esc(c)}">${esc(c)}</option>`).join("");

  selUser.addEventListener("change", () => {
    state.activeUserId = selUser.value;
    const u = state.users.find((x) => x.id === state.activeUserId);
    populateUserControls(u);
    fetchRecommendations();
    if ($("#view-activity").classList.contains("active")) loadActivityView();
  });

  $("#search-form").addEventListener("submit", (e) => {
    e.preventDefault();
    fetchRecommendations();
  });

  let searchDebounce = null;
  $("#inp-query").addEventListener("input", () => {
    clearTimeout(searchDebounce);
    searchDebounce = setTimeout(fetchRecommendations, 180);
  });

  $("#sel-category").addEventListener("change", fetchRecommendations);
  $("#select-mode").addEventListener("change", fetchRecommendations);
  $("#chk-constraints").addEventListener("change", fetchRecommendations);

  $$("[data-quick-mode]").forEach((btn) => {
    btn.addEventListener("click", () => {
      syncQuickModeButtons(btn.getAttribute("data-quick-mode"));
      fetchRecommendations();
    });
  });

  ["#rng-w1", "#rng-w2", "#rng-w3", "#rng-w4"].forEach((id) => {
    $(id).addEventListener("input", () => {
      renderWeightBar();
      fetchRecommendations();
    });
  });

  ["#inp-min-salary", "#inp-max-exp", "#inp-min-c1", "#inp-min-c2"].forEach((id) => {
    $(id).addEventListener("change", fetchRecommendations);
  });

  $("#btn-retry-feed").addEventListener("click", fetchRecommendations);

  $("#btn-save-prefs").addEventListener("click", async () => {
    const u = state.users.find((x) => x.id === state.activeUserId);
    if (!u) return;
    const skills = $("#inp-skills")
      .value.split(",")
      .map((s) => s.trim().toLowerCase())
      .filter(Boolean);

    const res = await fetch(`/api/users/${state.activeUserId}/preferences`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        preferred_skills: skills,
        preferred_categories: u.preferred_categories,
        experience_years: u.experience_years,
        weights: readNormalizedWeights(),
        constraints: readConstraints(),
      }),
    });
    if (res.ok) {
      const updated = await res.json();
      const idx = state.users.findIndex((x) => x.id === updated.id);
      if (idx >= 0) state.users[idx] = updated;
      showToast("Preferensi berhasil disimpan.");
      await fetchRecommendations();
    } else {
      showToast("Gagal menyimpan preferensi.");
    }
  });

  $("#btn-reset-demo").addEventListener("click", async () => {
    await fetch("/api/reset", { method: "POST" });
    const usersRes = await fetch("/api/users");
    state.users = await usersRes.json();
    $("#inp-query").value = "";
    $("#sel-category").value = "";
    $("#chk-constraints").checked = true;
    const cur = state.users.find((x) => x.id === state.activeUserId) || state.users[0];
    populateUserControls(cur);
    await fetchRecommendations();
    await loadFirebaseStatus();
    showToast("Pengaturan dikembalikan ke kondisi awal.");
  });

  $("#btn-sync-firebase").addEventListener("click", async () => {
    const res = await fetch("/api/firebase/sync", { method: "POST" });
    if (res.ok) {
      await loadFirebaseStatus();
      showToast("Koleksi berhasil disinkronkan ke Cloud Firestore.");
    } else {
      showToast("Firebase Emulator (:8088) belum aktif.");
    }
  });

  // Modal events + keyboard Escape (R-32)
  $("#btn-close-modal").addEventListener("click", closeDetailModal);
  $("#detail-modal").addEventListener("click", (e) => {
    if (e.target.id === "detail-modal") closeDetailModal();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closeDetailModal();
  });
  $$(".modal-tab").forEach((btn) => {
    btn.addEventListener("click", () => setModalTab(btn.getAttribute("data-mtab")));
  });
  $("#btn-modal-bookmark").addEventListener("click", () => {
    if (state.activeModalRec) submitInteraction(state.activeModalRec.job.id, 4, "bookmarked");
  });
  $("#btn-modal-apply").addEventListener("click", () => {
    if (state.activeModalRec) submitInteraction(state.activeModalRec.job.id, 5, "applied");
  });

  $("#sim-w-clarity").addEventListener("input", loadLecturerSimulation);
  $("#sim-min-practice").addEventListener("input", loadLecturerSimulation);
  $("#inp-catalog-search").addEventListener("input", renderCatalogTable);

  await Promise.all([loadFirebaseStatus(), fetchRecommendations()]);
}

document.addEventListener("DOMContentLoaded", bootstrap);
