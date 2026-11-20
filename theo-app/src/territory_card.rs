//! Territory cards as PDF (M-07): one A4 page per territory with the title, category and labels,
//! the map (the tile image when there is one, with the areas, lines and markers drawn on top),
//! the do-not-call addresses, the description and the link. Plain PDF 1.4, like `export.rs`.

use crate::export::{fit, pdf_string, text_width};
use crate::territories::ShapeKind;

/// A shape on a card map: what it is, its points in image pixels, its title, and its colour
/// (PDF `r g b`).
pub type CardShape = (ShapeKind, Vec<(f64, f64)>, Option<String>, &'static str);

/// The map of a card. Shapes are in image pixels (0..width, 0..height, y down).
#[derive(Clone, Debug, Default)]
pub struct CardMap {
    /// The tiles as one JPEG image, if they could be read.
    pub jpeg: Option<Vec<u8>>,
    pub width: f64,
    pub height: f64,
    pub shapes: Vec<CardShape>,
    pub attribution: String,
}

#[derive(Clone, Debug, Default)]
pub struct Card {
    pub title: String,
    pub congregation: String,
    /// "Business" (empty: none).
    pub category: String,
    /// "Has an elevator · Gated".
    pub labels: String,
    pub description: Option<String>,
    /// "Address · note · date".
    pub do_not_calls: Vec<String>,
    pub url: Option<String>,
    /// "Printed 20 Nov 2026".
    pub printed: String,
    pub map: CardMap,
}

const W: f32 = 595.28;
const H: f32 = 841.89;
const M: f32 = 40.0;
const BLUE: &str = "0.15 0.39 0.92";

/// The PDF colour of a marker.
pub fn marker_rgb(c: Option<crate::people::LabelColor>) -> &'static str {
    use crate::people::LabelColor as C;
    match c {
        Some(C::Green) => "0.13 0.6 0.33",
        Some(C::Yellow) => "0.85 0.6 0.05",
        Some(C::Red) => RED,
        Some(C::Gray) => "0.29 0.33 0.39",
        Some(C::Blue) | None => BLUE,
    }
}
const RED: &str = "0.86 0.15 0.15";

fn text(out: &mut Vec<u8>, font: &str, size: f32, x: f32, y: f32, s: &str, rgb: &str) {
    out.extend_from_slice(format!("BT {rgb} rg /{font} {size} Tf {x:.2} {y:.2} Td ").as_bytes());
    out.extend_from_slice(&pdf_string(s));
    out.extend_from_slice(b" Tj ET\n");
}

