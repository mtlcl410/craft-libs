//! Text contracts adapted from PdfCraft: an octal escape past `ÿ` in a shown string
//! (storytold/pdfcraft#7465).
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

fn helvetica_texts(encoding: &str, content: &[u8]) -> Vec<String> {
    let font = format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding {encoding} >>");
    glyph_texts(&font, "", content).into_iter().map(|text| text.unwrap_or_default()).collect()
}

/// PdfCraft patch 54 (storytold/pdfcraft#7465): an octal escape above `\377` in a shown string
/// keeps its low eight bits (ISO 32000-2 §7.3.4.2, Table 3: "high-order overflow shall be
/// ignored"). PDFBOX-142's form buttons write `(Abschlie\737en)`, `(L\766schen)` and
/// `(Pr\774fsumme)`; the escapes were dropped, so they read "Abschlieen", "Lschen" and
/// "Prfsumme". In WinAnsi 0o737 & 0xFF is ß, 0o766 & 0xFF ö and 0o774 & 0xFF ü.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn an_octal_escape_past_0o377_reads_its_low_eight_bits() {
    let text = |string: &str| helvetica_texts("/WinAnsiEncoding", format!("BT /F1 20 Tf 20 150 Td {string} Tj ET").as_bytes()).concat();
    assert_eq!(text(r"(Abschlie\737en)"), "Abschließen");
    assert_eq!(text(r"(L\766schen)"), "Löschen");
    assert_eq!(text(r"(Pr\774fsumme)"), "Prüfsumme");
    assert_eq!(text(r"(Abschlie\337en)"), "Abschließen", "control: the escape in range");
    assert_eq!(text(r"(\7377)"), "ß7", "three digits at most");
}
