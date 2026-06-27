pub struct Engine;

impl Engine {
    pub fn new() -> Self {
        Self
    }

    pub fn package_name(&self) -> &'static str {
        "dandrum-engine-core"
    }

    pub fn is_frontend_independent(&self) -> bool {
        true
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_core_exposes_frontend_independent_identity() {
        let engine = Engine::new();

        assert_eq!(engine.package_name(), "dandrum-engine-core");
        assert!(engine.is_frontend_independent());
    }
}
