use once_cell::sync::OnceCell;

mod golang;

#[derive(Debug)]
pub struct LanguageSpec {
    /// unique id for the language, the full id should be lowercase
    pub lang_id: &'static str,
    /// we want to avoid the lifetime, so we use a function that returns the language
    pub lang: fn() -> tree_sitter::Language,
    /// tree-sitter's query that resolves scopes, defs & refs
    /// the scope should be marked as `@scope` in the query
    /// the definition should be marked as `@definition` in the query
    /// the references don't need to be marked
    pub local_query: MemoizedQuery,
}

#[derive(Debug)]
pub struct MemoizedQuery {
    slot: OnceCell<tree_sitter::Query>,
    scope_query: &'static str,
}

impl MemoizedQuery {
    pub const fn new(scope_query: &'static str) -> Self {
        Self {
            slot: OnceCell::new(),
            scope_query,
        }
    }

    /// Get a reference to the relevant tree sitter compiled query.
    ///
    /// This method compiles the query if it has not already been compiled.
    pub fn query(
        &self,
        grammar: fn() -> tree_sitter::Language,
    ) -> Result<&tree_sitter::Query, tree_sitter::QueryError> {
        self.slot
            .get_or_try_init(|| tree_sitter::Query::new(grammar(), self.scope_query))
    }
}

pub static LANGUAGES: &[&LanguageSpec] = &[&golang::GO];