/// Splits text into lines no wider than `max`.
fn wrap(s: &str, max: f32, size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in s.lines() {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if text_width(&candidate, size, false) > max && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

fn circle(out: &mut Vec<u8>, x: f32, y: f32, r: f32) {
    let k = 0.5523 * r;
    out.extend_from_slice(
        format!(
            "{:.2} {y:.2} m {:.2} {:.2} {:.2} {:.2} {x:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {y:.2} c {:.2} {:.2} {:.2} {:.2} {x:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {y:.2} c ",
            x + r, x + r, y + k, x + k, y + r, y + r,
            x - k, y + r, x - r, y + k, x - r,
            x - r, y - k, x - k, y - r, y - r,
            x + k, y - r, x + r, y - k, x + r,
        )
        .as_bytes(),
    );
}

/// The content of one page, and whether it uses the image.
fn page(card: &Card) -> Vec<u8> {
    let mut c = Vec::new();
    let mut y = H - M;
    // Title, category and labels.
    text(&mut c, "F2", 22.0, M, y - 22.0, &fit(&format!("Territory {}", card.title), W - 2.0 * M - 150.0, 22.0, true), "0.1 0.1 0.12");
    if !card.category.is_empty() {
        let w = text_width(&card.category, 11.0, true);
        text(&mut c, "F2", 11.0, W - M - w, y - 18.0, &card.category, BLUE);
    }
    y -= 34.0;
    let sub = [card.labels.as_str(), card.congregation.as_str()].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" · ");
    text(&mut c, "F1", 10.0, M, y - 10.0, &fit(&sub, W - 2.0 * M, 10.0, false), "0.42 0.45 0.5");
    y -= 22.0;

    // The map.
    let mw = W - 2.0 * M;
    let mh = (mw * (card.map.height / card.map.width.max(1.0)) as f32).min(380.0);
    let my = y - mh;
    if card.map.jpeg.is_some() {
        c.extend_from_slice(format!("q {mw:.2} 0 0 {mh:.2} {M:.2} {my:.2} cm /Im1 Do Q\n").as_bytes());
    } else {
        c.extend_from_slice(format!("0.95 0.96 0.97 rg {M:.2} {my:.2} {mw:.2} {mh:.2} re f\n").as_bytes());
    }
    let sx = mw / card.map.width.max(1.0) as f32;
    let sy = mh / card.map.height.max(1.0) as f32;
    let at = |(px, py): (f64, f64)| (M + px as f32 * sx, my + mh - py as f32 * sy);
    c.extend_from_slice(format!("q {M:.2} {my:.2} {mw:.2} {mh:.2} re W n\n").as_bytes());
    for (kind, pts, _, _) in card.map.shapes.iter().filter(|s| s.0 == ShapeKind::Area) {
        let _ = kind;
        let mut path = String::new();
        for (i, p) in pts.iter().enumerate() {
            let (x, y) = at(*p);
            path.push_str(&format!("{x:.2} {y:.2} {} ", if i == 0 { "m" } else { "l" }));
        }
        c.extend_from_slice(format!("q /GS1 gs {BLUE} rg {BLUE} RG 2.2 w 1 j {path}h B Q\n").as_bytes());
    }
    for (_, pts, _, _) in card.map.shapes.iter().filter(|s| s.0 == ShapeKind::Line) {
        let mut path = String::new();
        for (i, p) in pts.iter().enumerate() {
            let (x, y) = at(*p);
            path.push_str(&format!("{x:.2} {y:.2} {} ", if i == 0 { "m" } else { "l" }));
        }
        c.extend_from_slice(format!("q {BLUE} RG 3 w 1 J [7 5] 0 d {path}S Q\n").as_bytes());
    }
    for (_, pts, title, rgb) in card.map.shapes.iter().filter(|s| s.0 == ShapeKind::Marker) {
        let Some(p) = pts.first() else { continue };
        let (x, y) = at(*p);
        c.extend_from_slice(format!("q {rgb} rg 1 1 1 RG 1.6 w ").as_bytes());
        circle(&mut c, x, y, 5.0);
        c.extend_from_slice(b"B Q\n");
        if let Some(t) = title {
            let t = fit(t, 180.0, 8.0, true);
            let tw = text_width(&t, 8.0, true);
            c.extend_from_slice(format!("1 1 1 rg {:.2} {:.2} {:.2} 11 re f\n", x + 7.0, y - 5.0, tw + 6.0).as_bytes());
            text(&mut c, "F2", 8.0, x + 10.0, y - 2.5, &t, "0.1 0.1 0.12");
        }
    }
    c.extend_from_slice(b"Q\n");
    c.extend_from_slice(format!("0.8 0.82 0.85 RG 0.6 w {M:.2} {my:.2} {mw:.2} {mh:.2} re S\n").as_bytes());
    let attribution = if card.map.jpeg.is_some() { card.map.attribution.clone() } else { "Map images were not available: the boundary only.".to_string() };
    if !attribution.is_empty() {
        let a = fit(&attribution, mw, 7.0, false);
        text(&mut c, "F1", 7.0, W - M - text_width(&a, 7.0, false), my - 10.0, &a, "0.42 0.45 0.5");
    }
    y = my - 26.0;

    // Do not call: easy to see.
    if !card.do_not_calls.is_empty() {
        let n = card.do_not_calls.len().min(10);
        let bh = 22.0 + n as f32 * 14.0 + 6.0;
        c.extend_from_slice(format!("1 0.95 0.95 rg {RED} RG 1.2 w {M:.2} {:.2} {mw:.2} {bh:.2} re B\n", y - bh).as_bytes());
        text(&mut c, "F2", 11.0, M + 10.0, y - 16.0, &format!("DO NOT CALL ({})", card.do_not_calls.len()), RED);
        for (i, d) in card.do_not_calls.iter().take(n).enumerate() {
            text(&mut c, "F1", 10.0, M + 10.0, y - 32.0 - i as f32 * 14.0, &fit(&format!("•  {d}"), mw - 20.0, 10.0, false), "0.1 0.1 0.12");
        }
        if card.do_not_calls.len() > n {
            text(&mut c, "F1", 9.0, M + mw - 120.0, y - 16.0, &format!("+{} more in the app", card.do_not_calls.len() - n), RED);
        }
        y -= bh + 16.0;
    }

    // Description and link.
    if let Some(d) = &card.description {
        text(&mut c, "F2", 10.0, M, y - 10.0, "Description", "0.1 0.1 0.12");
        y -= 16.0;
        for line in wrap(d, mw, 10.0) {
            if y < M + 60.0 {
                break;
            }
            text(&mut c, "F1", 10.0, M, y - 10.0, &line, "0.1 0.1 0.12");
            y -= 14.0;
        }
        y -= 8.0;
    }
    if let Some(u) = &card.url {
        text(&mut c, "F2", 10.0, M, y - 10.0, "Link", "0.1 0.1 0.12");
        text(&mut c, "F1", 10.0, M + 34.0, y - 10.0, &fit(u, mw - 34.0, 10.0, false), BLUE);
    }
    // Footer.
    text(&mut c, "F1", 8.0, M, M - 14.0, &fit(&format!("{} · Territory {} · {}", card.congregation, card.title, card.printed), mw, 8.0, false), "0.42 0.45 0.5");
    c
}

/// The cards as one PDF, one page each.
pub fn cards_pdf(cards: &[Card]) -> Vec<u8> {
    // Objects: 1 catalog, 2 pages, 3 Helvetica, 4 Helvetica-Bold, 5 transparency, then per card
    // a page, its content and (when there is a map image) the image.
    let mut objects: Vec<Vec<u8>> = vec![Vec::new(), Vec::new()];
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
    objects.push(b"<< /Type /ExtGState /ca 0.16 >>".to_vec());
    let mut kids = Vec::new();
    for card in cards {
        let page_no = objects.len() + 1;
        let content_no = page_no + 1;
        let image_no = page_no + 2;
        kids.push(format!("{page_no} 0 R"));
        let image = card.map.jpeg.as_ref().map(|_| format!(" /XObject << /Im1 {image_no} 0 R >>")).unwrap_or_default();
        objects.push(
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {W} {H}] /Resources << /Font << /F1 3 0 R /F2 4 0 R >> /ExtGState << /GS1 5 0 R >>{image} >> /Contents {content_no} 0 R >>").into_bytes(),
        );
        let content = page(card);
        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(&content);
        stream.extend_from_slice(b"\nendstream");
        objects.push(stream);
        if let Some(jpeg) = &card.map.jpeg {
            let (iw, ih) = jpeg_size(jpeg).unwrap_or((card.map.width as u32, card.map.height as u32));
            let mut img = format!("<< /Type /XObject /Subtype /Image /Width {iw} /Height {ih} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpeg.len()).into_bytes();
            img.extend_from_slice(jpeg);
            img.extend_from_slice(b"\nendstream");
            objects.push(img);
        }
    }
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    objects[1] = format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), kids.len()).into_bytes();
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

