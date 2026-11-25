// KarirAI v1.3: Dark Luminous Dashboard + Master-Detail Job Portal (Rust + Google Firebase)

const state = {
  users: [],
  jobs: [],
  activeUserId: "USR-01",
  quickMode: "all",
  lastResponse: null,
  selectedJobId: null,
  detailTab: "overview",
  lastTriggerEl: null,
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
  clearTimeout(showToast._t);
  showToast._t = setTimeout(() => el.classList.add("hidden"), 2800);
}

// ---------------- TEMA GELAP (DEFAULT) / TERANG (R-21 & R-34) ----------------
function applyTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("karir_ai_theme_v3", theme);
  const dark = theme === "dark";
  const btn = $("#btn-theme");
  btn.setAttribute("aria-pressed", String(dark));
  btn.setAttribute("aria-label", dark ? "Ubah ke mode terang" : "Ubah ke mode gelap");
  $("#icon-sun").classList.toggle("hidden", !dark);
  $("#icon-moon").classList.toggle("hidden", dark);
}

function initTheme() {
  applyTheme(localStorage.getItem("karir_ai_theme_v3") || "dark");
  $("#btn-theme").addEventListener("click", () => {
    const cur = document.documentElement.getAttribute("data-theme");
    applyTheme(cur === "dark" ? "light" : "dark");
  });
}

// ---------------- NAVIGASI ----------------
function switchView(viewId) {
  closeMobileDetail();
  $$(".dock-link").forEach((btn) => {
    const on = btn.getAttribute("data-view") === viewId;
    btn.classList.toggle("active", on);
    if (on) btn.setAttribute("aria-current", "page");
    else btn.removeAttribute("aria-current");
  });
  $$(".view-section").forEach((sec) => {
    sec.classList.toggle("active", sec.id === viewId);
  });

  if (viewId === "view-activity") loadActivityView();
  if (viewId === "view-lab") loadLabView();
}

function initNavigation() {
  $$(".dock-link").forEach((btn) => {
    btn.addEventListener("click", () => switchView(btn.getAttribute("data-view")));
  });
  $(".brand").addEventListener("click", (e) => {
    e.preventDefault();
    switchView("view-jobs");
  });

  const prefsBtn = $("#btn-toggle-prefs");
  const prefsDrawer = $("#prefs-drawer");
  prefsBtn.addEventListener("click", () => {
    const nextOpen = prefsDrawer.classList.contains("hidden");
    prefsDrawer.classList.toggle("hidden", !nextOpen);
    prefsBtn.setAttribute("aria-expanded", String(nextOpen));
  });
}

// ---------------- BOBOT & SYARAT ----------------
function readWeights() {
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
  const w = readWeights();
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
  updateFilterSummaryText();
}

function readConstraints() {
  const user = state.users.find((u) => u.id === state.activeUserId);
  let allowedModes = user?.constraints?.allowed_work_modes || ["Remote", "Hybrid", "Onsite"];
  if (state.quickMode !== "all") allowedModes = [state.quickMode];
  return {
    min_salary_idr_millions: Number($("#inp-min-salary").value) || 0,
    max_required_experience_years: Number($("#inp-max-exp").value) || 0,
    allowed_work_modes: allowedModes,
    min_role_clarity: Number($("#inp-min-c1").value) || 0,
    min_technical_challenge: Number($("#inp-min-c2").value) || 0,
  };
}

function updateFilterSummaryText() {
  const active = $("#chk-constraints").checked;
  if (!active) {
    $("#active-filter-summary").textContent = "Filter syarat wajib nonaktif";
    return;
  }
  const sal = Number($("#inp-min-salary").value) || 0;
  const exp = Number($("#inp-max-exp").value) || 0;
  $("#active-filter-summary").textContent = `Min Rp ${sal} Jt, maks ${exp} thn`;
}

