//! Reads the official *Life and Ministry Meeting Workbook* EPUB (REQUIREMENTS.md 1.6, D-15): the
//! weeks with their Bible reading, songs, sections and parts (number, title, description,
//! minutes). The EPUB is downloaded from jw.org's public download links (or picked as a file) and
//! read **on the device**; the content is never uploaded (R-030, R-032). No HTML is scraped from
//! jw.org or wol.jw.org (R-031): this reads the EPUB file only.
//!
//! The reader does not depend on the language: sections are told apart by their colour classes
//! (teal: Treasures from God's Word, gold: Apply yourself to the field ministry, maroon: Living as
//! Christians), parts by their numbers, minutes by the first number in parentheses, songs by the
//! number in the song links.

use crate::midweek::{PartKind, SectionKind};

/// One part of a week, as printed in the workbook.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PartContent {
    /// The number in the workbook (1, 2, 3…).
    pub number: u8,
    pub section: SectionKind,
    pub kind: PartKind,
    pub title: String,
    pub description: String,
    pub minutes: u8,
}

/// One week of the workbook.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WeekContent {
    /// The document of the week inside the EPUB (for example "202026402").
    pub doc: String,
    /// Monday of the week, `YYYY-MM-DD`.
    pub monday: String,
    /// The heading of the week ("NOVEMBER 9-15").
    pub label: String,
    /// The weekly Bible reading ("JEREMIAH 51-52").
    pub reading: String,
    /// Opening, middle and closing song.
    pub songs: [Option<u16>; 3],
    pub parts: Vec<PartContent>,
}

/// A whole issue (two months).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Workbook {
    /// `YYYYMM` of the first month (for example "202611").
    pub issue: String,
    /// jw.org language code (for example "E").
    pub lang: String,
    pub weeks: Vec<WeekContent>,
}

impl Workbook {
    pub fn week(&self, monday: &str) -> Option<&WeekContent> {
        self.weeks.iter().find(|w| w.monday == monday)
    }

    /// The key used to keep it on the device.
    pub fn key(&self) -> String {
        key(&self.issue, &self.lang)
    }
}

pub fn key(issue: &str, lang: &str) -> String {
    format!("{issue}-{lang}")
}

/// The jw.org language code of an app language.
pub fn jw_language(code: &str) -> &'static str {
    match code {
        "es" => "S",
        "fr" => "F",
        "de" => "X",
        "it" => "I",
        "pt" => "T",
        _ => "E",
    }
}

/// The issues that may hold the week of `monday`: the issue of its month (issues start in odd
/// months), then the one before (a week that starts at the end of an issue's last month).
pub fn issues_for(monday: &str) -> Vec<String> {
    let Some((y, m, _)) = crate::people::parse_date(monday) else { return Vec::new() };
    let start = if m % 2 == 1 { m } else { m - 1 };
    let (py, pm) = if start == 1 { (y - 1, 11) } else { (y, start - 2) };
    vec![format!("{y:04}{start:02}"), format!("{py:04}{pm:02}")]
}

/// The public jw.org address that tells where the EPUB of an issue is (it answers with JSON).
pub fn links_url(issue: &str, lang: &str) -> String {
    format!("https://b.jw-cdn.org/apis/pub-media/GETPUBMEDIALINKS?output=json&pub=mwb&issue={issue}&fileformat=EPUB&langwritten={lang}&alllangs=0")
}

/// The EPUB address in the answer of [`links_url`].
pub fn epub_url_from_links(json: &str, lang: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v["files"][lang]["EPUB"][0]["file"]["url"].as_str().map(String::from)
}

/// The first Monday on or after the 1st of a month: the first week of an issue.
fn first_monday(y: i32, m: u32) -> i64 {
    let first = crate::prefs::days_from_ymd((y, m, 1));
    // Day 0 (1970-01-01) was a Thursday; Monday is (z + 3) % 7 == 0.
    let wd = (first + 3).rem_euclid(7);
    first + (7 - wd) % 7
}

fn iso(z: i64) -> String {
    let (y, m, d) = crate::prefs::ymd_from_days(z);
    format!("{y:04}-{m:02}-{d:02}")
}

