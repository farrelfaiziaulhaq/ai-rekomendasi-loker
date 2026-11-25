#!/usr/bin/env python3
"""
Membangun Karya Ilmiah ke dalam dua format siap kumpul & siap edit:
  1. paper/KARYA_ILMIAH_SISTEM_REKOMENDASI_LOKER.pdf  (A4 Jurnal Akademik via Headless Chromium)
  2. paper/KARYA_ILMIAH_SISTEM_REKOMENDASI_LOKER.docx (Microsoft Word siap edit di Windows)
"""

import base64
import re
from pathlib import Path
import markdown
from docx import Document
from docx.enum.table import WD_TABLE_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.shared import Inches, Pt, RGBColor
from playwright.sync_api import sync_playwright

PAPER_DIR = Path("/home/farrel/ai-rekomendasi-loker/paper")
MD_PATH = PAPER_DIR / "KARYA_ILMIAH_SISTEM_REKOMENDASI_LOKER.md"
PDF_PATH = PAPER_DIR / "KARYA_ILMIAH_SISTEM_REKOMENDASI_LOKER.pdf"
DOCX_PATH = PAPER_DIR / "KARYA_ILMIAH_SISTEM_REKOMENDASI_LOKER.docx"


def img_to_data_uri(rel_path: str) -> str:
    p = (PAPER_DIR / rel_path).resolve()
    b64 = base64.b64encode(p.read_bytes()).decode("ascii")
    return f"data:image/png;base64,{b64}"


def format_display_equations(md_text: str) -> str:
    # Ubah blok $$...$$ menjadi kotak persamaan matematis HTML yang bersih & mudah dibaca
    def repl(match):
        eq = match.group(1).strip()
        # Konversi simbol LaTeX utama ke notasi tipografi Unicode yang tajam di Chromium font
        replacements = [
            (r"\left[", "["),
            (r"\right]", "]"),
            (r"\left(", "("),
            (r"\right)", ")"),
            (r"\text{TF-IDF}", "TF-IDF"),
            (r"\text{Sim}", "Sim"),
            (r"\text{Kat}", "Kat"),
            (r"\text{Final}", "Final"),
            (r"\text{eksplisit}", "eksplisit"),
            (r"\vec{u}", "u⃗"),
            (r"\vec{j}", "j⃗"),
            (r"\bar{r}", "r̄"),
            (r"\hat{r}", "r̂"),
            (r"\quad", "    ;    "),
            (r"\cap", "∩"),
            (r"\cup", "∪"),
            (r"\sum_", "∑_"),
            (r"\sum", "∑"),
            (r"\sqrt", "√"),
            (r"\times", "×"),
            (r"\cdot", "·"),
            (r"\ge", "≥"),
            (r"\le", "≤"),
            (r"\in", "∈"),
            (r"\alpha", "α"),
            (r"\beta", "β"),
            (r"\gamma", "γ"),
            (r"\dots", "…"),
            (r"\ln", "ln"),
            (r"\min", "min"),
            (r"\max", "max"),
            (r"\mathbb{I}", "𝕀"),
            (r"\|", "‖"),
            (r"\ ", " "),
        ]
        # Sederhanakan \frac{A}{B} dengan parser kurung kurawal berimbang (aman untuk kurung bersarang!)
        def parse_group(s, pos):
            if pos >= len(s) or s[pos] != "{":
                return "", pos
            depth = 0
            start = pos + 1
            for k in range(pos, len(s)):
                if s[k] == "{":
                    depth += 1
                elif s[k] == "}":
                    depth -= 1
                    if depth == 0:
                        return s[start:k], k + 1
            return s[start:], len(s)

        for _ in range(12):
            idx = eq.find(r"\frac{")
            if idx < 0:
                break
            num, next_p = parse_group(eq, idx + 5)
            while next_p < len(eq) and eq[next_p] == " ":
                next_p += 1
            den, end_p = parse_group(eq, next_p)
            eq = eq[:idx] + f"({num}) / ({den})" + eq[end_p:]
        for old, new in replacements:
            eq = eq.replace(old, new)
        eq = eq.replace("{", "").replace("}", "")
        return f'<div class="eq-box">{eq}</div>'

    text = re.sub(r"\$\$(.*?)\$\$", repl, md_text, flags=re.DOTALL)

    def inline_repl(m):
        s = m.group(1)
        for old, new in [
            (r"\vec{u}", "u⃗"), (r"\vec{j}", "j⃗"), (r"\bar{r}", "r̄"), (r"\hat{r}", "r̂"),
            (r"\ge", "≥"), (r"\le", "≤"), (r"\in", "∈"), (r"\alpha", "α"),
            (r"\beta", "β"), (r"\gamma", "γ"), (r"\sum", "∑"), (r"\dots", "…"),
            (r"\ ", " "),
        ]:
            s = s.replace(old, new)
        s = s.replace("{", "").replace("}", "")
        return f'<span class="math-inline">{s}</span>'

    text = re.sub(r"\$([^$\n]+)\$", inline_repl, text)
    return text