function syncQuickMode(mode) {
  state.quickMode = mode;
  $$("[data-quick-mode]").forEach((btn) => {
    btn.classList.toggle("active", btn.getAttribute("data-quick-mode") === mode);
  });
}

function populateUserControls(user) {
  if (!user) return;
  $("#active-profile-line").textContent =
    `Profil aktif: ${user.name} (${user.role_headline}, ${user.experience_years} thn)`;

  $("#rng-w1").value = Math.round(user.weights.c1_role_clarity * 100);
  $("#rng-w2").value = Math.round(user.weights.c2_technical_challenge * 100);
  $("#rng-w3").value = Math.round(user.weights.c3_compensation * 100);
  $("#rng-w4").value = Math.round(user.weights.c4_flexibility * 100);

  $("#inp-min-salary").value = user.constraints.min_salary_idr_millions;
  $("#inp-max-exp").value = user.constraints.max_required_experience_years;
  $("#inp-min-c1").value = user.constraints.min_role_clarity;
  $("#inp-min-c2").value = user.constraints.min_technical_challenge;
  $("#inp-skills").value = (user.preferred_skills || []).join(", ");

  const modes = user.constraints.allowed_work_modes || [];
  syncQuickMode(modes.length === 1 ? modes[0] : "all");
  renderWeightBar();
}

// ---------------- FETCH & RENDER MASTER-DETAIL ----------------
async function fetchRecommendations() {
  const errBox = $("#feed-error");
  const loadBox = $("#feed-loading");
  errBox.classList.add("hidden");
  loadBox.classList.remove("hidden");

  try {
    const res = await fetch("/api/recommend", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        user_id: state.activeUserId,
        mode: $("#select-mode").value,
        apply_hard_constraints: $("#chk-constraints").checked,
        top_k: 15,
        exclude_interacted: true,
        search_query: $("#inp-query").value.trim() || null,
        category_filter: $("#sel-category").value || null,
        override_weights: readWeights(),
        override_constraints: readConstraints(),
      }),
    });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = await res.json();
    state.lastResponse = data;

    const exists = data.recommendations.some((r) => r.job.id === state.selectedJobId);
    if (!exists) {
      state.selectedJobId = data.recommendations[0]?.job.id || null;
      state.detailTab = "overview";
    }
    renderBoard(data);
  } catch (err) {
    $("#feed-error-text").textContent = `Gagal memuat daftar lowongan: ${err.message}`;
    errBox.classList.remove("hidden");
  } finally {
    loadBox.classList.add("hidden");
  }
}

