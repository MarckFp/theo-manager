//! Exports a table of text to CSV, DOCX or PDF, on the device, with no extra crates
//! (bundle budget, `REQUIREMENTS.md` 9.2).
//!
//! The files are NOT encrypted. Screens warn before exporting (docs/model/m01-people.md).
//! Limits of this first version: the PDF uses the built-in Helvetica font, so characters outside
//! Western European (Windows-1252) print as `?`. A later version embeds a font subset.

/// The data to export.
#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub title: String,
    /// One line under the title (for example the active filters).
    pub subtitle: String,
    pub columns: Vec<String>,
    /// Relative column widths for DOCX and PDF (same length as `columns`).
    pub widths: Vec<f32>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Csv,
    Docx,
    Pdf,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "csv" => Some(Format::Csv),
            "docx" => Some(Format::Docx),
            "pdf" => Some(Format::Pdf),
            _ => None,
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Format::Csv => "csv",
            Format::Docx => "docx",
            Format::Pdf => "pdf",
        }
    }
    pub fn mime(self) -> &'static str {
        match self {
            Format::Csv => "text/csv;charset=utf-8",
            Format::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Format::Pdf => "application/pdf",
        }
    }
}

pub fn render(t: &Table, f: Format) -> Vec<u8> {
    match f {
        Format::Csv => csv(t),
        Format::Docx => docx(t),
        Format::Pdf => pdf(t),
    }
}

// ---------------------------------------------------------------------------------------------
// CSV (RFC 4180, UTF-8 with BOM so spreadsheet apps read accents correctly)
// ---------------------------------------------------------------------------------------------

fn csv_cell(s: &str) -> String {
    // Formula injection: spreadsheets run cells that start with these characters.
    let guarded = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) { format!("'{s}") } else { s.to_string() };
    if guarded.contains([',', '"', '\n', '\r', ';']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

pub fn csv(t: &Table) -> Vec<u8> {
    let mut out = String::from("\u{feff}");
    for row in std::iter::once(&t.columns).chain(t.rows.iter()) {
        out.push_str(&row.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out.into_bytes()
}

// ---------------------------------------------------------------------------------------------
// ZIP (stored, no compression) for DOCX
// ---------------------------------------------------------------------------------------------

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for b in data {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn zip(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    let le16 = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let le32 = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        // Local file header
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 20); // version needed
        le16(&mut out, 0x0800); // UTF-8 names
        le16(&mut out, 0); // stored
        le16(&mut out, 0); // time
        le16(&mut out, 0x21); // date: 1980-01-01
        le32(&mut out, crc);
        le32(&mut out, size);
        le32(&mut out, size);
        le16(&mut out, name.len() as u16);
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        // Central directory entry
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 20);
        le16(&mut central, 20);
        le16(&mut central, 0x0800);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0x21);
        le32(&mut central, crc);
        le32(&mut central, size);
        le32(&mut central, size);
        le16(&mut central, name.len() as u16);
        le16(&mut central, 0); // extra
        le16(&mut central, 0); // comment
        le16(&mut central, 0); // disk
        le16(&mut central, 0); // internal attributes
        le32(&mut central, 0); // external attributes
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, files.len() as u16);
    le16(&mut out, files.len() as u16);
    le32(&mut out, cd_size);
    le32(&mut out, cd_offset);
    le16(&mut out, 0);
    out
}

// ---------------------------------------------------------------------------------------------
// DOCX (minimal WordprocessingML)
// ---------------------------------------------------------------------------------------------

