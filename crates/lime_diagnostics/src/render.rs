//! Rendering of [`Diagnostic`]s into human-readable text with source snippets.
//!
//! Output format (matching `docs/ARCHITECTURE.md`):
//! ```text
//! file:line:col: error[E0201]: <message>
//!    12 | tampilkan(umur)
//!       |           ^^^^ <label message>
//!    = note: <note>
//! ```
//! Colour is optional and off by default so output is deterministic in tests.

use crate::{Diagnostic, Severity};
use lime_span::SourceMap;

/// Render a single diagnostic against the given source map.
pub fn render(diag: &Diagnostic, sm: &SourceMap) -> String {
    let mut out = String::new();
    render_into(&mut out, diag, sm);
    out
}

/// Render every diagnostic, separated by blank lines, plus a summary line.
pub fn render_all(diags: &[Diagnostic], sm: &SourceMap) -> String {
    let mut out = String::new();
    for d in diags {
        render_into(&mut out, d, sm);
        out.push('\n');
    }
    let errors = diags.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = diags.iter().filter(|d| d.severity == Severity::Warning).count();
    if errors > 0 || warnings > 0 {
        out.push_str(&summary(errors, warnings));
        out.push('\n');
    }
    out
}

pub fn summary(errors: usize, warnings: usize) -> String {
    match (errors, warnings) {
        (0, 0) => "selesai tanpa masalah".to_string(),
        (e, 0) => format!("gagal: {e} kesalahan"),
        (0, w) => format!("selesai: {w} peringatan"),
        (e, w) => format!("gagal: {e} kesalahan, {w} peringatan"),
    }
}

fn render_into(out: &mut String, diag: &Diagnostic, sm: &SourceMap) {
    // Header: location comes from the primary label if present.
    let primary = diag.labels.iter().find(|l| l.primary).or_else(|| diag.labels.first());

    let header_loc = primary.map(|l| {
        let loc = sm.location(l.span);
        format!("{}:{}:{}: ", sm.name(l.span.file), loc.line, loc.column)
    });

    if let Some(h) = &header_loc {
        out.push_str(h);
    }
    match diag.code {
        Some(code) => out.push_str(&format!("{}[{}]: ", diag.severity.label(), code)),
        None => out.push_str(&format!("{}: ", diag.severity.label())),
    }
    out.push_str(&diag.message);
    out.push('\n');

    // Source snippets for each label, primary first.
    let mut labels: Vec<&crate::Label> = diag.labels.iter().collect();
    labels.sort_by_key(|l| (!l.primary, l.span.start));
    for label in labels {
        render_label(out, label, sm);
    }

    for note in &diag.notes {
        out.push_str(&format!("   = {}: {}\n", note.severity.label(), note.message));
    }
}

fn render_label(out: &mut String, label: &crate::Label, sm: &SourceMap) {
    let span = label.span;
    let loc = sm.location(span);
    let line_text = sm.line_text(span.file, loc.line);
    let gutter_num = loc.line.to_string();
    let gutter_w = gutter_num.len().max(4);

    // Source line.
    out.push_str(&format!("{:>w$} | {}\n", gutter_num, line_text, w = gutter_w));

    // Caret line. Column is 1-based over scalar values; compute caret width from
    // the number of scalar values the span covers, clamped to at least 1.
    let underline_char = if label.primary { '^' } else { '-' };
    let start_col = loc.column as usize; // 1-based
    let span_scalars = sm.snippet(span).chars().count().max(1);
    let pad = start_col.saturating_sub(1);
    let underline: String = std::iter::repeat(underline_char).take(span_scalars).collect();

    out.push_str(&format!(
        "{:>w$} | {}{}",
        "",
        " ".repeat(pad),
        underline,
        w = gutter_w
    ));
    if !label.message.is_empty() {
        out.push(' ');
        out.push_str(&label.message);
    }
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{codes, Diagnostic};
    use lime_span::{SourceMap, Span};

    #[test]
    fn renders_caret_under_span() {
        let mut sm = SourceMap::new();
        let f = sm.add("main.lime", "tampilkan(umur)");
        // 'umur' occupies bytes 10..14
        let sp = Span::new(f, 10, 14);
        let d = Diagnostic::error(codes::E_UNDEFINED_NAME, "variabel 'umur' belum didefinisikan")
            .primary(sp, "tidak ditemukan pada scope ini");
        let text = render(&d, &sm);
        assert!(text.contains("main.lime:1:11: error[E0201]:"), "got: {text}");
        assert!(text.contains("tampilkan(umur)"), "got: {text}");
        assert!(text.contains("^^^^ tidak ditemukan"), "got: {text}");
    }

    #[test]
    fn summary_wording() {
        assert_eq!(summary(0, 0), "selesai tanpa masalah");
        assert_eq!(summary(2, 0), "gagal: 2 kesalahan");
        assert_eq!(summary(1, 3), "gagal: 1 kesalahan, 3 peringatan");
    }
}