function renderBoard(data) {
  const list = data.recommendations;
  const q = $("#inp-query").value.trim();
  const msText = `${(data.execution_time_micros / 1000).toFixed(2)} ms`;
  $("#results-count").textContent = list.length
    ? `${list.length} lowongan cocok${q ? ` untuk "${q}"` : ""}`
    : `Tidak ada lowongan cocok${q ? ` untuk "${q}"` : ""}`;
  $("#latency-readout").textContent = msText;
  $("#stat-latency").textContent = msText.replace(".", ",");

  const listEl = $("#jobs-list");
  if (!list.length) {
    listEl.innerHTML = `
      <div class="empty-box">
        <h3>Belum ada lowongan sesuai kriteria ini</h3>
        <p>Coba ubah kata kunci pencarian atau longgarkan filter syarat wajib Anda.</p>
        <button type="button" id="btn-relax-filter" class="btn-outline btn-sm">Tampilkan semua kandidat</button>
      </div>`;
    $("#btn-relax-filter").addEventListener("click", () => {
      $("#inp-query").value = "";
      $("#sel-category").value = "";
      $("#chk-constraints").checked = false;
      syncQuickMode("all");
      updateFilterSummaryText();
      fetchRecommendations();
    });
    renderDetailPane(null);
  } else {
    listEl.innerHTML = list
      .map((rec) => {
        const j = rec.job;
        const isSel = j.id === state.selectedJobId;
        const matched = new Set(rec.content_based.matched_skills.map((s) => s.toLowerCase()));
        const skillsHtml = j.skills
          .slice(0, 3)
          .map((sk) => `<span class="skill-chip ${matched.has(sk.toLowerCase()) ? "hit" : ""}">${esc(sk)}</span>`)
          .join("");

        return `
        <article class="job-card ${isSel ? "selected" : ""}" role="listitem">
          <div class="job-card-top">
            <button type="button" class="job-select-btn" data-select-job="${j.id}">
              ${esc(j.title)}
            </button>
            <span class="match-badge tabular">${Math.round(rec.match_percentage)}% cocok</span>
          </div>
          <p class="job-card-sub">${esc(j.company)} &middot; ${esc(j.location)}</p>
          <div class="job-card-meta tabular">
            Rp ${j.salary_idr_millions.toFixed(1)} Jt/bln &middot; ${esc(j.work_mode)} &middot; Min ${j.min_experience_years} thn
          </div>
          <div class="job-card-foot">
            <div class="skills-inline">${skillsHtml}</div>
            <button
              type="button"
              class="bookmark-btn"
              data-quick-save="${j.id}"
              aria-label="Simpan lowongan ${esc(j.title)}"
              title="Simpan lowongan"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"></path>
              </svg>
            </button>
          </div>
        </article>`;
      })
      .join("");

    const selectedRec = list.find((r) => r.job.id === state.selectedJobId) || list[0];
    renderDetailPane(selectedRec);

    $$("[data-select-job]").forEach((btn) => {
      btn.addEventListener("click", () => {
        state.selectedJobId = btn.getAttribute("data-select-job");
        state.lastTriggerEl = btn;
        $$(".job-card").forEach((el) => el.classList.remove("selected"));
        btn.closest(".job-card").classList.add("selected");
        const rec = state.lastResponse.recommendations.find((r) => r.job.id === state.selectedJobId);
        renderDetailPane(rec);
        if (window.innerWidth <= 980) {
          $("#detail-pane").classList.add("mobile-open");
          $("#btn-close-mobile-detail").focus();
        }
      });
    });

    $$("[data-quick-save]").forEach((btn) => {
      btn.addEventListener("click", (e) => {
        e.stopPropagation();
        submitInteraction(btn.getAttribute("data-quick-save"), 4, "bookmarked");
      });
    });
  }

  const drawer = $("#filtered-drawer");
  if (data.apply_hard_constraints && data.filtered_out_count > 0) {
    drawer.classList.remove("hidden");
    $("#filtered-summary-label").textContent =
      `${data.filtered_out_count} lowongan tidak memenuhi syarat wajib Anda`;
    $("#filtered-out-list").innerHTML = data.filtered_out_examples
      .map(
        (rec) => `
      <div class="filtered-item">
        <strong>${esc(rec.job.title)} (${esc(rec.job.company)}, Rp ${rec.job.salary_idr_millions.toFixed(1)} Jt)</strong>
        <span>${esc(rec.constraints.violations.join("; "))}</span>
      </div>`
      )
      .join("");
  } else {
    drawer.classList.add("hidden");
  }
}

function closeMobileDetail() {
  const pane = $("#detail-pane");
  if (!pane.classList.contains("mobile-open")) return;
  pane.classList.remove("mobile-open");
  if (state.lastTriggerEl && typeof state.lastTriggerEl.focus === "function") {
    state.lastTriggerEl.focus();
  }
}

