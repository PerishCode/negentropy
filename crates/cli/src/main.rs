use clap::Parser;
use std::fmt::Write;
use std::fs;
use std::path::Path;

#[derive(Parser)]
struct Cli {
    #[arg(default_value = ".")]
    root: String,
    #[arg(long)]
    strict: bool,
    #[arg(long)]
    debt: bool,
    #[arg(long)]
    json: bool,
}

fn main() {
    let cli = Cli::parse();
    let base = Path::new(&cli.root);
    let mut files = Vec::new();
    walk(base, &mut files);
    if cli.json {
        shape(base, &files);
        return;
    }
    let config = kernel::config::load(base);
    if report(&scan(base, &files, &config), cli.strict, cli.debt) {
        std::process::exit(1);
    }
}

fn shape(base: &Path, files: &[String]) {
    for path in files {
        draw(base, path);
    }
}

fn draw(base: &Path, path: &str) {
    let rel = relative(base, path);
    let text = fs::read_to_string(path).unwrap_or_default();
    let source = grammar::Source {
        path: rel.clone(),
        text,
    };
    let node = kernel::structure(&source);
    let mut out = String::new();
    tree(&node, &mut out);
    println!("{{\"path\":\"{}\",\"tree\":{}}}", escape(&rel), out);
}

fn tree(node: &kernel::Node, out: &mut String) {
    let _ = write!(
        out,
        "{{\"kind\":\"{}\",\"depth\":{},\"start\":{},\"end\":{},\"kids\":[",
        node.kind, node.depth, node.span.start, node.span.end
    );
    for (index, kid) in node.kids.iter().enumerate() {
        comma(out, index);
        tree(kid, out);
    }
    out.push_str("]}");
}

fn comma(out: &mut String, index: usize) {
    if index > 0 {
        out.push(',');
    }
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn walk(dir: &Path, files: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        visit(&entry.path(), files);
    }
}

fn visit(path: &Path, files: &mut Vec<String>) {
    if skip(path) {
        return;
    }
    if path.is_dir() {
        walk(path, files);
        return;
    }
    if wanted(path) {
        files.push(path.to_str().unwrap_or("").to_string());
    }
}

fn skip(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|part| part.to_str())
        .unwrap_or("");
    matches!(
        name,
        "target" | ".git" | ".task" | ".runseal" | ".github" | "node_modules" | ".local"
    )
}

fn wanted(path: &Path) -> bool {
    let name = path.to_str().unwrap_or("");
    name.ends_with(".rs") || name.ends_with(".ts") || name.ends_with(".md")
}

fn scan(base: &Path, files: &[String], config: &kernel::config::Config) -> Vec<kernel::Finding> {
    let mut findings = Vec::new();
    for path in files {
        one(base, path, config, &mut findings);
    }
    findings
}

fn one(
    base: &Path,
    path: &str,
    config: &kernel::config::Config,
    findings: &mut Vec<kernel::Finding>,
) {
    let rel = relative(base, path);
    if let Some(hit) = kernel::pathdepth(&rel, config) {
        findings.push(hit);
    }
    let text = fs::read_to_string(path).unwrap_or_default();
    let source = grammar::Source { path: rel, text };
    let node = kernel::structure(&source);
    findings.extend(kernel::check(&source, &node, config));
}

fn relative(base: &Path, path: &str) -> String {
    let full = Path::new(path);
    match full.strip_prefix(base) {
        Ok(rel) => rel
            .to_str()
            .unwrap_or(path)
            .trim_start_matches("./")
            .to_string(),
        Err(_) => path.to_string(),
    }
}

fn report(findings: &[kernel::Finding], strict: bool, listed: bool) -> bool {
    let mut ordered: Vec<&kernel::Finding> = findings.iter().collect();
    ordered.sort_by(order);
    let faults = emit(&ordered, kernel::Class::Fault, true);
    let blind = emit(&ordered, kernel::Class::Blind, true);
    let debt = emit(&ordered, kernel::Class::Debt, listed);
    summary(faults, blind, debt);
    faults > 0 || (strict && blind > 0)
}

fn order(a: &&kernel::Finding, b: &&kernel::Finding) -> std::cmp::Ordering {
    (a.path.as_str(), a.line, a.col).cmp(&(b.path.as_str(), b.line, b.col))
}

fn emit(ordered: &[&kernel::Finding], class: kernel::Class, listed: bool) -> usize {
    let mut count = 0;
    for &finding in ordered {
        count += line(finding, class, listed);
    }
    count
}

fn line(finding: &kernel::Finding, class: kernel::Class, listed: bool) -> usize {
    if finding.class != class {
        return 0;
    }
    if listed {
        println!(
            "{} {} {}",
            spot(finding),
            label(class, finding),
            finding.note
        );
    }
    1
}

fn spot(finding: &kernel::Finding) -> String {
    if finding.line == 0 {
        return finding.path.clone();
    }
    format!("{}:{}:{}", finding.path, finding.line, finding.col)
}

fn label(class: kernel::Class, finding: &kernel::Finding) -> &str {
    match class {
        kernel::Class::Blind => "blindspot",
        kernel::Class::Debt => "debt",
        kernel::Class::Fault => &finding.law,
    }
}

fn summary(faults: usize, blind: usize, debt: usize) {
    if faults + blind + debt == 0 {
        println!("clean");
        return;
    }
    println!("{faults} faults, {blind} blindspots, {debt} debt");
}
