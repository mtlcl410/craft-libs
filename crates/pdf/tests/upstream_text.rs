//! Text contracts adapted from PdfCraft: a CID font whose `/Encoding` names a predefined CMap of its collection (storytold/pdfcraft#7513).
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

/// PdfCraft patch 116 (storytold/pdfcraft#7513): a Type 0 font whose `/Encoding` names a CMap
/// Adobe publishes for a supplement of its collections (`/Adobe-Japan1-2`), which ISO 32000-2
/// §9.7.5.2 doesn't list in Table 116, reads two-byte codes and its `/ToUnicode` (§9.10.2). The
/// font was dropped for the Helvetica substitute, which read the codes one byte at a time and
/// never looked at its `/ToUnicode`.
#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn an_adobe_collection_cid_cmap_name_reads_two_byte_codes() {
    let to_unicode = "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CMapName /T def /CMapType 2 def \
                      1 begincodespacerange <0000> <FFFF> endcodespacerange \
                      3 beginbfchar <0022> <0041> <0465> <4E9C> <034B> <3042> endbfchar endcmap CMapName currentdict /CMap defineresource pop end end";
    let texts = |encoding: &str| {
        let font = format!("<< /Type /Font /Subtype /Type0 /BaseFont /TestJapan1 /Encoding /{encoding} /DescendantFonts [6 0 R] /ToUnicode 8 0 R >>");
        let extra = format!(
            "6 0 obj << /Type /Font /Subtype /CIDFontType2 /BaseFont /TestJapan1 /CIDSystemInfo << /Registry (Adobe) /Ordering (Japan1) /Supplement 2 >> \
             /FontDescriptor 7 0 R /DW 1000 >> endobj\n\
             7 0 obj << /Type /FontDescriptor /FontName /TestJapan1 /Flags 4 /FontBBox [0 -120 1000 880] /ItalicAngle 0 /Ascent 880 /Descent -120 /CapHeight 700 /StemV 80 >> endobj\n\
             8 0 obj << /Length {} >> stream\n{to_unicode}\nendstream endobj\n",
            to_unicode.len() + 1
        );
        glyph_texts(&font, &extra, b"BT /F1 20 Tf 20 150 Td <00220465034B> Tj ET").into_iter().map(Option::unwrap_or_default).collect::<Vec<_>>()
    };
    assert_eq!(texts("Identity-H"), ["A", "亜", "あ"], "control: Identity-H");
    assert_eq!(texts("Adobe-Japan1-2"), ["A", "亜", "あ"]);
}
