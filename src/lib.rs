mod error;
mod language;
use std::collections::HashMap;

pub use language::*;
use tree_sitter::QueryCursor;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub def: tree_sitter::Range,
    pub refs: Vec<tree_sitter::Range>,
}

pub fn code_intel(src: &[u8], lang_id: &str) -> Result<Vec<Symbol>, error::CodeIntelError> {
    let lang = LANGUAGES
        .iter()
        .find(|l| l.lang_id == lang_id)
        .ok_or(error::CodeIntelError::UnsupportedLanguage)?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language((lang.lang)())
        .map_err(|e| error::CodeIntelError::LanguageError(e))?;

    let tree = parser
        .parse(src, None)
        .ok_or(error::CodeIntelError::ParseError)?;

    let query = lang
        .local_query
        .query(lang.lang)
        .map_err(|e| error::CodeIntelError::QueryError(e))?;

    let root_node = tree.root_node();

    let mut scope_idx = None;
    let mut definition_idx = None;

    for (i, name) in query.capture_names().iter().enumerate() {
        let i = i as u32;
        if name == "scope" {
            scope_idx = Some(i);
        } else if name == "definition" {
            definition_idx = Some(i);
        }
    }

    let mut cursor = QueryCursor::new();
    let captures = cursor.captures(query, root_node, src);

    // collect all captures
    // the value of the map is a map from captured identifier's name to their range
    let capture_map = captures.fold(
        HashMap::<_, Vec<_>>::new(),
        |mut map, (match_, capture_idx)| {
            let capture = match_.captures[capture_idx];
            // let range = capture.node.range();
            map.entry(capture.index).or_default().push(capture.node);
            map
        },
    );

    // 1. collect all the scopes, which are marked as `@scope`` in the query
    let mut scopes: HashMap<usize, HashMap<&[u8], Symbol>> = HashMap::new();
    if let Some(idx) = scope_idx {
        if let Some(captures) = capture_map.get(&idx) {
            for node in captures {
                scopes.insert(node.id(), HashMap::new());
            }
        }
    }

    // 2. collect all the definitions, which are marked as `@definition`` in the query
    if let Some(idx) = definition_idx {
        if let Some(captures) = capture_map.get(&idx) {
            for node in captures {
                let mut scope = Some(node.clone());
                while scope.is_some() {
                    let n = scope.as_ref().unwrap();
                    // has found the scope
                    if scopes.contains_key(&n.id()) {
                        let content = &src[node.range().start_byte..node.range().end_byte];
                        scopes.entry(n.id()).or_default().insert(
                            content,
                            Symbol {
                                def: node.range(),
                                refs: vec![],
                            },
                        );
                        break;
                    }

                    scope = n.parent();
                }
            }
        }
    }

    // 3. walk the tree, and find the nearest scope for each reference
    walk_tree_with(tree.root_node(), &mut |cur| {
        if cur.kind().contains("identifier") {
            let content = &src[cur.range().start_byte..cur.range().end_byte];
            let mut scope = Some(cur.clone());
            while scope.is_some() {
                let scope_tmp = scope.as_ref().unwrap();
                if scopes.contains_key(&scope_tmp.id()) {
                    if let Some(defs) = scopes.get_mut(&scope_tmp.id()) {
                        if let Some(symbol) = defs.get_mut(&content) {
                            if cur.range() != symbol.def {
                                symbol.refs.push(cur.range());
                            }
                        }
                    }
                    break;
                }
                scope = scope_tmp.parent();
            }
        }
    });

    let symbols = scopes
        .values()
        .flat_map(|symbols| symbols.values().cloned())
        .collect::<Vec<_>>();

    Ok(symbols)
}

fn walk_tree_with(node: tree_sitter::Node, f: &mut dyn FnMut(&tree_sitter::Node)) {
    f(&node);
    let mut cursor = node.walk();
    for n in node.children(&mut cursor) {
        walk_tree_with(n, f);
    }
}

#[cfg(test)]
mod tests {
    use crate::code_intel;

    #[test]
    fn test() {
        let src = r#"
        func f1(p int) {
            //  v f1.x def
            //  v f1.x ref
            x := 1
            y := 2

            fmt.Println(p)
            fmt.Println(x)
            fmt.Println(x)
        }
        "#
        .as_bytes();
        let symbols = code_intel(src, "go");
        assert!(symbols.is_ok());
        let symbols = symbols.unwrap();
        assert_eq!(symbols.len(), 3);
        for symbol in symbols {
            println!("symbol:{:?}, refs: {:?}", symbol.def, symbol.refs);
        }
    }
}
