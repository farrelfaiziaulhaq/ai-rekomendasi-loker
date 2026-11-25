//! Filter Syarat Wajib (Hard Constraint Filtering — Slide 14, 16, & 17 `AI_Sistem_Rekomendasi_Visual.pdf`)
//! "Kadang ada syarat yang wajib dipenuhi — Skor total tinggi belum cukup jika item tidak memenuhi syarat pengguna."

use crate::models::{ConstraintCheckResult, HardConstraints, JobPosting};

/// Memeriksa apakah sebuah lowongan kerja lolos seluruh syarat mutlak pengguna.
pub fn check_hard_constraints(
    constraints: &HardConstraints,
    job: &JobPosting,
) -> ConstraintCheckResult {
    let mut violations = Vec::new();
    let mut passed_checks = Vec::new();

    // 1. Syarat Gaji Minimal
    if job.salary_idr_millions + 1e-6 >= constraints.min_salary_idr_millions {
        passed_checks.push(format!(
            "Gaji Rp {:.1} Jt >= minimal Rp {:.1} Jt",
            job.salary_idr_millions, constraints.min_salary_idr_millions
        ));
    } else {
        violations.push(format!(
            "Gaji Rp {:.1} Jt di bawah batas minimal Rp {:.1} Jt",
            job.salary_idr_millions, constraints.min_salary_idr_millions
        ));
    }

    // 2. Syarat Pengalaman Maksimal yang Diminta Lowongan
    if job.min_experience_years <= constraints.max_required_experience_years {
        passed_checks.push(format!(
            "Syarat pengalaman {} thn <= batas profil {} thn",
            job.min_experience_years, constraints.max_required_experience_years
        ));
    } else {
        violations.push(format!(
            "Membutuhkan minimal {} thn pengalaman (batas Anda maks {} thn)",
            job.min_experience_years, constraints.max_required_experience_years
        ));
    }

    // 3. Syarat Mode Kerja (Remote / Hybrid / Onsite)
    if constraints.allowed_work_modes.is_empty()
        || constraints
            .allowed_work_modes
            .iter()
            .any(|m| m.eq_ignore_ascii_case(&job.work_mode))
    {
        passed_checks.push(format!("Mode kerja {} sesuai preferensi", job.work_mode));
    } else {
        violations.push(format!(
            "Mode kerja {} tidak termasuk syarat wajib ({})",
            job.work_mode,
            constraints.allowed_work_modes.join("/")
        ));
    }

    // 4. Syarat Minimal Skor Kejelasan Peran (C1)
    if job.criteria.c1_role_clarity + 1e-6 >= constraints.min_role_clarity {
        if constraints.min_role_clarity > 0.0 {
            passed_checks.push(format!(
                "Aspek Kejelasan {:.1} >= minimal {:.1}",
                job.criteria.c1_role_clarity, constraints.min_role_clarity
            ));
        }
    } else {
        violations.push(format!(
            "Aspek Kejelasan {:.1} < syarat minimal {:.1}",
            job.criteria.c1_role_clarity, constraints.min_role_clarity
        ));
    }

    // 5. Syarat Minimal Skor Tantangan Teknis / Latihan (C2 — paralel langsung Slide 14 & 16!)
    if job.criteria.c2_technical_challenge + 1e-6 >= constraints.min_technical_challenge {
        if constraints.min_technical_challenge > 0.0 {
            passed_checks.push(format!(
                "Aspek Tantangan Teknis {:.1} >= minimal {:.1}",
                job.criteria.c2_technical_challenge, constraints.min_technical_challenge
            ));
        }
    } else {
        violations.push(format!(
            "Aspek Tantangan Teknis {:.1} < syarat minimal {:.1}",
            job.criteria.c2_technical_challenge, constraints.min_technical_challenge
        ));
    }

    ConstraintCheckResult {
        passed: violations.is_empty(),
        violations,
        passed_checks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::JobCriteria;

    #[test]
    fn filter_syarat_wajib_menggugurkan_item_tidak_memenuhi_syarat() {
        let job = JobPosting {
            id: "JOB-TEST".into(),
            source_dataset: "test".into(),
            source_job_id: "1".into(),
            title: "Senior Architect".into(),
            company: "Corp".into(),
            category: "Backend".into(),
            level: "Senior".into(),
            min_experience_years: 5,
            work_mode: "Onsite".into(),
            location: "Jakarta".into(),
            salary_idr_millions: 12.0,
            salary_usd_annual_ref: 45000,
            skills: vec!["rust".into()],
            description: "Desc".into(),
            criteria: JobCriteria {
                c1_role_clarity: 4.5,
                c2_technical_challenge: 3.0,
                c3_compensation: 4.0,
                c4_flexibility: 3.0,
            },
            overall_avg_rating: 3.62,
        };
        let cons = HardConstraints {
            min_salary_idr_millions: 15.0,
            max_required_experience_years: 2,
            allowed_work_modes: vec!["Remote".into()],
            min_role_clarity: 4.0,
            min_technical_challenge: 4.0,
        };
        let res = check_hard_constraints(&cons, &job);
        assert!(!res.passed);
        assert_eq!(res.violations.len(), 4);
    }
}
