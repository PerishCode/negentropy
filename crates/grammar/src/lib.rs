mod format;
mod lex;
mod parse;
mod peg;

use lex::{Slot, Token};
use std::sync::OnceLock;

pub struct Source {
    pub path: String,
    pub text: String,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Root,
    Scope,
    Item,
    Literal,
    Pattern,
    Comment,
    Loose,
    Word,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, out: &mut std::fmt::Formatter) -> std::fmt::Result {
        out.write_str(name(*self))
    }
}

fn name(kind: Kind) -> &'static str {
    match kind {
        Kind::Root => "root",
        Kind::Scope => "scope",
        Kind::Item => "item",
        Kind::Literal => "literal",
        Kind::Pattern => "pattern",
        Kind::Comment => "comment",
        Kind::Loose => "loose",
        Kind::Word => "word",
    }
}

pub struct Span {
    pub start: usize,
    pub end: usize,
}

pub struct Cst {
    pub kind: Kind,
    pub span: Span,
    pub kids: Vec<Cst>,
}

const RUST: &str = include_str!("../grammars/rust.g4");

fn rules() -> &'static (Vec<format::Rule>, Vec<format::Rule>) {
    static CELL: OnceLock<(Vec<format::Rule>, Vec<format::Rule>)> = OnceLock::new();
    CELL.get_or_init(|| split(format::load(RUST)))
}

fn split(all: Vec<format::Rule>) -> (Vec<format::Rule>, Vec<format::Rule>) {
    let mut lexers = Vec::new();
    let mut parsers = Vec::new();
    for rule in all {
        route(rule, &mut lexers, &mut parsers);
    }
    (lexers, parsers)
}

fn route(rule: format::Rule, lexers: &mut Vec<format::Rule>, parsers: &mut Vec<format::Rule>) {
    if upper(&rule.name) {
        lexers.push(rule);
        return;
    }
    parsers.push(rule);
}

fn upper(name: &str) -> bool {
    name.chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
}

pub fn parse(source: &Source) -> Cst {
    if source.path.ends_with(".md") {
        return heads(source);
    }
    if source.path.ends_with(".rs") || source.path.ends_with(".ts") {
        return braces(source);
    }
    bare(source)
}

fn bare(source: &Source) -> Cst {
    Cst {
        kind: Kind::Root,
        span: whole(source),
        kids: Vec::new(),
    }
}

fn whole(source: &Source) -> Span {
    Span {
        start: 0,
        end: source.text.len(),
    }
}

fn braces(source: &Source) -> Cst {
    let bytes = source.text.as_bytes();
    let (lexers, parsers) = rules();
    let (sig, comments) = sift(lex::lex(lexers, bytes));
    let mut root = parse::run(parsers, &sig, bytes, source.text.len());
    for span in comments {
        root.kids.push(note(span));
    }
    root
}

fn sift(tokens: Vec<Token>) -> (Vec<Token>, Vec<Span>) {
    let mut sig = Vec::new();
    let mut comments = Vec::new();
    for token in tokens {
        sort(token, &mut sig, &mut comments);
    }
    (sig, comments)
}

fn sort(token: Token, sig: &mut Vec<Token>, comments: &mut Vec<Span>) {
    if matches!(token.kind, Slot::Comment) {
        comments.push(Span {
            start: token.start,
            end: token.end,
        });
        return;
    }
    sig.push(token);
}

fn note(span: Span) -> Cst {
    Cst {
        kind: Kind::Comment,
        span,
        kids: Vec::new(),
    }
}

struct Head {
    level: usize,
    start: usize,
    kids: Vec<Cst>,
}

fn heads(source: &Source) -> Cst {
    let bytes = source.text.as_bytes();
    let mut stack = vec![Head {
        level: 0,
        start: 0,
        kids: Vec::new(),
    }];
    let mut at = 0;
    while at < bytes.len() {
        at = row(bytes, at, &mut stack);
    }
    settle(&mut stack, 1, source.text.len());
    let root = stack.pop().unwrap();
    Cst {
        kind: Kind::Root,
        span: whole(source),
        kids: root.kids,
    }
}

fn row(bytes: &[u8], at: usize, stack: &mut Vec<Head>) -> usize {
    let end = eol(bytes, at);
    let level = hashes(bytes, at, end);
    if level > 0 {
        raise(stack, level, at);
    }
    step(bytes, end)
}

fn raise(stack: &mut Vec<Head>, level: usize, start: usize) {
    settle(stack, level, start);
    stack.push(Head {
        level,
        start,
        kids: Vec::new(),
    });
}

fn settle(stack: &mut Vec<Head>, level: usize, at: usize) {
    while deep(stack, level) {
        close(stack, at);
    }
}

fn deep(stack: &[Head], level: usize) -> bool {
    stack.len() > 1 && stack.last().unwrap().level >= level
}

fn close(stack: &mut Vec<Head>, at: usize) {
    let done = stack.pop().unwrap();
    let node = Cst {
        kind: Kind::Scope,
        span: Span {
            start: done.start,
            end: at,
        },
        kids: done.kids,
    };
    stack.last_mut().unwrap().kids.push(node);
}

fn hashes(bytes: &[u8], at: usize, end: usize) -> usize {
    let mut run = at;
    while run < end && bytes[run] == b'#' {
        run += 1;
    }
    marked(bytes, run, end, run - at)
}

fn marked(bytes: &[u8], run: usize, end: usize, count: usize) -> usize {
    if count > 0 && run < end && bytes[run] == b' ' {
        count
    } else {
        0
    }
}

fn eol(bytes: &[u8], at: usize) -> usize {
    let mut end = at;
    while end < bytes.len() && bytes[end] != b'\n' {
        end += 1;
    }
    end
}

fn step(bytes: &[u8], end: usize) -> usize {
    if end < bytes.len() { end + 1 } else { end }
}