def build_pdf():
    raw_md = MD_PATH.read_text(encoding="utf-8")

    # Embed gambar lokal sebagai base64 agar 100% ter-render tajam di PDF
    def img_repl(m):
        alt, src = m.group(1), m.group(2)
        uri = img_to_data_uri(src)
        return f'<figure><img src="{uri}" alt="{alt}"/></figure>'

    md_prepared = re.sub(r"!\[([^\]]*)\]\(([^)]+)\)", img_repl, raw_md)
    md_prepared = format_display_equations(md_prepared)
    body_html = markdown.markdown(md_prepared, extensions=["tables", "fenced_code"])

    html_doc = f"""<!DOCTYPE html>
<html lang="id">
<head>
<meta charset="UTF-8"/>
<style>
  @page {{
    size: A4;
    margin: 20mm 18mm 20mm 18mm;
  }}
  body {{
    font-family: "Liberation Serif", "Times New Roman", Georgia, serif;
    font-size: 10.5pt;
    line-height: 1.52;
    color: #141c22;
    text-align: justify;
  }}
  h1 {{
    font-size: 15.5pt;
    line-height: 1.3;
    text-align: center;
    color: #0d3b3e;
    margin-bottom: 8px;
  }}
  h1 + p {{
    text-align: center;
    font-size: 9.8pt;
    color: #334155;
    margin-bottom: 14px;
  }}
  h2 {{
    font-family: "Liberation Sans", Arial, sans-serif;
    font-size: 11.8pt;
    color: #0f6e72;
    border-bottom: 1.2px solid #cbd5e1;
    padding-bottom: 3px;
    margin-top: 18px;
    margin-bottom: 8px;
    page-break-after: avoid;
  }}
  h3 {{
    font-family: "Liberation Sans", Arial, sans-serif;
    font-size: 10.5pt;
    color: #1e293b;
    margin-top: 13px;
    margin-bottom: 5px;
    page-break-after: avoid;
  }}
  hr {{
    border: none;
    border-top: 1px solid #cbd5e1;
    margin: 12px 0;
  }}
  .eq-box {{
    font-family: "DejaVu Sans", "Cambria Math", serif;
    font-size: 9.8pt;
    background: #f4f8f8;
    border-left: 3px solid #0f6e72;
    padding: 8px 14px;
    margin: 10px 0;
    text-align: center;
    border-radius: 4px;
    page-break-inside: avoid;
  }}
  .math-inline {{
    font-family: "DejaVu Sans", serif;
    font-style: italic;
  }}
  figure {{
    margin: 12px 0 4px;
    text-align: center;
    page-break-inside: avoid;
  }}
  figure img {{
    max-width: 96%;
    max-height: 255px;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
  }}
  table {{
    width: 100%;
    border-collapse: collapse;
    font-family: "Liberation Sans", Arial, sans-serif;
    font-size: 8.2pt;
    margin: 10px 0 14px;
    page-break-inside: avoid;
  }}
  th, td {{
    border: 1px solid #94a3b8;
    padding: 5px 6px;
    text-align: left;
  }}
  th {{
    background: #e6f2f2;
    color: #0d3b3e;
    font-weight: bold;
  }}
  code {{
    font-family: "DejaVu Sans Mono", monospace;
    font-size: 8.6pt;
    background: #f1f5f9;
    padding: 1px 4px;
    border-radius: 3px;
  }}
</style>
</head>
<body>
{body_html}
</body>
</html>"""

    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True, executable_path="/usr/bin/chromium")
        page = browser.new_page()
        page.set_content(html_doc, wait_until="load")
        page.pdf(
            path=str(PDF_PATH),
            format="A4",
            print_background=True,
            display_header_footer=True,
            header_template='<div style="font-size:7.5pt;color:#64748b;width:100%;text-align:center;font-family:sans-serif;">Karya Ilmiah Mata Kuliah Artificial Intelligence — KarirAI (Rust + Google Firebase)</div>',
            footer_template='<div style="font-size:8pt;color:#64748b;width:100%;text-align:center;font-family:sans-serif;">Halaman <span class="pageNumber"></span> dari <span class="totalPages"></span></div>',
        )
        browser.close()
    print(f"[OK] PDF Karya Ilmiah dibuat: {PDF_PATH} ({PDF_PATH.stat().st_size / 1024:.1f} KB)")