/// Width and height from a JPEG's frame header.
fn jpeg_size(b: &[u8]) -> Option<(u32, u32)> {
    let mut i = 2;
    while i + 9 < b.len() {
        if b[i] != 0xFF {
            return None;
        }
        let marker = b[i + 1];
        let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if matches!(marker, 0xC0..=0xC3) {
            return Some((u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32, u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32));
        }
        i += 2 + len;
    }
    None
}

/// Decodes standard base64 (the canvas image of the map).
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let bytes: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            n |= val(*c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round() {
        assert_eq!(base64_decode("aGVsbG8=").as_deref(), Some(&b"hello"[..]));
        assert_eq!(base64_decode("aGk=").as_deref(), Some(&b"hi"[..]));
    }

    #[test]
    fn a_card_without_image_is_a_valid_pdf() {
        let card = Card {
            title: "12".into(),
            congregation: "Riverside".into(),
            do_not_calls: vec!["1 Main Street".into()],
            description: Some("Shops and flats.".into()),
            map: CardMap { width: 1000.0, height: 680.0, shapes: vec![(ShapeKind::Area, vec![(10.0, 10.0), (500.0, 20.0), (400.0, 600.0)], None, BLUE), (ShapeKind::Marker, vec![(200.0, 200.0)], Some("Gate".into()), RED)], ..Default::default() },
            ..Default::default()
        };
        let pdf = cards_pdf(&[card.clone(), card]);
        let s = String::from_utf8_lossy(&pdf);
        assert!(s.starts_with("%PDF-1.4"));
        assert!(s.contains("/Count 2"));
        assert!(s.trim_end().ends_with("%%EOF"));
        assert!(s.contains(r"DO NOT CALL \(1\)"));
    }
}
