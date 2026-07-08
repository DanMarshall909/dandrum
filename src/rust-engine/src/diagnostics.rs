use std::fmt;

/// A structured diagnostic record with stable error code, severity, and optional
/// source location, module/port references, expected/actual values, and fix.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    error_code: String,
    severity: Severity,
    message: String,
    source_location: Option<SourceLocation>,
    module_id: Option<String>,
    port_name: Option<String>,
    expected: Option<String>,
    actual: Option<String>,
    suggested_fix: Option<String>,
}

/// Severity level for a diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Prevents rendering.
    Error,
    /// Render proceeds but behaviour may be unexpected.
    Warning,
    /// Advisory information.
    Info,
}

/// Source location in a YAML file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    file: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
}

/// Error code namespace prefixes.
pub mod error_codes {
    pub const LOADING: &str = "loading";
    pub const VALIDATION: &str = "validation";
    pub const GRAPH: &str = "graph";
    pub const SCRIPT: &str = "script";
    pub const RENDER: &str = "render";

    // Validation sub-codes
    pub const VALIDATION_TYPE_MISMATCH: &str = "validation.type_mismatch";
    pub const VALIDATION_MISSING_FIELD: &str = "validation.missing_field";
    pub const VALIDATION_INVALID_VALUE: &str = "validation.invalid_value";
    pub const VALIDATION_UNKNOWN_MODULE: &str = "validation.unknown_module";

    // Script sub-codes
    pub const SCRIPT_UNSUPPORTED_API: &str = "script.unsupported_api";
    pub const SCRIPT_UNSUPPORTED_PORT: &str = "script.unsupported_port";
    pub const SCRIPT_BUDGET_EXCEEDED: &str = "script.budget_exceeded";
    pub const SCRIPT_PARSE: &str = "script.parse";
    pub const SCRIPT_VALIDATION: &str = "script.validation";

    // Graph sub-codes
    pub const GRAPH_MISSING_MODULE: &str = "graph.missing_module";
    pub const GRAPH_MISSING_PORT: &str = "graph.missing_port";
    pub const GRAPH_INCORRECT_PORT_DIRECTION: &str = "graph.incorrect_port_direction";
    pub const GRAPH_INCOMPATIBLE_SIGNAL_TYPES: &str = "graph.incompatible_signal_types";
    pub const GRAPH_MULTIPLE_SOURCES: &str = "graph.multiple_sources";
    pub const GRAPH_CYCLE_DETECTED: &str = "graph.cycle_detected";
    pub const GRAPH_VOICE_TO_GLOBAL: &str = "graph.voice_to_global_direct_routing";
    pub const GRAPH_UNKNOWN_MODULE_TYPE: &str = "graph.unknown_module_type";
    pub const GRAPH_UNSUPPORTED_MODULE_TYPE: &str = "graph.unsupported_module_type";

    // Kernel sub-codes (unified graph kernel)
    pub const KERNEL_UNKNOWN_DEFINITION: &str = "kernel.unknown_definition";
    pub const KERNEL_MISSING_NODE: &str = "kernel.missing_node";
    pub const KERNEL_MISSING_PORT: &str = "kernel.missing_port";
    pub const KERNEL_INCORRECT_PORT_DIRECTION: &str = "kernel.incorrect_port_direction";
    pub const KERNEL_STATIC_PARAM_NOT_A_PORT: &str = "kernel.static_param_not_a_port";
    pub const KERNEL_MISSING_STATIC_ARGUMENT: &str = "kernel.missing_static_argument";
    pub const KERNEL_UNKNOWN_STATIC_ARGUMENT: &str = "kernel.unknown_static_argument";
    pub const KERNEL_STATIC_ARGUMENT_TYPE_MISMATCH: &str = "kernel.static_argument_type_mismatch";
    pub const KERNEL_STATIC_ARGUMENT_EXPRESSION: &str = "kernel.static_argument_expression";
    pub const KERNEL_UNKNOWN_STATIC_PARAM_REFERENCE: &str =
        "kernel.unknown_static_param_reference";
    pub const KERNEL_CHANNEL_COUNT_MISMATCH: &str = "kernel.channel_count_mismatch";
    pub const KERNEL_INCOMPATIBLE_SIGNAL_TYPES: &str = "kernel.incompatible_signal_types";
    pub const KERNEL_OVERRIDE_UNKNOWN_PORT: &str = "kernel.override_unknown_port";
    pub const KERNEL_CYCLE_WITHOUT_FEEDBACK_DELAY: &str =
        "kernel.cycle_without_feedback_delay";
    pub const KERNEL_RECURSIVE_DEFINITION: &str = "kernel.recursive_definition";
    pub const KERNEL_MAX_DEPTH_EXCEEDED: &str = "kernel.max_depth_exceeded";

