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

    // Module-library sub-codes
    pub const LIBRARY_UNKNOWN_MACRO: &str = "library.unknown_macro";
    pub const LIBRARY_PATH_ESCAPE: &str = "library.path_escape";
    pub const LIBRARY_MALFORMED_REFERENCE: &str = "library.malformed_reference";
    pub const LIBRARY_LATEST_UNAVAILABLE: &str = "library.latest_unavailable";
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