fn xml(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn run(text: &str, bold: bool, size_half_points: u32, gray: bool) -> String {
    let b = if bold { "<w:b/>" } else { "" };
    let c = if gray { "<w:color w:val=\"6B7280\"/>" } else { "" };
    format!("<w:r><w:rPr>{b}{c}<w:sz w:val=\"{size_half_points}\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r>", xml(text))
}

pub fn docx(t: &Table) -> Vec<u8> {
    const WIDTH: f32 = 9906.0; // A4 width minus 2 cm margins, in twentieths of a point
    let total: f32 = t.widths.iter().sum::<f32>().max(1.0);
    let grid: Vec<u32> = t.widths.iter().map(|w| (w / total * WIDTH) as u32).collect();
    let cell = |text: &str, w: u32, header: bool| {
        let shade = if header { "<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"F3F4F6\"/>" } else { "" };
        format!("<w:tc><w:tcPr><w:tcW w:w=\"{w}\" w:type=\"dxa\"/>{shade}</w:tcPr><w:p><w:pPr><w:spacing w:before=\"40\" w:after=\"40\"/></w:pPr>{}</w:p></w:tc>", run(text, header, 18, false))
    };
    let mut rows = String::new();
    rows.push_str("<w:tr><w:trPr><w:tblHeader/></w:trPr>");
    for (c, w) in t.columns.iter().zip(&grid) {
        rows.push_str(&cell(c, *w, true));
    }
    rows.push_str("</w:tr>");
    for r in &t.rows {
        rows.push_str("<w:tr><w:trPr><w:cantSplit/></w:trPr>");
        for (c, w) in r.iter().zip(&grid) {
            rows.push_str(&cell(c, *w, false));
        }
        rows.push_str("</w:tr>");
    }
    let border = "w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"D1D5DB\"";
    let grid_cols: String = grid.iter().map(|w| format!("<w:gridCol w:w=\"{w}\"/>")).collect();
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:body>\
<w:p>{title}</w:p><w:p><w:pPr><w:spacing w:after=\"200\"/></w:pPr>{sub}</w:p>\
<w:tbl><w:tblPr><w:tblW w:w=\"{WIDTH}\" w:type=\"dxa\"/><w:tblBorders>\
<w:top {border}/><w:left {border}/><w:bottom {border}/><w:right {border}/><w:insideH {border}/><w:insideV {border}/>\
</w:tblBorders><w:tblLayout w:type=\"fixed\"/><w:tblCellMar><w:left w:w=\"80\" w:type=\"dxa\"/><w:right w:w=\"80\" w:type=\"dxa\"/></w:tblCellMar></w:tblPr>\
<w:tblGrid>{grid_cols}</w:tblGrid>{rows}</w:tbl>\
<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1000\" w:right=\"1000\" w:bottom=\"1000\" w:left=\"1000\" w:header=\"500\" w:footer=\"500\" w:gutter=\"0\"/></w:sectPr>\
</w:body></w:document>",
        title = run(&t.title, true, 32, false),
        sub = run(&t.subtitle, false, 18, true),
    );
    let content_types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
<Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
<Default Extension=\"xml\" ContentType=\"application/xml\"/>\
<Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
</Types>";
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/>\
</Relationships>";
    zip(&[
        ("[Content_Types].xml", content_types.as_bytes().to_vec()),
        ("_rels/.rels", rels.as_bytes().to_vec()),
        ("word/document.xml", document.into_bytes()),
    ])
}

// ---------------------------------------------------------------------------------------------
// PDF (A4, Helvetica, WinAnsiEncoding)
// ---------------------------------------------------------------------------------------------

/// Helvetica advance widths (1/1000 em) for ASCII 32..=126.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556,
    556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833,
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556,
    556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334,
    260, 334, 584,
];

/// Windows-1252 byte for a character, or `?`.
fn win_ansi(c: char) -> u8 {
    let u = c as u32;
    match u {
        0x20..=0x7E | 0xA0..=0xFF => u as u8,
        _ => match c {
            '€' => 0x80,
            '‚' => 0x82,
            '„' => 0x84,
            '…' => 0x85,
            'Š' => 0x8A,
            'Œ' => 0x8C,
            'Ž' => 0x8E,
            '‘' => 0x91,
            '’' => 0x92,
            '“' => 0x93,
            '”' => 0x94,
            '•' => 0x95,
            '–' => 0x96,
            '—' => 0x97,
            'š' => 0x9A,
            'œ' => 0x9C,
            'ž' => 0x9E,
            'Ÿ' => 0x9F,
            _ => b'?',
        },
    }
}

fn char_width(c: char, size: f32, bold: bool) -> f32 {
    let base = crate::people::fold(&c.to_string()).chars().next().unwrap_or(c);
    let w = match base as u32 {
        0x20..=0x7E => f32::from(HELVETICA[(base as u32 - 0x20) as usize]),
        _ => 556.0,
    };
    w * size / 1000.0 * if bold { 1.06 } else { 1.0 }
}

pub(crate) fn text_width(s: &str, size: f32, bold: bool) -> f32 {
    s.chars().map(|c| char_width(c, size, bold)).sum()
}

/// Cuts `s` so it fits `max` points, with "…" when cut.
pub(crate) fn fit(s: &str, max: f32, size: f32, bold: bool) -> String {
    if text_width(s, size, bold) <= max {
        return s.to_string();
    }
    let ell = char_width('…', size, bold);
    let mut out = String::new();
    let mut w = 0.0;
    for c in s.chars() {
        let cw = char_width(c, size, bold);
        if w + cw + ell > max {
            break;
        }
        out.push(c);
        w += cw;
    }
    out.push('…');
    out
}

pub(crate) fn pdf_string(s: &str) -> Vec<u8> {
    let mut out = vec![b'('];
    for c in s.chars() {
        let b = win_ansi(c);
        if matches!(b, b'(' | b')' | b'\\') {
            out.push(b'\\');
        }
        out.push(b);
    }
    out.push(b')');
    out
}

fn text_op(out: &mut Vec<u8>, font: &str, size: f32, x: f32, y: f32, s: &str, gray: bool) {
    let color = if gray { "0.42 0.45 0.5 rg" } else { "0.1 0.1 0.12 rg" };
    out.extend_from_slice(format!("BT {color} /{font} {size} Tf {x:.2} {y:.2} Td ").as_bytes());
    out.extend_from_slice(&pdf_string(s));
    out.extend_from_slice(b" Tj ET\n");
}