    // Module-library sub-codes
    pub const LIBRARY_UNKNOWN_MACRO: &str = "library.unknown_macro";
    pub const LIBRARY_PATH_ESCAPE: &str = "library.path_escape";
    pub const LIBRARY_MALFORMED_REFERENCE: &str = "library.malformed_reference";
    pub const LIBRARY_LATEST_UNAVAILABLE: &str = "library.latest_unavailable";
    pub const LIBRARY_SEED_FAILED: &str = "library.seed_failed";
    pub const LIBRARY_PACKAGE_READ_FAILED: &str = "library.package_read_failed";
    pub const LIBRARY_PACKAGE_PARSE_FAILED: &str = "library.package_parse_failed";
    pub const LIBRARY_PACKAGE_NAME_MISMATCH: &str = "library.package_name_mismatch";
}

impl Diagnostic {
    pub fn new(
        error_code: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            error_code: error_code.into(),
            severity,
            message: message.into(),
            source_location: None,
            module_id: None,
            port_name: None,
            expected: None,
            actual: None,
            suggested_fix: None,
        }
    }

    pub fn with_source_location(mut self, location: SourceLocation) -> Self {
        self.source_location = Some(location);
        self
    }

    pub fn with_module_id(mut self, module_id: impl Into<String>) -> Self {
        self.module_id = Some(module_id.into());
        self
    }

    pub fn with_port_name(mut self, port_name: impl Into<String>) -> Self {
        self.port_name = Some(port_name.into());
        self
    }

    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    pub fn with_actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }

    pub fn with_suggested_fix(mut self, fix: impl Into<String>) -> Self {
        self.suggested_fix = Some(fix.into());
        self
    }

    pub fn error_code(&self) -> &str {
        &self.error_code
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source_location(&self) -> Option<&SourceLocation> {
        self.source_location.as_ref()
    }

    pub fn module_id(&self) -> Option<&str> {
        self.module_id.as_deref()
    }

    pub fn port_name(&self) -> Option<&str> {
        self.port_name.as_deref()
    }

    pub fn expected(&self) -> Option<&str> {
        self.expected.as_deref()
    }

    pub fn actual(&self) -> Option<&str> {
        self.actual.as_deref()
    }

    pub fn suggested_fix(&self) -> Option<&str> {
        self.suggested_fix.as_deref()
    }
}

impl SourceLocation {
    pub fn new(file: Option<String>, line: Option<usize>, column: Option<usize>) -> Self {
        Self { file, line, column }
    }

    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }

    pub fn line(&self) -> Option<usize> {
        self.line
    }

    pub fn column(&self) -> Option<usize> {
        self.column
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.severity, self.error_code, self.message
        )?;

        if let Some(loc) = &self.source_location {
            write!(f, " at ")?;
            if let Some(file) = &loc.file {
                write!(f, "{file}")?;
            }
            if let Some(line) = loc.line {
                write!(f, ":{line}")?;
                if let Some(col) = loc.column {
                    write!(f, ":{col}")?;
                }
            }
        }

        if let Some(module_id) = &self.module_id {
            write!(f, " module={module_id}")?;
            if let Some(port_name) = &self.port_name {
                write!(f, ".{port_name}")?;
            }
        }

        if let Some(expected) = &self.expected {
            write!(f, " expected={expected}")?;
        }
        if let Some(actual) = &self.actual {
            write!(f, " actual={actual}")?;
        }
        if let Some(fix) = &self.suggested_fix {
            write!(f, " suggestion={fix}")?;
        }

        Ok(())
    }
}

