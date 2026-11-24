//! Text contracts adapted from PdfCraft: Type 3 fonts naming their codes in hexadecimal (storytold/pdfcraft#7710).
#![cfg(feature = "interpret")]
#![allow(clippy::unwrap_used, clippy::panic)]

use craft_pdf::interpret::{
    BlendMode, ClipPath, Context, Device, GlyphDrawMode, Image, InterpreterCache, InterpreterSettings, Paint, PathDrawMode, SoftMask, TransformExt,
    font::Glyph,
};
use hayro_cmap::BfString;
use kurbo::{Affine, BezPath, Rect};

/// The text of each glyph drawn, `None` where the font gives none.
#[derive(Default)]
struct TextDevice {
    glyphs: Vec<Option<String>>,
}

impl Device<'_> for TextDevice {
    fn set_soft_mask(&mut self, _: Option<SoftMask<'_>>) {}
    fn set_blend_mode(&mut self, _: BlendMode) {}
    fn draw_path(&mut self, _: &BezPath, _: Affine, _: &Paint<'_>, _: &PathDrawMode) {}
    fn push_clip_path(&mut self, _: &ClipPath) {}
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'_>>, _: BlendMode) {}
    fn draw_glyph(&mut self, glyph: &Glyph<'_>, _: Affine, _: Affine, _: &Paint<'_>, _: &GlyphDrawMode) {
        self.glyphs.push(glyph.as_unicode().map(|text| match text {
            BfString::Char(c) => c.to_string(),
            BfString::String(s) => s,
        }));
    }
    fn draw_image(&mut self, _: Image<'_, '_>, _: Affine) {}
    fn pop_clip_path(&mut self) {}
    fn pop_transparency_group(&mut self) {}
}

/// The glyphs a one-page PDF draws with its font `/F1`, object 5; `extra` holds further objects.
fn drawn_glyphs(font: &str, extra: &str, content: &[u8]) -> TextDevice {
    let mut pdf = format!(
        "%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
         2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
         3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n\
         4 0 obj << /Length {} >> stream\n",
        content.len()
    )
    .into_bytes();
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(format!("\nendstream endobj\n5 0 obj {font} endobj\n{extra}trailer << /Root 1 0 R >>\n%%EOF\n").as_bytes());
    let pdf = craft_pdf::open_slice(&pdf, None).unwrap();
    let page = &pdf.pages()[0];
    let cache = InterpreterCache::new();
    let mut context =
        Context::new(page.initial_transform(true).to_kurbo(), Rect::new(0.0, 0.0, 300.0, 300.0), &cache, page.xref(), InterpreterSettings::default());
    let mut device = TextDevice::default();
    craft_pdf::interpret_page(page, &mut context, &mut device).unwrap();
    device
}

fn glyph_texts(font: &str, extra: &str, content: &[u8]) -> Vec<Option<String>> {
    drawn_glyphs(font, extra, content).glyphs
}

/// The text a Type 3 font whose `/Differences` gives `names` (code, glyph name), every glyph a
/// filled box, reads from `codes`.
fn type3_text(names: &[(u8, String)], codes: &[u8]) -> String {
    let differences: String = names.iter().map(|(code, name)| format!("{code} /{name} ")).collect();
    let procs: String = names.iter().map(|(_, name)| format!("/{name} 6 0 R ")).collect();
    let font = format!(
        "<< /Type /Font /Subtype /Type3 /FontBBox [0 0 600 700] /FontMatrix [0.001 0 0 0.001 0 0] /FirstChar 0 /LastChar 255 \
         /Widths [{}] /Encoding << /Type /Encoding /Differences [{differences}] >> /CharProcs << {procs}>> >>",
        vec!["600"; 256].join(" ")
    );
    let glyph = "600 0 0 0 500 700 d1 0 0 500 700 re f";
    let extra = format!("6 0 obj << /Length {} >> stream\n{glyph}\nendstream endobj\n", glyph.len());
    let mut content = b"BT /F1 20 Tf 20 150 Td <".to_vec();
    content.extend(codes.iter().flat_map(|b| format!("{b:02X}").into_bytes()));
    content.extend_from_slice(b"> Tj ET");
    glyph_texts(&font, &extra, &content).into_iter().map(Option::unwrap_or_default).collect()
}

/// PdfCraft storytold/pdfcraft#7710: a Type 3 font naming all 256 codes in hexadecimal (`/x0` …
/// `/xff`, TeX's dvips fonts, PDFBOX-728) has numbered names at control codes, but names every
/// one of them, 0 through 31, in hexadecimal, which no decimal counter or counter from code 1
/// does: "für Weierstraß" reads in full, codes from 128 up included. A numbered name at its own
/// code reads as the code's Latin-1 character (§9.10.2 leaves such a font without a mapping).
/// A decimal counter `/C1` … `/C31` beside `/C174` (`/C10` at 10 reads 16 in hexadecimal), the
/// same names from `/x1` (code 0 left out) and a hexadecimal subset keeping a few glyphs at their
/// own control codes (`/G0A` at 10, `/G0D` at 13, `/GD9` at 0xD9) still read nothing from 128 up.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn a_type3_font_naming_its_codes_x0_to_xff_reads_them_from_128_up() {
    let names = |pairs: &[(u8, &str)]| pairs.iter().map(|&(code, n)| (code, n.to_string())).collect::<Vec<_>>();
    let hex_names: Vec<_> = (0..=255u8).map(|code| (code, format!("x{code:x}"))).collect();
    assert_eq!(type3_text(&hex_names, b"f\xfcr Weierstra\xdf"), "für Weierstraß", "/x0 … /xff");
    let mut counter_to_31: Vec<_> = (1..=31u8).map(|code| (code, format!("C{code}"))).collect();
    counter_to_31.push((174, "C174".to_string()));
    assert_eq!(type3_text(&counter_to_31, b"\xae"), "", "a counter /C1 … /C31 beside /C174 at 174");
    assert_eq!(type3_text(&hex_names[1..], b"f\xfcr Weierstra\xdf"), "fr Weierstra", "/x1 … /xff, code 0 left out");
    let subset = names(&[(10, "G0A"), (13, "G0D"), (174, "GAE"), (217, "GD9")]);
    assert_eq!(type3_text(&subset, b"\xae\xd9"), "", "/G0A at 10 and /G0D at 13 beside /GD9 at 217");
}

/// PdfCraft storytold/pdfcraft#7710 (review): the dvips hexadecimal block is a Type 3 rule. A
/// Type 1 font without `/ToUnicode` whose `/Differences` counts its glyphs in order of use in
/// hexadecimal from code 0 (`/zz0` … `/zzae`) fills the control block the same way, but its
/// `/zzae` at 0xAE reads as nothing, as it did before the Type 3 rule; the same names in a Type 3
/// font read `®`.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn a_type1_font_counting_its_glyphs_zz0_to_zzae_still_reads_nothing_from_them() {
    let counter: Vec<_> = (0..=0xaeu8).map(|code| (code, format!("zz{code:x}"))).collect();
    let differences: String = counter.iter().map(|(code, name)| format!("{code} /{name} ")).collect();
    let font = format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /Type /Encoding /Differences [{differences}] >> >>");
    let content = b"BT /F1 20 Tf 20 150 Td <AE> Tj ET";
    assert_eq!(glyph_texts(&font, "", content), [None], "Type 1 /zz0 … /zzae, code 0xAE");
    assert_eq!(type3_text(&counter, b"\xae"), "\u{ae}", "Type 3 /zz0 … /zzae, code 0xAE");
}