pub fn pdf(t: &Table) -> Vec<u8> {
    const W: f32 = 595.28;
    const H: f32 = 841.89;
    const M: f32 = 40.0;
    const ROW: f32 = 18.0;
    const SIZE: f32 = 9.0;
    const PAD: f32 = 4.0;
    let total: f32 = t.widths.iter().sum::<f32>().max(1.0);
    let cols: Vec<f32> = t.widths.iter().map(|w| w / total * (W - 2.0 * M)).collect();

    // Lay out the pages.
    let mut pages: Vec<Vec<u8>> = Vec::new();
    let mut rows = t.rows.iter().peekable();
    let mut first = true;
    while first || rows.peek().is_some() {
        let mut c = Vec::new();
        let mut y = H - M;
        if first {
            text_op(&mut c, "F2", 16.0, M, y - 16.0, &t.title, false);
            y -= 24.0;
            text_op(&mut c, "F1", SIZE, M, y - SIZE, &fit(&t.subtitle, W - 2.0 * M, SIZE, false), true);
            y -= 22.0;
        }
        // Header row
        c.extend_from_slice(format!("0.953 0.957 0.965 rg {M:.2} {:.2} {:.2} {ROW:.2} re f\n", y - ROW, W - 2.0 * M).as_bytes());
        let mut x = M;
        for (name, w) in t.columns.iter().zip(&cols) {
            text_op(&mut c, "F2", SIZE, x + PAD, y - ROW + 6.0, &fit(name, w - 2.0 * PAD, SIZE, true), false);
            x += w;
        }
        y -= ROW;
        while let Some(r) = rows.peek() {
            if y - ROW < M + 20.0 {
                break;
            }
            let mut x = M;
            for (v, w) in r.iter().zip(&cols) {
                text_op(&mut c, "F1", SIZE, x + PAD, y - ROW + 6.0, &fit(v, w - 2.0 * PAD, SIZE, false), false);
                x += w;
            }
            y -= ROW;
            c.extend_from_slice(format!("0.85 0.86 0.88 RG 0.5 w {M:.2} {y:.2} m {:.2} {y:.2} l S\n", W - M).as_bytes());
            rows.next();
        }
        pages.push(c);
        first = false;
    }
    let n = pages.len();
    for (i, c) in pages.iter_mut().enumerate() {
        text_op(c, "F1", 8.0, M, M - 14.0, &format!("{} · page {} of {}", t.title, i + 1, n), true);
    }

    // Objects: 1 catalog, 2 pages, 3 Helvetica, 4 Helvetica-Bold, then (page, content) pairs.
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 5 + i * 2)).collect();
    objects.push(format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).into_bytes());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
    for (i, c) in pages.into_iter().enumerate() {
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {W} {H}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                6 + i * 2
            )
            .into_bytes(),
        );
        let mut stream = format!("<< /Length {} >>\nstream\n", c.len()).into_bytes();
        stream.extend_from_slice(&c);
        stream.extend_from_slice(b"\nendstream");
        objects.push(stream);
    }
    let mut out = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: usize) -> Table {
        Table {
            title: "All users".into(),
            subtitle: "Gender: Female".into(),
            columns: vec!["Name".into(), "Notes".into()],
            widths: vec![1.0, 2.0],
            rows: (0..rows).map(|i| vec![format!("Lucía {i}"), "=1+1, \"quoted\"".into()]).collect(),
        }
    }

    #[test]
    fn csv_quotes_and_blocks_formulas() {
        let s = String::from_utf8(csv(&table(1))).unwrap_or_default();
        assert!(s.starts_with('\u{feff}'));
        assert!(s.contains("\"'=1+1, \"\"quoted\"\"\""));
    }

    #[test]
    fn crc32_matches_known_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn docx_is_a_zip_with_the_document() {
        let d = docx(&table(3));
        assert_eq!(&d[..4], b"PK\x03\x04");
        assert!(d.windows(17).any(|w| w == b"word/document.xml"));
    }

    #[test]
    fn pdf_has_pages_and_a_valid_trailer() {
        let p = pdf(&table(100));
        let s = String::from_utf8_lossy(&p);
        assert!(s.starts_with("%PDF-1.4"));
        assert!(s.contains("/Count 3"), "100 rows need 3 A4 pages");
        assert!(s.trim_end().ends_with("%%EOF"));
        // "í" is written as its Windows-1252 byte.
        assert!(p.contains(&0xED));
    }

    /// Writes sample files to `target/export-samples/` to open them by hand:
    /// `cargo test -- --ignored write_samples`.
    #[test]
    #[ignore]
    fn write_samples() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/export-samples");
        std::fs::create_dir_all(&dir).ok();
        let d = crate::mock::demo_directory(32);
        let t = Table {
            title: "Riverside \u{b7} All users".into(),
            subtitle: "Sample with invented data".into(),
            columns: vec!["Name".into(), "Type".into(), "Mobile phone".into()],
            widths: vec![2.0, 2.0, 1.5],
            rows: d.people.iter().map(|p| vec![format!("{} {}", p.first_name, p.last_name), p.publisher_type.label().into(), p.mobile_phone.clone().unwrap_or_default()]).collect(),
        };
        for f in [Format::Csv, Format::Docx, Format::Pdf] {
            std::fs::write(dir.join(format!("sample.{}", f.extension())), render(&t, f)).ok();
        }
    }
}