/// Collection of diagnostics with filtering helpers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.items.extend(diagnostics);
    }

    pub fn all(&self) -> &[Diagnostic] {
        &self.items
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter().filter(|d| d.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Warning)
    }

    pub fn infos(&self) -> impl Iterator<Item = &Diagnostic> {
        self.items.iter().filter(|d| d.severity == Severity::Info)
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, diagnostic) in self.items.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "- {diagnostic}")?;
        }
        Ok(())
    }
}

impl From<Diagnostic> for Diagnostics {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            items: vec![diagnostic],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_constructs_with_minimal_fields() {
        let d = Diagnostic::new("test.code", Severity::Error, "something went wrong");
        assert_eq!(d.error_code(), "test.code");
        assert_eq!(d.severity(), Severity::Error);
        assert_eq!(d.message(), "something went wrong");
        assert!(d.module_id().is_none());
        assert!(d.source_location().is_none());
    }

    #[test]
    fn diagnostic_constructs_with_all_fields() {
        let d = Diagnostic::new("graph.missing_module", Severity::Error, "missing module")
            .with_module_id("osc")
            .with_source_location(SourceLocation::new(
                Some("patch.yaml".to_string()),
                Some(10),
                Some(5),
            ))
            .with_suggested_fix("add an 'osc' module declaration");

        assert_eq!(d.module_id(), Some("osc"));
        let loc = d.source_location().unwrap();
        assert_eq!(loc.file(), Some("patch.yaml"));
        assert_eq!(loc.line(), Some(10));
        assert_eq!(loc.column(), Some(5));
        assert!(d.suggested_fix().is_some());
    }

    #[test]
    fn severity_display() {
        assert_eq!(Severity::Error.to_string(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Info.to_string(), "info");
    }

    #[test]
    fn diagnostic_display_includes_fields() {
        let d = Diagnostic::new("test.code", Severity::Warning, "something odd")
            .with_module_id("mod1")
            .with_expected("Audio")
            .with_actual("Control");
        let text = d.to_string();
        assert!(text.contains("test.code"));
        assert!(text.contains("warning"));
        assert!(text.contains("something odd"));
        assert!(text.contains("mod1"));
        assert!(text.contains("Audio"));
        assert!(text.contains("Control"));
    }

    #[test]
    fn diagnostics_collection_accumulates_multiple_items() {
        let mut collection = Diagnostics::new();
        assert!(collection.is_empty());
        assert_eq!(collection.len(), 0);

        collection.push(Diagnostic::new("a", Severity::Error, "first"));
        collection.push(Diagnostic::new("b", Severity::Warning, "second"));
        collection.push(Diagnostic::new("c", Severity::Info, "third"));

        assert!(!collection.is_empty());
        assert_eq!(collection.len(), 3);
        assert_eq!(collection.errors().count(), 1);
        assert_eq!(collection.warnings().count(), 1);
        assert_eq!(collection.infos().count(), 1);
    }

    #[test]
    fn diagnostics_collection_detects_errors() {
        let mut collection = Diagnostics::new();
        assert!(!collection.has_errors());

        collection.push(Diagnostic::new("w", Severity::Warning, "warn"));
        assert!(!collection.has_errors());

        collection.push(Diagnostic::new("e", Severity::Error, "err"));
        assert!(collection.has_errors());
    }

    #[test]
    fn diagnostics_collection_display_formats_all_items() {
        let mut collection = Diagnostics::new();
        collection.push(Diagnostic::new("a", Severity::Error, "first"));
        collection.push(Diagnostic::new("b", Severity::Warning, "second"));

        let text = collection.to_string();
        assert!(text.contains("first"));
        assert!(text.contains("second"));
        assert!(text.contains("- "));
    }

    #[test]
    fn error_code_constants_have_expected_namespace_format() {
        assert!(error_codes::GRAPH_MISSING_MODULE.starts_with("graph."));
        assert!(error_codes::VALIDATION_TYPE_MISMATCH.starts_with("validation."));
        assert!(error_codes::SCRIPT.starts_with("script"));
    }
}
