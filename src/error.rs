#[derive(Debug)]
pub enum CodeIntelError {
    QueryError(tree_sitter::QueryError),
    UnsupportedLanguage,
    LanguageError(tree_sitter::LanguageError),
    ParseError,
}
