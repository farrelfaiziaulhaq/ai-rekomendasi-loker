# Panduan Menjalankan Proyek di Windows 10/11 & Integrasi Firebase

Karena mayoritas anggota kelompok menggunakan **Windows 10 / Windows 11**, proyek ini dirancang dengan **2 cara menjalankan yang sangat simpel** (termasuk opsi **1-Klik Tanpa Install Rust sama sekali!**).

---

## Cara 1 (Paling Direkomendasikan untuk Anggota Kelompok): Pakai Paket ZIP GitHub Release (0 Instalasi!)

Anda **tidak perlu** menginstal Rust, Visual Studio C++ Build Tools, Python, ataupun menunggu kompilasi kode:

1. Buka halaman **[GitHub Releases Proyek Ini](https://github.com/farrelfaiziaulhaq/ai-rekomendasi-loker/releases/latest)**.
2. Unduh file **`karir-ai-v1.0.0-windows-x86_64.zip`**.
3. Klik kanan file `.zip` tersebut → pilih **Extract All...**
4. Buka folder hasil ekstrak, lalu **klik dua kali `start.bat`**:
   - Server `karir-ai.exe` langsung menyala dalam `< 1 detik`!
   - Browser otomatis membuka **`http://127.0.0.1:3000`**.
5. File peluncur tambahan di dalam paket ZIP Windows:
   - **`start.bat`** : Mode instan siap pakai (data tersimpan otomatis di `data/runtime_firestore_mirror.json`, berjalan 100% lancar bahkan saat di ruang kelas tanpa internet!).
   - **`start-firebase-emulator.bat`** : Menyalakan Firebase Auth (`:9098`), Cloud Firestore (`:8088`), dan Dashboard Firebase Emulator UI (`http://127.0.0.1:4040`) lalu menyinkronkan seluruh data secara otomatis.
   - **`cek-evaluasi-ai.bat`** : Menampilkan hasil pembuktian studi kasus PDF dosen & tabel metrik ilmiah `Precision@3`, `Precision@5`, `Recall@5`, `F1@5`, dan `NDCG@5` langsung di jendela Command Prompt.

---

## Cara 2: Menjalankan & Mengedit dari Source Code di Windows

Jika Anda ingin memodifikasi kode Rust atau tampilan web:

1. Clone repositori:
   ```powershell
   git clone https://github.com/farrelfaiziaulhaq/ai-rekomendasi-loker.git
   cd ai-rekomendasi-loker
   ```
2. Jika laptop belum terpasang Rust, cukup **klik dua kali `setup.bat`** (otomatis memasang Rust via `winget`).
3. Untuk menjalankan aplikasi, **klik dua kali `run.bat`** di File Explorer (atau ketik `.\run.ps1` di PowerShell).
4. Perintah PowerShell opsional lainnya:
   ```powershell
   .\run.ps1                 # Jalankan web server di http://127.0.0.1:3000
   .\run.ps1 -Emulator       # Jalankan bersama Firebase Emulator Suite (:4040)
   .\run.ps1 -Eval           # Cetak tabel evaluasi Precision@K & NDCG@K di terminal
   .\run.ps1 -CaseStudy      # Cetak verifikasi matematis Slide 09-17 PDF Dosen
   .\run.ps1 -Reset          # Kembalikan data ke kondisi awal dataset
   ```

---

## Cara Menghubungkan ke Google Firebase Cloud Produksi (Opsional)

Secara bawaan, aplikasi mendukung **3 mode basis data otomatis** (`src/firebase.rs`):
1. **Local Firestore Mirror (Default Zero-Setup)**: Langsung jalan tanpa akun/internet.
2. **Firebase Local Emulator Suite (`run-emulator.bat` / `./run.sh --emulator`)**: Menjalankan server Cloud Firestore & Auth asli secara lokal lengkap dengan dashboard visual di `http://127.0.0.1:4040`.
3. **Google Firebase Cloud Production**:
   - Buat proyek gratis di https://console.firebase.google.com dan aktifkan **Cloud Firestore** + **Authentication (Email/Password)**.
   - Salin `.env.example` menjadi `.env`, lalu isi `FIREBASE_PROJECT_ID` dan `FIREBASE_API_KEY` dari Firebase Console Anda.
   - Jalankan ulang `karir-ai` dan klik tombol **Sinkronkan Koleksi ke Cloud Firestore Sekarang** pada Tab 5 aplikasi.

---

## Cara Membuat Rilis Baru Otomatis (CI/CD GitHub Actions)

Setiap kali ada perubahan kode yang di-*push* ke GitHub:
1. Workflow **CI (`.github/workflows/ci.yml`)** otomatis menguji kompilasi, *unit test* algoritma AI, *clippy lint*, dan evaluasi metrik baik di runner **Ubuntu Linux** maupun **Windows Server**.
2. Untuk menerbitkan versi rilis baru (`.zip` Windows & `.tar.gz` Linux secara otomatis tanpa build manual), cukup jalankan:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```
   atau buka tab **Actions → Release → Run workflow** langsung dari halaman web GitHub!
