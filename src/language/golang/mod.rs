use super::{LanguageSpec, MemoizedQuery};

pub static GO: LanguageSpec = LanguageSpec {
    lang_id: "go",
    lang: tree_sitter_go::language,
    local_query: MemoizedQuery::new(include_str!("./local_query.scm")),
};
