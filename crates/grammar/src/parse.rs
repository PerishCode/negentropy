use crate::format::Rule;
use crate::lex::Token;
use crate::peg::{Pat, Quant};
use crate::{Cst, Kind, Span};
use std::collections::HashMap;

struct State<'a> {
    rules: &'a [Rule],
    index: HashMap<&'a str, usize>,
    toks: &'a [Token],
    src: &'a [u8],
}

pub fn run(rules: &[Rule], toks: &[Token], src: &[u8], end: usize) -> Cst {
    let mut index = HashMap::new();
    for (slot, rule) in rules.iter().enumerate() {
        index.insert(rule.name.as_str(), slot);
    }
    let state = State {
        rules,
        index,
        toks,
        src,
    };
    let kids = start(&state);
    Cst {
        kind: Kind::Root,
        span: Span { start: 0, end },
        kids,
    }
}

fn start(st: &State) -> Vec<Cst> {
    match st.index.get("unit") {
        Some(idx) => sweep(st, *idx),
        None => Vec::new(),
    }
}

fn sweep(st: &State, unit: usize) -> Vec<Cst> {
    let mut at = 0;
    let mut kids = Vec::new();
    let mut lost: Option<usize> = None;
    while at < st.toks.len() {
        at = tick(st, unit, at, &mut kids, &mut lost);
    }
    flush(st, &mut kids, lost, st.toks.len());
    kids
}

fn tick(
    st: &State,
    unit: usize,
    at: usize,
    kids: &mut Vec<Cst>,
    lost: &mut Option<usize>,
) -> usize {
    match rule(st, unit, at) {
        Some((next, mut got)) if next > at => {
            flush(st, kids, lost.take(), at);
            kids.append(&mut got);
            next
        }
        _ => {
            lost.get_or_insert(at);
            at + 1
        }
    }
}

fn flush(st: &State, kids: &mut Vec<Cst>, lost: Option<usize>, end: usize) {
    if let Some(lo) = lost {
        kids.push(loose(st, lo, end));
    }
}

fn loose(st: &State, lo: usize, end: usize) -> Cst {
    let start = st.toks[lo].start;
    let stop = st.toks[end - 1].end;
    Cst {
        kind: Kind::Loose,
        span: Span { start, end: stop },
        kids: Vec::new(),
    }
}

fn rule(st: &State, idx: usize, at: usize) -> Option<(usize, Vec<Cst>)> {
    let r = &st.rules[idx];
    let (next, kids) = expr(st, &r.pat, at)?;
    Some(wrap(st, r, at, next, kids))
}

fn wrap(st: &State, r: &Rule, at: usize, next: usize, kids: Vec<Cst>) -> (usize, Vec<Cst>) {
    match kind(r) {
        Some(kind) => (next, vec![node(kind, span(st, at, next), kids)]),
        None => (next, kids),
    }
}

fn kind(r: &Rule) -> Option<Kind> {
    match r.tag.as_deref() {
        Some("scope") => Some(Kind::Scope),
        Some("item") => Some(Kind::Item),
        Some("literal") => Some(Kind::Literal),
        Some("pattern") => Some(Kind::Pattern),
        Some("word") => Some(Kind::Word),
        _ => None,
    }
}

fn node(kind: Kind, span: Span, kids: Vec<Cst>) -> Cst {
    Cst { kind, span, kids }
}

fn span(st: &State, at: usize, next: usize) -> Span {
    let start = if at < st.toks.len() {
        st.toks[at].start
    } else {
        0
    };
    let end = if next > 0 && next <= st.toks.len() {
        st.toks[next - 1].end
    } else {
        start
    };
    Span { start, end }
}

fn expr(st: &State, pat: &Pat, at: usize) -> Option<(usize, Vec<Cst>)> {
    match pat {
        Pat::Lit(want) => lit(st, want, at),
        Pat::Name(name) => refer(st, name, at),
        Pat::Any => any(st, at),
        Pat::Seq(parts) => seq(st, parts, at),
        Pat::Alt(parts) => alt(st, parts, at),
        Pat::Rep(inner, quant) => rep(st, inner, quant, at),
        Pat::Not(inner) => not(st, inner, at),
        Pat::Class(..) => None,
    }
}

fn lit(st: &State, want: &[u8], at: usize) -> Option<(usize, Vec<Cst>)> {
    if at < st.toks.len() && text(st, at) == want {
        return Some((at + 1, Vec::new()));
    }
    None
}

fn text<'a>(st: &'a State, at: usize) -> &'a [u8] {
    let tok = &st.toks[at];
    &st.src[tok.start..tok.end]
}

fn refer(st: &State, name: &str, at: usize) -> Option<(usize, Vec<Cst>)> {
    match st.index.get(name) {
        Some(idx) => rule(st, *idx, at),
        None => term(st, name, at),
    }
}

fn term(st: &State, name: &str, at: usize) -> Option<(usize, Vec<Cst>)> {
    if at < st.toks.len() && st.toks[at].name == name {
        return Some((at + 1, Vec::new()));
    }
    None
}

fn any(st: &State, at: usize) -> Option<(usize, Vec<Cst>)> {
    if at < st.toks.len() {
        Some((at + 1, Vec::new()))
    } else {
        None
    }
}

fn seq(st: &State, parts: &[Pat], at: usize) -> Option<(usize, Vec<Cst>)> {
    let mut pos = at;
    let mut kids = Vec::new();
    for part in parts {
        let (next, mut got) = expr(st, part, pos)?;
        pos = next;
        kids.append(&mut got);
    }
    Some((pos, kids))
}

fn alt(st: &State, parts: &[Pat], at: usize) -> Option<(usize, Vec<Cst>)> {
    for part in parts {
        if let Some(hit) = expr(st, part, at) {
            return Some(hit);
        }
    }
    None
}

fn rep(st: &State, inner: &Pat, quant: &Quant, at: usize) -> Option<(usize, Vec<Cst>)> {
    match quant {
        Quant::Opt => Some(opt(st, inner, at)),
        Quant::Star => Some(star(st, inner, at)),
        Quant::Plus => plus(st, inner, at),
    }
}

fn opt(st: &State, inner: &Pat, at: usize) -> (usize, Vec<Cst>) {
    match expr(st, inner, at) {
        Some(hit) => hit,
        None => (at, Vec::new()),
    }
}

fn star(st: &State, inner: &Pat, at: usize) -> (usize, Vec<Cst>) {
    let mut pos = at;
    let mut kids = Vec::new();
    while let Some((next, mut got)) = expr(st, inner, pos) {
        if next == pos {
            break;
        }
        pos = next;
        kids.append(&mut got);
    }
    (pos, kids)
}

fn plus(st: &State, inner: &Pat, at: usize) -> Option<(usize, Vec<Cst>)> {
    let (end, kids) = star(st, inner, at);
    if end > at { Some((end, kids)) } else { None }
}

fn not(st: &State, inner: &Pat, at: usize) -> Option<(usize, Vec<Cst>)> {
    match expr(st, inner, at) {
        Some(_) => None,
        None => Some((at, Vec::new())),
    }
}