// ---------------------------------------------------------------------------------------------
// EPUB (zip) reading
// ---------------------------------------------------------------------------------------------

/// The files of a zip archive (stored or deflated), by name. At most 40 MB unpacked in all.
fn unzip(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let u16_at = |i: usize| -> Option<usize> { Some(u16::from_le_bytes([*bytes.get(i)?, *bytes.get(i + 1)?]) as usize) };
    let u32_at = |i: usize| -> Option<usize> { Some(u32::from_le_bytes([*bytes.get(i)?, *bytes.get(i + 1)?, *bytes.get(i + 2)?, *bytes.get(i + 3)?]) as usize) };
    let bad = || "The file is not a valid EPUB.".to_string();
    // End of central directory: the last "PK\x05\x06".
    let eocd = (0..bytes.len().saturating_sub(21)).rev().find(|&i| bytes[i..].starts_with(&[0x50, 0x4b, 0x05, 0x06])).ok_or_else(bad)?;
    let count = u16_at(eocd + 10).ok_or_else(bad)?;
    let mut at = u32_at(eocd + 16).ok_or_else(bad)?;
    let mut out = Vec::new();
    let mut total = 0usize;
    for _ in 0..count {
        if u32_at(at) != Some(0x0201_4b50) {
            return Err(bad());
        }
        let method = u16_at(at + 10).ok_or_else(bad)?;
        let size = u32_at(at + 20).ok_or_else(bad)?;
        let unpacked = u32_at(at + 24).ok_or_else(bad)?;
        let (n, e, c) = (u16_at(at + 28).ok_or_else(bad)?, u16_at(at + 30).ok_or_else(bad)?, u16_at(at + 32).ok_or_else(bad)?);
        let local = u32_at(at + 42).ok_or_else(bad)?;
        let name = String::from_utf8_lossy(bytes.get(at + 46..at + 46 + n).ok_or_else(bad)?).to_string();
        at += 46 + n + e + c;
        // Only the text of the book is needed.
        if !(name.ends_with(".xhtml") || name.ends_with(".opf")) {
            continue;
        }
        total += unpacked;
        if total > 40 * 1024 * 1024 {
            return Err("The file is too big for a workbook.".into());
        }
        if u32_at(local) != Some(0x0403_4b50) {
            return Err(bad());
        }
        let start = local + 30 + u16_at(local + 26).ok_or_else(bad)? + u16_at(local + 28).ok_or_else(bad)?;
        let data = bytes.get(start..start + size).ok_or_else(bad)?;
        let content = match method {
            0 => data.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(data, unpacked.max(1)).map_err(|_| bad())?,
            _ => return Err("The EPUB uses a compression the app cannot read.".into()),
        };
        out.push((name, content));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// XHTML scanning
// ---------------------------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Token {
    Start { name: String, classes: String, closed: bool },
    End(String),
    Text(String),
}

fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';').filter(|e| *e < 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let c = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            e if e.starts_with("#x") => u32::from_str_radix(&e[2..], 16).ok().and_then(char::from_u32),
            e if e.starts_with('#') => e[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match c {
            Some(c) => out.push(c),
            None => out.push_str(&rest[..=end]),
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

fn tokens(html: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut rest = html;
    while !rest.is_empty() {
        match rest.find('<') {
            Some(0) => {
                if rest.starts_with("<!--") {
                    rest = rest.find("-->").map(|i| &rest[i + 3..]).unwrap_or("");
                    continue;
                }
                let Some(end) = rest.find('>') else { break };
                let tag = &rest[1..end];
                rest = &rest[end + 1..];
                if tag.starts_with('?') || tag.starts_with('!') {
                    continue;
                }
                if let Some(name) = tag.strip_prefix('/') {
                    out.push(Token::End(name.trim().to_lowercase()));
                    continue;
                }
                let closed = tag.ends_with('/');
                let tag = tag.trim_end_matches('/');
                let name = tag.split(|c: char| c.is_whitespace()).next().unwrap_or("").to_lowercase();
                let classes = tag
                    .find("class=\"")
                    .and_then(|i| {
                        let v = &tag[i + 7..];
                        v.find('"').map(|j| v[..j].to_string())
                    })
                    .unwrap_or_default();
                out.push(Token::Start { name, classes, closed });
            }
            Some(i) => {
                out.push(Token::Text(decode(&rest[..i])));
                rest = &rest[i..];
            }
            None => {
                out.push(Token::Text(decode(rest)));
                rest = "";
            }
        }
    }
    out
}

/// A heading or paragraph with its text, its classes and the texts of its links.
#[derive(Debug, Default)]
struct Block {
    tag: String,
    classes: String,
    text: String,
    links: Vec<String>,
    music: bool,
}

fn clean(s: &str) -> String {
    s.replace(['\u{200b}', '\u{a0}'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The blocks of a page, without figures, notes, answer boxes and the references at the end.
fn blocks(html: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut current: Option<Block> = None;
    let mut link: Option<String> = None;
    let mut skip = 0usize;
    let mut stack: Vec<(String, bool)> = Vec::new();
    for t in tokens(html) {
        match t {
            Token::Start { name, classes, closed } => {
                if classes.contains("groupExt") {
                    break;
                }
                let skipping = matches!(name.as_str(), "figure" | "aside" | "textarea" | "label" | "script" | "style" | "head") || classes.contains("gen-field");
                if closed {
                    continue;
                }
                stack.push((name.clone(), skipping));
                if skipping {
                    skip += 1;
                    continue;
                }
                if skip > 0 {
                    continue;
                }
                if matches!(name.as_str(), "h1" | "h2" | "h3" | "p") {
                    if let Some(b) = current.take() {
                        out.push(b);
                    }
                    current = Some(Block { tag: name.clone(), classes: classes.clone(), music: classes.contains("dc-icon--music"), ..Default::default() });
                } else if let Some(b) = current.as_mut() {
                    if classes.contains("dc-icon--music") {
                        b.music = true;
                    }
                    if name == "a" {
                        link = Some(String::new());
                    }
                }
            }
            Token::End(name) => {
                // Close up to the matching element.
                while let Some((n, skipping)) = stack.pop() {
                    if skipping {
                        skip = skip.saturating_sub(1);
                    }
                    if n == name {
                        break;
                    }
                }
                if skip > 0 {
                    continue;
                }
                if name == "a"
                    && let (Some(l), Some(b)) = (link.take(), current.as_mut())
                {
                    b.links.push(clean(&l));
                }
                if matches!(name.as_str(), "h1" | "h2" | "h3" | "p")
                    && let Some(b) = current.take()
                {
                    out.push(b);
                }
            }
            Token::Text(s) => {
                if skip > 0 {
                    continue;
                }
                if let Some(b) = current.as_mut() {
                    b.text.push_str(&s);
                    if let Some(l) = link.as_mut() {
                        l.push_str(&s);
                    }
                }
            }
        }
    }
    if let Some(b) = current.take() {
        out.push(b);
    }
    for b in &mut out {
        b.text = clean(&b.text);
    }
    out.retain(|b| !b.text.is_empty());
    out
}

fn section_of(classes: &str) -> Option<SectionKind> {
    if classes.contains("teal") {
        Some(SectionKind::Treasures)
    } else if classes.contains("gold") {
        Some(SectionKind::Ministry)
    } else if classes.contains("maroon") {
        Some(SectionKind::Living)
    } else {
        None
    }
}

fn first_number(s: &str) -> Option<u32> {
    let start = s.find(|c: char| c.is_ascii_digit())?;
    s[start..].chars().take_while(char::is_ascii_digit).collect::<String>().parse().ok()
}

/// "(10 min.) text" → (10, "text").
fn minutes_and_rest(s: &str) -> Option<(u8, String)> {
    let open = s.find('(')?;
    let close = s[open..].find(')')? + open;
    let inside = &s[open..close];
    let m = first_number(inside)?;
    if m == 0 || m > 120 || open > 3 {
        return None;
    }
    Some((m as u8, s[close + 1..].trim().to_string()))
}

/// What kind of part it is, from its section, its place and (for the field ministry) its title.
/// Unknown titles become a student part; planners can change the kind.
pub fn classify(section: SectionKind, index_in_section: usize, last_in_section: bool, title: &str, description: &str) -> PartKind {
    let t = title.to_lowercase();
    let d = description.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| t.contains(w));
    match section {
        SectionKind::Treasures => match index_in_section {
            0 => PartKind::TreasuresTalk,
            1 => PartKind::SpiritualGems,
            _ => PartKind::BibleReading,
        },
        SectionKind::Ministry => {
            if has(&["conversation", "conversación", "conversa", "gespräch", "conversazione"]) {
                PartKind::StartingConversation
            } else if has(&["following up", "follow", "interés", "interesse", "nachbesuch", "revisita"]) {
                PartKind::FollowingUp
            } else if has(&["disciple", "discípulo", "jünger", "discepol"]) {
                PartKind::MakingDisciples
            } else if has(&["explaining", "explicar", "expliquer", "erklären", "spiegare", "explicando"]) {
                PartKind::ExplainingBeliefs
            } else if d.contains("discussion") || d.contains("análisis") || d.contains("elder") || d.contains("anciano") {
                PartKind::Discussion
            } else if has(&["talk", "discurso", "discours", "vortrag", "discorso"]) {
                PartKind::StudentTalk
            } else {
                PartKind::StartingConversation
            }
        }
        SectionKind::Living if last_in_section => PartKind::BibleStudy,
        SectionKind::Living => PartKind::Living,
        SectionKind::Other => PartKind::Other,
    }
}

/// One week page. None when the page is not a week (the cover, the contents).
fn parse_week(doc: &str, html: &str) -> Option<WeekContent> {
    let bs = blocks(html);
    let label = bs.iter().find(|b| b.tag == "h1")?.text.clone();
    let reading = bs.iter().find(|b| b.tag == "h2" && section_of(&b.classes).is_none()).map(|b| b.text.clone()).unwrap_or_default();
    let mut songs: Vec<u16> = Vec::new();
    let mut section = SectionKind::Treasures;
    // (number, section, title, minutes, description lines)
    type Raw = (u8, SectionKind, String, Option<u8>, Vec<String>);
    let mut raw: Vec<Raw> = Vec::new();
    // Inside a part: its paragraphs are its description. Outside (before the first part, between a
    // section heading and its first part, after "Concluding comments"), a line that links to a
    // numbered item is a song. Older issues mark song headings with a music class instead.
    let mut in_part = false;
    let song_of = |b: &Block| b.links.iter().find_map(|l| first_number(l)).or_else(|| if b.music { first_number(&b.text) } else { None });
    for b in &bs {
        match b.tag.as_str() {
            "h2" => {
                if let Some(s) = section_of(&b.classes) {
                    section = s;
                    in_part = false;
                }
            }
            "h3" => {
                let numbered = b.text.split_once('.').filter(|(n, _)| !n.is_empty() && n.trim().chars().all(|c| c.is_ascii_digit()));
                match numbered {
                    Some((n, title)) => {
                        let s = section_of(&b.classes).unwrap_or(section);
                        raw.push((n.trim().parse().unwrap_or(0), s, title.trim().to_string(), None, Vec::new()));
                        in_part = true;
                    }
                    None => {
                        // "Concluding Comments" (or a song heading) ends the parts.
                        in_part = false;
                        if let Some(n) = song_of(b) {
                            songs.push(n as u16);
                        }
                    }
                }
            }
            "p" if !in_part => {
                if let Some(n) = song_of(b) {
                    songs.push(n as u16);
                }
            }
            "p" => {
                if let Some(last) = raw.last_mut() {
                    if last.3.is_none()
                        && let Some((m, rest)) = minutes_and_rest(&b.text)
                    {
                        last.3 = Some(m);
                        if !rest.is_empty() {
                            last.4.push(rest);
                        }
                    } else {
                        last.4.push(b.text.clone());
                    }
                }
            }
            _ => {}
        }
    }
    if raw.len() < 3 {
        return None;
    }
    let parts: Vec<PartContent> = raw
        .iter()
        .enumerate()
        .map(|(i, (number, s, title, minutes, lines))| {
            let in_section: Vec<usize> = raw.iter().enumerate().filter(|(_, r)| r.1 == *s).map(|(j, _)| j).collect();
            let index = in_section.iter().position(|j| *j == i).unwrap_or(0);
            let last = in_section.last() == Some(&i);
            let description = lines.join("\n");
            PartContent { number: *number, section: *s, kind: classify(*s, index, last, title, &description), title: title.clone(), description, minutes: minutes.unwrap_or(0) }
        })
        .collect();
    Some(WeekContent {
        doc: doc.to_string(),
        monday: String::new(),
        label,
        reading,
        songs: [songs.first().copied(), songs.get(1).copied(), songs.get(2).copied()],
        parts,
    })
}

/// The week documents in reading order (the spine of `content.opf`).
fn spine(opf: &str) -> Vec<String> {
    // id → href from the manifest, then the itemref order.
    let mut items: Vec<(String, String)> = Vec::new();
    for t in opf.split('<').filter(|t| t.starts_with("item ")) {
        let attr = |name: &str| t.find(&format!("{name}=\"")).and_then(|i| t[i + name.len() + 2..].split('"').next().map(String::from));
        if let (Some(id), Some(href)) = (attr("id"), attr("href")) {
            items.push((id, href));
        }
    }
    opf.split('<')
        .filter(|t| t.starts_with("itemref"))
        .filter_map(|t| t.find("idref=\"").and_then(|i| t[i + 7..].split('"').next()))
        .filter_map(|id| items.iter().find(|(i, _)| i == id).map(|(_, h)| h.clone()))
        .collect()
}

/// Reads a workbook EPUB of `issue` (`YYYYMM`) in `lang`.
pub fn read_epub(bytes: &[u8], issue: &str, lang: &str) -> Result<Workbook, String> {
    let files = unzip(bytes)?;
    let text = |name: &str| files.iter().find(|(n, _)| n == name || n.ends_with(&format!("/{name}"))).map(|(_, b)| String::from_utf8_lossy(b).to_string());
    let opf_name = files.iter().find(|(n, _)| n.ends_with(".opf")).map(|(n, _)| n.clone()).ok_or("The EPUB has no table of contents.")?;
    let base = opf_name.rsplit_once('/').map(|(b, _)| format!("{b}/")).unwrap_or_default();
    let order = spine(&text(&opf_name).unwrap_or_default());
    let mut weeks: Vec<WeekContent> = order
        .iter()
        .filter(|h| h.ends_with(".xhtml"))
        .filter_map(|h| {
            let html = text(&format!("{base}{h}"))?;
            parse_week(h.trim_end_matches(".xhtml").rsplit('/').next().unwrap_or(h), &html)
        })
        .collect();
    if weeks.is_empty() {
        return Err("No meeting weeks found. Is this a Life and Ministry Meeting Workbook?".into());
    }
    // Weeks follow each other from the first Monday of the issue's first month.
    let (y, m) = (issue.get(..4).and_then(|y| y.parse().ok()).unwrap_or(2026), issue.get(4..6).and_then(|m| m.parse().ok()).unwrap_or(1));
    let first = first_monday(y, m);
    for (i, w) in weeks.iter_mut().enumerate() {
        w.monday = iso(first + 7 * i as i64);
    }
    Ok(Workbook { issue: issue.into(), lang: lang.into(), weeks })
}

/// Why a download did not work, and where the file is when the user can fetch it themselves.
#[derive(Clone, Debug, PartialEq)]
pub struct DownloadError {
    pub message: String,
    /// The official EPUB address (the web build cannot read it: the user downloads it and
    /// picks the file, R-035).
    pub epub_url: Option<String>,
}

/// Guesses the issue from the file name of a picked EPUB ("mwb_E_202611.epub").
pub fn issue_from_file_name(name: &str) -> Option<(String, String)> {
    let stem = name.rsplit(['/', '\\']).next()?.trim_end_matches(".epub");
    let mut parts = stem.split('_');
    if parts.next()? != "mwb" {
        return None;
    }
    let lang = parts.next()?.to_string();
    let issue = parts.next()?.to_string();
    (issue.len() == 6 && issue.chars().all(|c| c.is_ascii_digit())).then_some((issue, lang))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An invented week page with the structure of the workbook (no jw.org text).
    const PAGE: &str = r##"<?xml version="1.0" encoding="utf-8" ?><html><head><title>x</title></head><body>
<header><h1 class="du-fontSize--basePlus2">WEEK ONE</h1><h2 class="du-fontSize--base"><strong><a href="#c1">BOOK 1-2</a></strong></h2></header>
<div class="bodyTxt">
<h3 class="dc-icon--music"><a href="x"><strong>Song 12</strong></a> <strong>and Prayer | Opening Comments</strong> <span>(1 min.)</span></h3>
<div><h2 class="du-color--teal-700"><strong>SECTION A</strong></h2></div>
<div><figure><img src="a.jpg"/></figure></div>
<h3 class="du-color--teal-700"><strong>1. First Talk</strong></h3>
<div><p>(10 min.)</p></div><p>Point one (<a href="#">ref</a>)</p><p>Point two</p>
<h3 class="du-color--teal-700"><strong>2. Gems</strong></h3><div><p>(10 min.)</p></div>
<ul><li><p>A question?</p><div class="gen-field"><label>Your answer</label><textarea></textarea></div></li></ul>
<h3 class="du-color--teal-700"><strong>3. Reading</strong></h3><div><p>(4 min.) Book 1:1-5 &amp; more</p></div>
<div><h2 class="du-color--gold-700"><strong>SECTION B</strong></h2></div>
<h3 class="du-color--gold-700"><strong>4. Starting a Conversation</strong></h3><div><p>(3 min.) HOUSE TO HOUSE. Something.</p></div>
<h3 class="du-color--gold-700"><strong>5. Talk</strong></h3><div><p>(5 min.) Theme: Something.</p></div>
<div><h2 class="du-color--maroon-600"><strong>SECTION C</strong></h2></div>
<h3 class="dc-icon--music"><a href="x"><strong>Song 34</strong></a></h3>
<h3 class="du-color--maroon-600"><strong>6. Local Needs</strong></h3><div><p>(15 min.)</p></div>
<h3 class="du-color--maroon-600"><strong>7. Congregation Bible Study</strong></h3><div><p>(30 min.) book chap. 1</p></div>
<h3><strong>Concluding Comments</strong> <span>(3 min.)</span> <strong>|</strong> <span class="dc-icon--music"><a href="x"><strong>Song 56</strong></a></span> <strong>and Prayer</strong></h3>
</div><div class="groupExt"><p>References that are not read.</p></div></body></html>"##;

    #[test]
    fn reads_a_week() {
        let w = parse_week("w1", PAGE).unwrap_or_else(|| panic!("no week"));
        assert_eq!(w.label, "WEEK ONE");
        assert_eq!(w.reading, "BOOK 1-2");
        assert_eq!(w.songs, [Some(12), Some(34), Some(56)]);
        assert_eq!(w.parts.len(), 7);
        let p = |n: usize| &w.parts[n];
        assert_eq!((p(0).number, p(0).section, p(0).kind, p(0).minutes), (1, SectionKind::Treasures, PartKind::TreasuresTalk, 10));
        assert_eq!(p(0).title, "First Talk");
        assert_eq!(p(0).description, "Point one (ref)\nPoint two");
        assert_eq!(p(1).kind, PartKind::SpiritualGems);
        assert_eq!(p(1).description, "A question?");
        assert_eq!((p(2).kind, p(2).minutes, p(2).description.as_str()), (PartKind::BibleReading, 4, "Book 1:1-5 & more"));
        assert_eq!((p(3).section, p(3).kind, p(3).minutes), (SectionKind::Ministry, PartKind::StartingConversation, 3));
        assert_eq!(p(4).kind, PartKind::StudentTalk);
        assert_eq!((p(5).section, p(5).kind), (SectionKind::Living, PartKind::Living));
        assert_eq!((p(6).kind, p(6).minutes), (PartKind::BibleStudy, 30));
    }

    #[test]
    fn reads_an_epub_and_dates_the_weeks() {
        // A tiny stored (uncompressed) zip with the package file and two week pages.
        let opf = r#"<package><manifest><item id="a" href="w1.xhtml" media-type="x"/><item id="b" href="w2.xhtml" media-type="x"/></manifest><spine><itemref idref="a"/><itemref idref="b"/></spine></package>"#;
        let files = [("OEBPS/content.opf", opf.as_bytes()), ("OEBPS/w1.xhtml", PAGE.as_bytes()), ("OEBPS/w2.xhtml", PAGE.as_bytes())];
        let mut zip = Vec::new();
        let mut central = Vec::new();
        for (name, data) in files {
            let offset = zip.len() as u32;
            let header = |sig: u32| sig.to_le_bytes().to_vec();
            let mut local = header(0x0403_4b50);
            local.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            local.extend_from_slice(&(data.len() as u32).to_le_bytes());
            local.extend_from_slice(&(data.len() as u32).to_le_bytes());
            local.extend_from_slice(&(name.len() as u16).to_le_bytes());
            local.extend_from_slice(&0u16.to_le_bytes());
            local.extend_from_slice(name.as_bytes());
            zip.extend_from_slice(&local);
            zip.extend_from_slice(data);
            let mut c = header(0x0201_4b50);
            c.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            c.extend_from_slice(&(data.len() as u32).to_le_bytes());
            c.extend_from_slice(&(data.len() as u32).to_le_bytes());
            c.extend_from_slice(&(name.len() as u16).to_le_bytes());
            c.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            c.extend_from_slice(&offset.to_le_bytes());
            c.extend_from_slice(name.as_bytes());
            central.extend_from_slice(&c);
        }
        let cd_offset = zip.len() as u32;
        zip.extend_from_slice(&central);
        zip.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        zip.extend_from_slice(&[0, 0, 0, 0, 3, 0, 3, 0]);
        zip.extend_from_slice(&(central.len() as u32).to_le_bytes());
        zip.extend_from_slice(&cd_offset.to_le_bytes());
        zip.extend_from_slice(&[0, 0]);
        let wb = read_epub(&zip, "202611", "E").unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(wb.weeks.len(), 2);
        // November 2026 starts on a Sunday: the first week is Monday 2 November.
        assert_eq!(wb.weeks[0].monday, "2026-11-02");
        assert_eq!(wb.weeks[1].monday, "2026-11-09");
    }

    #[test]
    fn issues_and_links() {
        assert_eq!(issues_for("2026-12-28"), ["202611", "202609"]);
        assert_eq!(issues_for("2027-01-04"), ["202701", "202611"]);
        assert_eq!(issue_from_file_name("C:/x/mwb_S_202701.epub"), Some(("202701".into(), "S".into())));
        let json = r#"{"files":{"E":{"EPUB":[{"file":{"url":"https://example.org/mwb_E_202611.epub"}}]}}}"#;
        assert_eq!(epub_url_from_links(json, "E").as_deref(), Some("https://example.org/mwb_E_202611.epub"));
    }

    /// Reads a real workbook when one was downloaded to `target/epub` (never in the repository:
    /// the content may not be distributed). Run with `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn reads_a_downloaded_workbook() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/epub");
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Some((issue, lang)) = issue_from_file_name(&name) else { continue };
        let Ok(bytes) = std::fs::read(e.path()) else { continue };
        let wb = read_epub(&bytes, &issue, &lang).unwrap_or_else(|e| panic!("{e}"));
        assert!(wb.weeks.len() >= 8, "{name}: {} weeks", wb.weeks.len());
        for w in &wb.weeks {
            assert!(w.songs.iter().all(Option::is_some), "{}: {:?}", w.label, w.songs);
            assert!(w.parts.iter().any(|p| p.kind == PartKind::BibleStudy), "{}", w.label);
            assert!(w.parts.iter().all(|p| p.minutes > 0), "{}: {:?}", w.label, w.parts.iter().map(|p| (p.number, p.minutes)).collect::<Vec<_>>());
            println!("{} {} {:?}", w.monday, w.reading, w.songs);
            for p in &w.parts {
                println!("  {}. {:?} {:?} {} ({} min)", p.number, p.section, p.kind, p.title, p.minutes);
            }
        }
        }
    }
}