def build_docx():
    doc = Document()
    for s in doc.sections:
        s.top_margin = Inches(0.85)
        s.bottom_margin = Inches(0.85)
        s.left_margin = Inches(0.9)
        s.right_margin = Inches(0.9)

    normal_style = doc.styles["Normal"]
    normal_style.font.name = "Times New Roman"
    normal_style.font.size = Pt(11)

    lines = MD_PATH.read_text(encoding="utf-8").splitlines()
    i = 0
    in_eq = False
    eq_buf = []

    while i < len(lines):
        line = lines[i].rstrip()

        if line.startswith("$$"):
            if line.endswith("$$") and len(line) > 4:
                p = doc.add_paragraph()
                p.alignment = WD_ALIGN_PARAGRAPH.CENTER
                r = p.add_run(line.strip("$ "))
                r.italic = True
                r.font.size = Pt(10.5)
                i += 1
                continue
            in_eq = not in_eq
            if not in_eq and eq_buf:
                p = doc.add_paragraph()
                p.alignment = WD_ALIGN_PARAGRAPH.CENTER
                r = p.add_run(" ".join(eq_buf))
                r.italic = True
                r.font.size = Pt(10.5)
                eq_buf = []
            i += 1
            continue

        if in_eq:
            eq_buf.append(line.strip())
            i += 1
            continue

        if not line or line == "---":
            i += 1
            continue

        # Tabel Markdown
        if line.startswith("|") and i + 1 < len(lines) and lines[i + 1].startswith("|---"):
            headers = [c.strip().strip("*") for c in line.strip("|").split("|")]
            i += 2
            rows_data = []
            while i < len(lines) and lines[i].startswith("|"):
                rows_data.append([c.strip().replace("**", "") for c in lines[i].strip("|").split("|")])
                i += 1
            tbl = doc.add_table(rows=1 + len(rows_data), cols=len(headers))
            tbl.style = "Table Grid"
            tbl.alignment = WD_TABLE_ALIGNMENT.CENTER
            for col_idx, h_text in enumerate(headers):
                cell = tbl.cell(0, col_idx)
                cell.text = h_text
                for run in cell.paragraphs[0].runs:
                    run.bold = True
                    run.font.size = Pt(8.5)
            for r_idx, r_vals in enumerate(rows_data, start=1):
                for col_idx, val in enumerate(r_vals[: len(headers)]):
                    cell = tbl.cell(r_idx, col_idx)
                    cell.text = val
                    for run in cell.paragraphs[0].runs:
                        run.font.size = Pt(8.5)
            continue

        # Gambar Markdown
        img_m = re.match(r"!\[([^\]]*)\]\(([^)]+)\)", line)
        if img_m:
            img_file = (PAPER_DIR / img_m.group(2)).resolve()
            if img_file.exists():
                p = doc.add_paragraph()
                p.alignment = WD_ALIGN_PARAGRAPH.CENTER
                p.add_run().add_picture(str(img_file), width=Inches(5.8))
            i += 1
            continue

        if line.startswith("# "):
            p = doc.add_heading(line[2:].replace("*", ""), level=0)
            p.alignment = WD_ALIGN_PARAGRAPH.CENTER
        elif line.startswith("## "):
            h = doc.add_heading(line[3:].replace("*", ""), level=1)
            for r in h.runs:
                r.font.color.rgb = RGBColor(15, 110, 114)
        elif line.startswith("### "):
            doc.add_heading(line[4:].replace("*", ""), level=2)
        else:
            clean = re.sub(r"\*\*([^*]+)\*\*", r"\1", line)
            clean = re.sub(r"\*([^*]+)\*", r"\1", clean)
            clean = clean.replace("`", "")
            p = doc.add_paragraph(clean)
            if line.startswith("*Gambar "):
                p.alignment = WD_ALIGN_PARAGRAPH.CENTER
                for r in p.runs:
                    r.italic = True
                    r.font.size = Pt(9.5)
            else:
                p.alignment = WD_ALIGN_PARAGRAPH.JUSTIFY

        i += 1

    doc.save(str(DOCX_PATH))
    print(f"[OK] DOCX Karya Ilmiah dibuat: {DOCX_PATH} ({DOCX_PATH.stat().st_size / 1024:.1f} KB)")


if __name__ == "__main__":
    build_pdf()
    build_docx()
