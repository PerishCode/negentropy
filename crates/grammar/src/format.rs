use crate::peg::{Item, Pat, Quant};

pub struct Rule {
    pub name: String,
    pub pat: Pat,
    pub tag: Option<String>,
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

pub fn load(text: &str) -> Vec<Rule> {
    let mut cur = Cursor {
        bytes: text.as_bytes(),
        at: 0,
    };
    let mut rules = Vec::new();
    cur.trivia();
    while cur.at < cur.bytes.len() {
        rules.push(cur.rule());
        cur.trivia();
    }
    rules
}

impl Cursor<'_> {
    fn peek(&self) -> u8 {
        if self.at < self.bytes.len() {
            self.bytes[self.at]
        } else {
            0
        }
    }

    fn ahead(&self) -> u8 {
        if self.at + 1 < self.bytes.len() {
            self.bytes[self.at + 1]
        } else {
            0
        }
    }

    fn bump(&mut self) -> u8 {
        let byte = self.peek();
        self.at += 1;
        byte
    }

    fn eat(&mut self, byte: u8) -> bool {
        if self.peek() == byte {
            self.at += 1;
            return true;
        }
        false
    }

    fn trivia(&mut self) {
        while self.trims() {}
    }

    fn trims(&mut self) -> bool {
        let before = self.at;
        self.spaces();
        self.line();
        self.at != before
    }

    fn spaces(&mut self) {
        while space(self.peek()) {
            self.at += 1;
        }
    }

    fn line(&mut self) {
        if self.peek() != b'/' || self.ahead() != b'/' {
            return;
        }
        while self.at < self.bytes.len() && self.peek() != b'\n' {
            self.at += 1;
        }
    }

    fn rule(&mut self) -> Rule {
        let name = self.name();
        self.trivia();
        self.eat(b':');
        self.trivia();
        let pat = self.alt();
        self.trivia();
        let tag = self.action();
        self.trivia();
        self.eat(b';');
        Rule { name, pat, tag }
    }

    fn action(&mut self) -> Option<String> {
        if self.peek() == b'-' && self.ahead() == b'>' {
            self.at += 2;
            self.trivia();
            return Some(self.name());
        }
        None
    }

    fn name(&mut self) -> String {
        let start = self.at;
        while namebyte(self.peek()) {
            self.at += 1;
        }
        String::from_utf8_lossy(&self.bytes[start..self.at]).into_owned()
    }

    fn alt(&mut self) -> Pat {
        let mut parts = vec![self.seq()];
        self.trivia();
        while self.peek() == b'/' || self.peek() == b'|' {
            self.at += 1;
            self.trivia();
            parts.push(self.seq());
            self.trivia();
        }
        fold(parts, Pat::Alt)
    }

    fn seq(&mut self) -> Pat {
        let mut parts = Vec::new();
        while self.starts() {
            parts.push(self.rep());
            self.trivia();
        }
        fold(parts, Pat::Seq)
    }

    fn starts(&self) -> bool {
        let byte = self.peek();
        namehead(byte) || matches!(byte, b'\'' | b'[' | b'~' | b'.' | b'!' | b'(')
    }

    fn rep(&mut self) -> Pat {
        let atom = self.atom();
        quantify(atom, self.quant())
    }

    fn quant(&mut self) -> Option<Quant> {
        if self.eat(b'*') {
            return Some(Quant::Star);
        }
        if self.eat(b'+') {
            return Some(Quant::Plus);
        }
        if self.eat(b'?') {
            return Some(Quant::Opt);
        }
        None
    }

    fn atom(&mut self) -> Pat {
        let byte = self.peek();
        if byte == b'\'' {
            return Pat::Lit(self.strlit());
        }
        if byte == b'[' {
            return Pat::Class(self.class(), false);
        }
        if byte == b'~' {
            self.at += 1;
            return Pat::Class(self.class(), true);
        }
        if byte == b'!' {
            self.at += 1;
            return Pat::Not(Box::new(self.atom()));
        }
        self.wide(byte)
    }

    fn wide(&mut self, byte: u8) -> Pat {
        if byte == b'.' {
            self.at += 1;
            return Pat::Any;
        }
        if namehead(byte) {
            return Pat::Name(self.name());
        }
        self.group()
    }

    fn group(&mut self) -> Pat {
        self.eat(b'(');
        self.trivia();
        let inner = self.alt();
        self.trivia();
        self.eat(b')');
        inner
    }

    fn strlit(&mut self) -> Vec<u8> {
        self.eat(b'\'');
        let mut out = Vec::new();
        while self.at < self.bytes.len() && self.peek() != b'\'' {
            out.push(self.escaped());
        }
        self.eat(b'\'');
        out
    }

    fn escaped(&mut self) -> u8 {
        if self.peek() == b'\\' {
            self.at += 1;
            return unescape(self.bump());
        }
        self.bump()
    }

    fn class(&mut self) -> Vec<Item> {
        self.eat(b'[');
        let mut items = Vec::new();
        while self.at < self.bytes.len() && self.peek() != b']' {
            items.push(self.item());
        }
        self.eat(b']');
        items
    }

    fn item(&mut self) -> Item {
        let lo = self.escaped();
        if self.peek() == b'-' && self.ahead() != b']' {
            self.at += 1;
            return Item::Span(lo, self.escaped());
        }
        Item::One(lo)
    }
}

fn space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn namebyte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn namehead(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn unescape(byte: u8) -> u8 {
    match byte {
        b'n' => b'\n',
        b'r' => b'\r',
        b't' => b'\t',
        other => other,
    }
}

fn fold(mut parts: Vec<Pat>, wrap: fn(Vec<Pat>) -> Pat) -> Pat {
    if parts.len() == 1 {
        return parts.pop().unwrap();
    }
    wrap(parts)
}

fn quantify(atom: Pat, quant: Option<Quant>) -> Pat {
    match quant {
        Some(one) => Pat::Rep(Box::new(atom), one),
        None => atom,
    }
}