function renderDetailPane(rec) {
  const body = $("#detail-pane-body");
  if (!rec) {
    body.innerHTML = `
      <div class="empty-box" style="border:none;padding:48px 16px;">
        <h3>Pilih lowongan untuk melihat detail</h3>
        <p>Ringkasan pekerjaan, syarat kualifikasi, dan bukti perhitungan kecocokan AI akan tampil di sini.</p>
      </div>`;
    return;
  }

  const j = rec.job;
  const cb = rec.content_based;
  const cf = rec.collaborative;
  const mc = rec.multi_criteria;
  const matched = new Set(cb.matched_skills.map((s) => s.toLowerCase()));

  const skillsMarkup = j.skills
    .map((sk) => `<span class="skill-chip ${matched.has(sk.toLowerCase()) ? "hit" : ""}">${esc(sk)}</span>`)
    .join("");

  const overviewHtml = `
    <div class="reason-box">
      <strong>Alasan kecocokan:</strong> ${esc(rec.human_explanation)}.
    </div>

    <div class="detail-sec">
      <h3>Deskripsi peran</h3>
      <p>${esc(j.description)}</p>
    </div>

    <div class="detail-sec">
      <h3>Keahlian (${cb.matched_skills.length} dari ${j.skills.length} cocok dengan profil Anda)</h3>
      <div class="skills-inline">${skillsMarkup}</div>
    </div>

    <div class="detail-sec">
      <h3>Penilaian 4 aspek lowongan (skala 1-5)</h3>
      <div class="aspect-meters">
        ${meterItem("Kejelasan peran", j.criteria.c1_role_clarity)}
        ${meterItem("Tantangan teknis", j.criteria.c2_technical_challenge)}
        ${meterItem("Kompensasi gaji", j.criteria.c3_compensation)}
        ${meterItem("Fleksibilitas kerja", j.criteria.c4_flexibility)}
      </div>
    </div>

    <div class="detail-sec">
      <h3>Kesesuaian syarat wajib</h3>
      <ul class="check-list">
        ${rec.constraints.passed_checks.map((c) => `<li>${esc(c)}</li>`).join("")}
      </ul>
    </div>
  `;

  const termRows = cb.top_matched_terms.length
    ? cb.top_matched_terms
        .map(
          (t) => `
      <tr>
        <td><code>${esc(t.term)}</code></td>
        <td class="tabular">${t.tfidf_user.toFixed(4)}</td>
        <td class="tabular">${t.tfidf_job.toFixed(4)}</td>
        <td class="tabular"><strong>${t.product.toFixed(4)}</strong></td>
      </tr>`
        )
        .join("")
    : `<tr><td colspan="4">Tidak ada irisan token kata kunci langsung.</td></tr>`;

  const cfRows = cf.neighbors_used.length
    ? cf.neighbors_used
        .map(
          (n) => `
      <tr>
        <td>${esc(n.neighbor_name)} (${n.neighbor_id})</td>
        <td class="tabular"><strong>${n.similarity.toFixed(4)}</strong></td>
        <td class="tabular">${n.shared_jobs_count} item</td>
        <td class="tabular">${n.neighbor_rating_on_job.toFixed(1)} / 5.0</td>
      </tr>`
        )
        .join("")
    : `<tr><td colspan="4">Belum ada tetangga serupa pada lowongan ini (otomatis memakai redistribusi Cold-Start).</td></tr>`;

  const mathHtml = `
    <div class="detail-sec" style="margin-top:0;padding-top:0;border-top:none;">
      <h3>1. Content-Based Filtering (TF-IDF &amp; Cosine Similarity)</h3>
      <div class="formula-box">Cosine(u, j) = ${cb.dot_product.toFixed(4)} / (${cb.user_norm.toFixed(4)} * ${cb.job_norm.toFixed(4)}) = ${cb.tfidf_cosine.toFixed(4)} | S_CB = ${cb.final_cb_score.toFixed(4)}</div>
      <div class="table-wrap">
        <table>
          <thead><tr><th>Kata kunci</th><th>TF-IDF Profil</th><th>TF-IDF Lowongan</th><th>Kontribusi</th></tr></thead>
          <tbody>${termRows}</tbody>
        </table>
      </div>
    </div>

    <div class="detail-sec">
      <h3>2. Collaborative Filtering (User-Based Pearson)</h3>
      <div class="formula-box">Rata-rata user = ${cf.target_user_mean.toFixed(2)} | Prediksi rating = ${cf.predicted_rating_1_to_5.toFixed(2)} / 5.00 | S_CF = ${cf.normalized_cf_score.toFixed(4)}</div>
      <div class="table-wrap">
        <table>
          <thead><tr><th>Tetangga serupa</th><th>Pearson</th><th>Irisan</th><th>Rating</th></tr></thead>
          <tbody>${cfRows}</tbody>
        </table>
      </div>
    </div>

    <div class="detail-sec">
      <h3>3. Skor Multikriteria (4 Aspek Berbobot)</h3>
      <div class="formula-box">${esc(mc.formula_trace)}</div>
    </div>

    <div class="detail-sec">
      <h3>4. Skor Akhir Hybrid</h3>
      <div class="formula-box">S_Final = (${rec.alpha_cb_weight.toFixed(4)} * ${cb.final_cb_score.toFixed(4)}) + (${rec.beta_cf_weight.toFixed(4)} * ${cf.normalized_cf_score.toFixed(4)}) + (${rec.gamma_mc_weight.toFixed(4)} * ${mc.normalized_mc_score.toFixed(4)}) = ${rec.final_score.toFixed(4)} (${rec.match_percentage.toFixed(1)}%)</div>
    </div>
  `;

  body.innerHTML = `
    <div class="detail-top">
      <div class="detail-id">
        <div class="monogram" aria-hidden="true">${esc(initials(j.company))}</div>
        <div>
          <h2 class="detail-title">${esc(j.title)}</h2>
          <p class="detail-company">${esc(j.company)} &middot; ${esc(j.location)}</p>
        </div>
      </div>
      <span class="match-badge tabular">${Math.round(rec.match_percentage)}% cocok</span>
    </div>

    <div class="facts-bar tabular">
      <div><strong>Rp ${j.salary_idr_millions.toFixed(1)} Jt/bln</strong></div>
      <div><span>Mode:</span> <strong>${esc(j.work_mode)}</strong></div>
      <div><span>Pengalaman:</span> <strong>Min ${j.min_experience_years} thn</strong></div>
      <div><span>Bidang:</span> <strong>${esc(j.category)}</strong></div>
    </div>

    <div class="detail-actions">
      <div class="action-btns">
        <button type="button" id="btn-detail-apply" class="btn-primary">Lamar lowongan</button>
        <button type="button" id="btn-detail-save" class="btn-outline">Simpan</button>
      </div>

      <div class="segment-control" role="tablist" aria-label="Tampilan detail lowongan">
        <button type="button" class="seg-item ${state.detailTab === "overview" ? "active" : ""}" role="tab" aria-selected="${state.detailTab === "overview"}" data-dtab="overview">Ringkasan</button>
        <button type="button" class="seg-item ${state.detailTab === "math" ? "active" : ""}" role="tab" aria-selected="${state.detailTab === "math"}" data-dtab="math">Hitungan AI</button>
      </div>
    </div>

    ${state.detailTab === "overview" ? overviewHtml : mathHtml}
  `;

  $("#btn-detail-apply").addEventListener("click", () => submitInteraction(j.id, 5, "applied"));
  $("#btn-detail-save").addEventListener("click", () => submitInteraction(j.id, 4, "bookmarked"));
  $$("[data-dtab]").forEach((btn) => {
    btn.addEventListener("click", () => {
      state.detailTab = btn.getAttribute("data-dtab");
      renderDetailPane(rec);
    });
  });
}

