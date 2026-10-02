pub mod codes;
pub mod render;

pub use codes::Code;

use exotickik_span::Span;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Label {
    pub span: Span,
    pub message: String,

    pub primary: bool,
}

#[derive(Clone, Debug)]
pub struct SubNote {
    pub severity: Severity,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<Code>,
    pub message: String,
    pub labels: Vec<Label>,
    pub notes: Vec<SubNote>,
}

impl Diagnostic {
    pub fn error(code: Code, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            severity: Severity::Error,
            code: Some(code),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(code: Code, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            severity: Severity::Warning,
            code: Some(code),
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn primary(mut self, span: Span, message: impl Into<String>) -> Diagnostic {
        self.labels.push(Label { span, message: message.into(), primary: true });
        self
    }

    pub fn secondary(mut self, span: Span, message: impl Into<String>) -> Diagnostic {
        self.labels.push(Label { span, message: message.into(), primary: false });
        self
    }

    pub fn note(mut self, message: impl Into<String>) -> Diagnostic {
        self.notes.push(SubNote { severity: Severity::Note, message: message.into() });
        self
    }

    pub fn help(mut self, message: impl Into<String>) -> Diagnostic {
        self.notes.push(SubNote { severity: Severity::Help, message: message.into() });
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

#[derive(Default)]
pub struct DiagnosticEngine {
    diagnostics: Vec<Diagnostic>,
    error_count: usize,
}

impl DiagnosticEngine {
    pub fn new() -> DiagnosticEngine {
        DiagnosticEngine { diagnostics: Vec::new(), error_count: 0 }
    }

    pub fn emit(&mut self, diag: Diagnostic) {
        if diag.is_error() {
            self.error_count += 1;
        }
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    pub fn error_count(&self) -> usize {
        self.error_count
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn take(&mut self) -> Vec<Diagnostic> {
        self.error_count = 0;
        std::mem::take(&mut self.diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exotickik_span::{FileId, Span};

    #[test]
    fn engine_counts_errors_and_warnings() {
        let mut eng = DiagnosticEngine::new();
        let sp = Span::new(FileId(0), 0, 1);
        eng.emit(Diagnostic::error(codes::E_UNDEFINED_NAME, "x").primary(sp, "here"));
        eng.emit(Diagnostic::warning(codes::W_UNREACHABLE, "dead"));
        assert!(eng.has_errors());
        assert_eq!(eng.error_count(), 1);
        assert_eq!(eng.warning_count(), 1);
    }
}
