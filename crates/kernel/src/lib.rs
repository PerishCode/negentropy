pub mod config;

use config::Config;
use grammar::{Cst, Kind, Source, Span};

pub struct Node {
    pub kind: Kind,
    pub depth: usize,
    pub span: Span,
    pub kids: Vec<Node>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Class {
    Fault,
    Blind,
    Debt,
}

pub struct Finding {
    pub law: String,
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub note: String,
    pub class: Class,
}

pub fn structure(source: &Source) -> Node {
    lift(&grammar::parse(source), 0)
}

fn lift(cst: &Cst, depth: usize) -> Node {
    let here = depth + scoped(cst.kind);
    Node {
        kind: cst.kind,
        depth: here,
        span: Span {
            start: cst.span.start,
            end: cst.span.end,
        },
        kids: cst.kids.iter().map(|kid| lift(kid, here)).collect(),
    }
}

fn scoped(kind: Kind) -> usize {
    if kind == Kind::Scope { 1 } else { 0 }
}

pub fn check(source: &Source, node: &Node, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    laws(source, node, config, &mut findings);
    findings
}

fn laws(source: &Source, node: &Node, config: &Config, findings: &mut Vec<Finding>) {
    block(source, node, config, findings);
    comment(source, node, config, findings);
    coverage(source, node, findings);
    single(source, node, config, findings);
    for kid in &node.kids {
        laws(source, kid, config, findings);
    }
}

fn coverage(source: &Source, node: &Node, findings: &mut Vec<Finding>) {
    if node.kind == Kind::Loose {
        findings.push(mark(
            source,
            node.span.start,
            "coverage",
            "unparsed region",
            Class::Blind,
        ));
    }
}

fn single(source: &Source, node: &Node, config: &Config, findings: &mut Vec<Finding>) {
    let name = word(source, node);
    if node.kind == Kind::Word
        && config.file.word.single
        && compound(name)
        && !registered(name, &config.vocab)
        && !config.exempt(&source.path, "word")
    {
        findings.push(mark(source, node.span.start, "word", name, Class::Debt));
    }
}

fn registered(name: &str, vocab: &[String]) -> bool {
    vocab.iter().any(|entry| entry == name)
}

fn word<'a>(source: &'a Source, node: &Node) -> &'a str {
    source
        .text
        .get(node.span.start..node.span.end)
        .unwrap_or("")
}

fn compound(name: &str) -> bool {
    segments(name) > 1
}

fn segments(name: &str) -> usize {
    let mut count = 0;
    let mut prev: Option<char> = None;
    for glyph in name.chars() {
        count += grow(prev, glyph);
        prev = seat(glyph);
    }
    count
}

fn grow(prev: Option<char>, glyph: char) -> usize {
    if glyph == '_' {
        return 0;
    }
    match prev {
        None => 1,
        Some(before) => usize::from(glyph.is_uppercase() && !before.is_uppercase()),
    }
}

fn seat(glyph: char) -> Option<char> {
    if glyph == '_' { None } else { Some(glyph) }
}

fn block(source: &Source, node: &Node, config: &Config, findings: &mut Vec<Finding>) {
    if node.depth > config.file.limit.block
        && node.kind == Kind::Scope
        && !config.exempt(&source.path, "block")
    {
        findings.push(mark(
            source,
            node.span.start,
            "block",
            "depth over limit",
            Class::Fault,
        ));
    }
}

fn comment(source: &Source, node: &Node, config: &Config, findings: &mut Vec<Finding>) {
    if node.kind == Kind::Comment
        && !config.file.comment.allow
        && !config.exempt(&source.path, "comment")
    {
        findings.push(mark(
            source,
            node.span.start,
            "comment",
            "denied by default",
            Class::Fault,
        ));
    }
}

fn mark(source: &Source, at: usize, law: &str, note: &str, class: Class) -> Finding {
    let (line, col) = place(&source.text, at);
    Finding {
        law: law.to_string(),
        path: source.path.clone(),
        line,
        col,
        note: note.to_string(),
        class,
    }
}

fn place(text: &str, at: usize) -> (usize, usize) {
    let at = at.min(text.len());
    let head = &text[..at];
    let line = head.matches('\n').count() + 1;
    let start = head.rfind('\n').map(|nl| nl + 1).unwrap_or(0);
    (line, at - start + 1)
}

pub fn pathdepth(path: &str, config: &Config) -> Option<Finding> {
    if depthof(path) > config.file.limit.path && !config.exempt(path, "path") {
        return Some(pathmark(path));
    }
    None
}

fn depthof(path: &str) -> usize {
    path.split('/').count().saturating_sub(1)
}

fn pathmark(path: &str) -> Finding {
    Finding {
        law: "path".to_string(),
        path: path.to_string(),
        line: 0,
        col: 0,
        note: "depth over three".to_string(),
        class: Class::Fault,
    }
}

#[cfg(test)]
mod tests {
    use grammar::Source;

    fn laws(text: &str) -> Vec<String> {
        let source = Source {
            path: "t.rs".to_string(),
            text: text.to_string(),
        };
        let node = super::structure(&source);
        let config = super::config::Config {
            file: Default::default(),
            vocab: Vec::new(),
        };
        super::check(&source, &node, &config)
            .iter()
            .map(|f| f.law.clone())
            .collect()
    }

    #[test]
    fn depth() {
        assert!(
            laws("fn f() { if a { for b in c { while d { let x = 1; } } } }")
                .contains(&"block".to_string())
        );
    }

    #[test]
    fn literal() {
        assert!(
            !laws("fn f() { if a { for b in c { let v = S { x: 1 }; } } }")
                .contains(&"block".to_string())
        );
    }

    #[test]
    fn coverage() {
        assert!(
            !laws("use a::{B, C};\npub fn f() -> u32 { 1 }\ntrait T { fn m(&self); }")
                .contains(&"coverage".to_string())
        );
    }

    #[test]
    fn comment() {
        assert!(laws("fn f() {}\n// note").contains(&"comment".to_string()));
    }

    #[test]
    fn compound() {
        assert!(laws("fn read_file() {}").contains(&"word".to_string()));
    }

    #[test]
    fn reference() {
        assert!(!laws("fn read() { write_all(); }").contains(&"word".to_string()));
    }

    #[test]
    fn binding() {
        assert!(laws("fn f() { let bad_name = 1; }").contains(&"word".to_string()));
        assert!(
            !laws("fn f() { let good = 1; if let Some(x) = y {} }").contains(&"word".to_string())
        );
    }

    #[test]
    fn field() {
        assert!(laws("struct S { bad_field: u32 }").contains(&"word".to_string()));
        assert!(!laws("struct S { good: u32 }\nenum E { Ok, Bad }").contains(&"word".to_string()));
    }
}