function meterItem(label, val) {
  const pct = Math.min(100, Math.max(0, (val / 5) * 100));
  return `
    <div class="meter-item">
      <div class="meter-head">
        <span>${label}</span>
        <strong class="tabular">${val.toFixed(1)} / 5</strong>
      </div>
      <div class="meter-track"><div class="meter-fill" style="width:${pct}%"></div></div>
    </div>`;
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
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    showToast(action === "applied" ? "Lamaran tercatat ke Firebase." : "Lowongan disimpan.");
    closeMobileDetail();
    await fetchRecommendations();
    await loadFirebaseStatus();
  } catch (err) {
    showToast(`Gagal menyimpan: ${err.message}`);
  }
}

// ---------------- VIEW 2: AKTIVITAS ----------------
async function loadActivityView() {
  const [histRes, cfRes] = await Promise.all([
    fetch(`/api/users/${state.activeUserId}/interactions`),
    fetch(`/api/cf-matrix/${state.activeUserId}`),
  ]);
  const history = await histRes.json();
  const neighbors = await cfRes.json();

  $("#history-count").textContent = `${history.length} item`;
  if (!history.length) {
    $("#user-history-wrap").innerHTML = `<p class="meta-muted">Belum ada lowongan yang dilamar atau disimpan.</p>`;
  } else {
    $("#user-history-wrap").innerHTML = `
      <div class="table-wrap">
        <table>
          <thead><tr><th>Lowongan</th><th>Status</th><th>Rating</th></tr></thead>
          <tbody>
            ${history
              .slice()
              .reverse()
              .map((h) => {
                const a = h.interaction.action;
                const label = a === "applied" ? "Dilamar" : a === "bookmarked" ? "Disimpan" : "Dilihat";
                return `
                <tr>
                  <td><strong>${esc(h.job_title)}</strong><br/><span class="meta-muted">${esc(h.company)} (${esc(h.work_mode)})</span></td>
                  <td><span class="state-pill ${esc(a)}">${label}</span></td>
                  <td class="tabular"><strong>${h.interaction.rating.toFixed(1)}</strong> / 5</td>
                </tr>`;
              })
              .join("")}
          </tbody>
        </table>
      </div>`;
  }

  $("#cf-neighbors-wrap").innerHTML = `
    <div class="table-wrap">
      <table>
        <thead><tr><th>Pengguna</th><th>Irisan</th><th>Pearson</th></tr></thead>
        <tbody>
          ${neighbors
            .map(
              (n, i) => `
            <tr class="${i < 3 && n.similarity > 0.1 ? "row-best" : ""}">
              <td><strong>${esc(n.neighbor_name)}</strong><br/><span class="meta-muted">${esc(n.role_headline)}</span></td>
              <td class="tabular">${n.shared_jobs_count} item</td>
              <td class="tabular"><strong>${n.similarity.toFixed(4)}</strong></td>
            </tr>`
            )
            .join("")}
        </tbody>
      </table>
    </div>`;
}

// ---------------- VIEW 3: EVALUASI AI ----------------
async function loadLabView() {
  await Promise.all([loadEvaluation(), loadLecturerSimulation(), loadFirebaseStatus()]);
  renderCatalogTable();
}

async function loadEvaluation() {
  const res = await fetch("/api/evaluate");
  const rep = await res.json();

  const best = rep.methods.find((m) => m.method_id === "proposed_hmc_cf");
  if (best) {
    $("#stat-p3").textContent = `${(best.precision_at_3 * 100).toFixed(1).replace(".", ",")}%`;
  }

  $("#eval-meta").textContent = `Indeks dibangun ${(rep.index_build_micros / 1000).toFixed(2)} ms`;
  $("#eval-desc").textContent =
    `Diuji pada ${rep.evaluated_users_count} profil, ${rep.catalog_jobs_count} lowongan riil, dan ${rep.total_interactions_count} interaksi (${rep.train_interactions_count} train / ${rep.test_interactions_count} test hold-out).`;

  $("#eval-bars").innerHTML = rep.methods
    .map(
      (m) => `
    <div class="metric-bar-row">
      <div class="metric-bar-head">
        <span>${esc(m.method_label)}</span>
        <span class="meta-muted tabular">${m.avg_latency_micros} us (${(m.avg_latency_micros / 1000).toFixed(2)} ms)</span>
      </div>
      <div class="metric-bar-cols tabular">
        <div>
          Precision@3: <strong>${(m.precision_at_3 * 100).toFixed(1)}%</strong>
          <div class="meter-track"><div class="meter-fill" style="width:${(m.precision_at_3 * 100).toFixed(1)}%"></div></div>
        </div>
        <div>
          NDCG@5: <strong>${(m.ndcg_at_5 * 100).toFixed(1)}%</strong>
          <div class="meter-track"><div class="meter-fill" style="width:${(m.ndcg_at_5 * 100).toFixed(1)}%"></div></div>
        </div>
        <div>
          Syarat wajib: <strong>${(m.constraint_satisfaction_rate * 100).toFixed(1)}%</strong>
          <div class="meter-track"><div class="meter-fill ok" style="width:${(m.constraint_satisfaction_rate * 100).toFixed(1)}%"></div></div>
        </div>
      </div>
    </div>`
    )
    .join("");

  $("#eval-table-wrap").innerHTML = `
    <table>
      <thead>
        <tr><th>Metode</th><th>P@3</th><th>P@5</th><th>R@5</th><th>F1@5</th><th>NDCG@5</th><th>Syarat</th><th>Latensi</th></tr>
      </thead>
      <tbody class="tabular">
        ${rep.methods
          .map(
            (m) => `
          <tr class="${m.method_id === "proposed_hmc_cf" ? "row-best" : ""}">
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
    <div class="mini-case">
      <h4>Rata-rata identik (4.0)</h4>
      <p><strong>Lowongan A:</strong> Kejelasan ${d.item_a_clarity}, Tantangan ${d.item_a_practice}</p>
      <p><strong>Lowongan B:</strong> Kejelasan ${d.item_b_clarity}, Tantangan ${d.item_b_practice}</p>
    </div>
    <div class="mini-case">
      <h4>Prioritas Dina (70% Kejelasan)</h4>
      <p>Skor A = <strong>${d.dina_70_30_score_a.toFixed(1)}</strong>, Skor B = ${d.dina_70_30_score_b.toFixed(1)}</p>
      <p>Terpilih: <strong>${esc(d.dina_winner)}</strong></p>
    </div>
    <div class="mini-case">
      <h4>Prioritas Bima (70% Tantangan)</h4>
      <p>Skor A = ${d.bima_30_70_score_a.toFixed(1)}, Skor B = <strong>${d.bima_30_70_score_b.toFixed(1)}</strong></p>
      <p>Terpilih: <strong>${esc(d.bima_winner)}</strong></p>
    </div>
  `;

  $("#sim-result").innerHTML = `
    <p style="margin:0 0 6px;">
      <strong>Lowongan A:</strong> ${d.pair_exercise_50_50_score_a.toFixed(2)} (${d.item_a_passes_requirement ? "Lolos" : "Gugur"}) &middot;
      <strong>Lowongan B:</strong> ${d.pair_exercise_50_50_score_b.toFixed(2)} (${d.item_b_passes_requirement ? "Lolos" : "Gugur"})
    </p>
    <p style="margin:0;"><strong>Keputusan: ${esc(d.pair_exercise_final_choice)}.</strong> ${esc(d.pair_exercise_explanation)}</p>
  `;
}

async function loadFirebaseStatus() {
  const res = await fetch("/api/firebase/status");
  const st = await res.json();
  const fs = $("#footer-fb-status");
  fs.textContent = st.connected ? `Firebase aktif (${st.project_id})` : "Penyimpanan lokal aktif";
  fs.classList.toggle("connected", st.connected);

  $("#firebase-info-grid").innerHTML = `
    <div class="kv-cell"><span>Mode basis data</span><strong>${esc(st.mode)}</strong></div>
    <div class="kv-cell"><span>Project ID</span><strong><code>${esc(st.project_id)}</code></strong></div>
    <div class="kv-cell"><span>Endpoint Firestore</span><strong><code>${esc(st.firestore_endpoint)}</code></strong></div>
    <div class="kv-cell"><span>Koleksi tersinkron</span><strong>${st.synced_jobs} lowongan, ${st.synced_users} pengguna, ${st.synced_interactions} interaksi</strong></div>
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
        <tr><th>ID</th><th>Lowongan</th><th>Mode &amp; gaji</th><th>Aspek (C1/C2/C3/C4)</th><th>Dataset</th></tr>
      </thead>
      <tbody>
        ${rows
          .map(
            (j) => `
          <tr>
            <td><code>${j.id}</code></td>
            <td><strong>${esc(j.title)}</strong><br/><span class="meta-muted">${esc(j.company)}</span></td>
            <td class="tabular">${esc(j.work_mode)}, Rp ${j.salary_idr_millions.toFixed(1)} Jt</td>
            <td class="tabular"><code>${j.criteria.c1_role_clarity.toFixed(1)} / ${j.criteria.c2_technical_challenge.toFixed(1)} / ${j.criteria.c3_compensation.toFixed(1)} / ${j.criteria.c4_flexibility.toFixed(1)}</code></td>
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
    $("#feed-error-text").textContent = "Server tidak dapat dijangkau.";
    $("#feed-error").classList.remove("hidden");
    return;
  }

  $("#stat-jobs-count").textContent = String(state.jobs.length);

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
    state.selectedJobId = null;
    populateUserControls(state.users.find((x) => x.id === state.activeUserId));
    fetchRecommendations();
    if ($("#view-activity").classList.contains("active")) loadActivityView();
  });

  $("#search-form").addEventListener("submit", (e) => {
    e.preventDefault();
    fetchRecommendations();
  });

  let debounce = null;
  $("#inp-query").addEventListener("input", () => {
    clearTimeout(debounce);
    debounce = setTimeout(fetchRecommendations, 180);
  });

  $("#sel-category").addEventListener("change", fetchRecommendations);
  $("#select-mode").addEventListener("change", fetchRecommendations);
  $("#chk-constraints").addEventListener("change", () => {
    updateFilterSummaryText();
    fetchRecommendations();
  });

  $$("[data-quick-mode]").forEach((btn) => {
    btn.addEventListener("click", () => {
      syncQuickMode(btn.getAttribute("data-quick-mode"));
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
    $(id).addEventListener("change", () => {
      updateFilterSummaryText();
      fetchRecommendations();
    });
  });

  $("#btn-retry-feed").addEventListener("click", fetchRecommendations);
  $("#btn-close-mobile-detail").addEventListener("click", closeMobileDetail);
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closeMobileDetail();
  });

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
        weights: readWeights(),
        constraints: readConstraints(),
      }),
    });
    if (res.ok) {
      const updated = await res.json();
      const idx = state.users.findIndex((x) => x.id === updated.id);
      if (idx >= 0) state.users[idx] = updated;
      showToast("Preferensi disimpan.");
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
    state.selectedJobId = null;
    populateUserControls(state.users.find((x) => x.id === state.activeUserId) || state.users[0]);
    await fetchRecommendations();
    await loadFirebaseStatus();
    showToast("Pengaturan dikembalikan ke kondisi awal.");
  });

  $("#btn-sync-firebase").addEventListener("click", async () => {
    const res = await fetch("/api/firebase/sync", { method: "POST" });
    if (res.ok) {
      await loadFirebaseStatus();
      showToast("Koleksi disinkronkan ke Cloud Firestore.");
    } else {
      showToast("Firebase Emulator (:8088) belum aktif.");
    }
  });

  $("#sim-w-clarity").addEventListener("input", loadLecturerSimulation);
  $("#sim-min-practice").addEventListener("input", loadLecturerSimulation);
  $("#inp-catalog-search").addEventListener("input", renderCatalogTable);

  await Promise.all([loadFirebaseStatus(), fetchRecommendations(), loadEvaluation()]);
}

document.addEventListener("DOMContentLoaded", bootstrap);
