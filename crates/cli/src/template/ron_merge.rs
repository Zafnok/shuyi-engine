//! Two RON files merged into one, keeping the first's text: its layout
//! and its comments. The second's struct fields and map entries that the
//! first lacks are added to it, with their own comments; a struct or a map
//! both have is merged the same way; a list gains the items it lacks.
//! Anything else both have must be the same, or the merge fails with its
//! path and both values.
//!
//! The text is read only for where each value starts and ends (a
//! [`Node`]): it is never rewritten, so what comes out is the first file
//! with the second's pieces inserted.

/// A container's brackets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Delim {
    /// `( )`: a struct, a tuple or an enum's fields.
    Paren,
    /// `[ ]`: a list.
    Bracket,
    /// `{ }`: a map.
    Brace,
}

impl Delim {
    fn close(self) -> u8 {
        match self {
            Delim::Paren => b')',
            Delim::Bracket => b']',
            Delim::Brace => b'}',
        }
    }
}

/// Where a value is in its text.
#[derive(Debug)]
struct Node {
    start: usize,
    end: usize,
    container: Option<Container>,
}

/// A value in brackets.
#[derive(Debug)]
struct Container {
    delim: Delim,
    /// Its struct's name (`Template`), if it has one.
    name: Option<(usize, usize)>,
    open: usize,
    close: usize,
    items: Vec<Item>,
    /// Whether a comma follows the last item.
    trailing_comma: bool,
}

/// One field, entry or item of a container.
#[derive(Debug)]
struct Item {
    /// Where the text before it starts: after the comma before it, or the
    /// opening bracket.
    lead: usize,
    start: usize,
    /// Its key's span: a field's name or a map entry's key.
    key: Option<(usize, usize)>,
    value: Node,
}

/// The reader of a RON text.
struct Parser<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            bytes: text.as_bytes(),
            at: 0,
        }
    }

    /// Past `n` more bytes. Every move forward goes through here.
    fn advance(&mut self, n: usize) {
        self.at += n;
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<u8> {
        self.bytes.get(self.at + ahead).copied()
    }

    /// Where it is, for errors: `line:column`.
    fn here(&self) -> String {
        let before = self.text.get(..self.at).unwrap_or(self.text);
        let line = before.matches('\n').count() + 1;
        let column = before.len() - before.rfind('\n').map_or(0, |i| i + 1) + 1;
        format!("{line}:{column}")
    }

    fn fail<T>(&self, message: &str) -> Result<T, String> {
        Err(format!("{} at {}", message, self.here()))
    }

    /// Past spaces and comments.
    fn trivia(&mut self) -> Result<(), String> {
        loop {
            match (self.peek(), self.peek_at(1)) {
                (Some(b), _) if b.is_ascii_whitespace() => self.advance(1),
                (Some(b'/'), Some(b'/')) => {
                    while self.peek().is_some_and(|b| b != b'\n') {
                        self.advance(1);
                    }
                }
                (Some(b'/'), Some(b'*')) => {
                    self.advance(2);
                    let mut depth = 1;
                    while depth > 0 {
                        match (self.peek(), self.peek_at(1)) {
                            (None, _) => return self.fail("a comment never closed"),
                            (Some(b'*'), Some(b'/')) => {
                                depth -= 1;
                                self.advance(2);
                            }
                            (Some(b'/'), Some(b'*')) => {
                                depth += 1;
                                self.advance(2);
                            }
                            _ => self.advance(1),
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// Past a `"…"` string, at its opening quote.
    fn string(&mut self) -> Result<(), String> {
        self.advance(1);
        loop {
            match self.peek() {
                None => return self.fail("a string never closed"),
                Some(b'\\') => self.advance(2),
                Some(b'"') => {
                    self.advance(1);
                    return Ok(());
                }
                Some(_) => self.advance(1),
            }
        }
    }

    /// Past a raw string, `r#"…"#`, at its `r`.
    fn raw_string(&mut self) -> Result<(), String> {
        self.advance(1);
        let mut hashes = 0;
        while self.peek() == Some(b'#') {
            hashes += 1;
            self.advance(1);
        }
        if self.peek() != Some(b'"') {
            return self.fail("a raw string with no quote");
        }
        self.advance(1);
        let end = format!("\"{}", "#".repeat(hashes));
        match self.text[self.at..].find(&end) {
            Some(found) => {
                self.advance(found + end.len());
                Ok(())
            }
            None => self.fail("a raw string never closed"),
        }
    }

    /// Past a `'…'` character, at its opening quote.
    fn char_literal(&mut self) -> Result<(), String> {
        self.advance(1);
        if self.peek() == Some(b'\\') {
            self.advance(2);
        }
        while self.peek().is_some_and(|b| b != b'\'') {
            self.advance(1);
        }
        if self.peek().is_none() {
            return self.fail("a character never closed");
        }
        self.advance(1);
        Ok(())
    }

    /// Past a word or a number: a value that isn't in brackets or quotes.
    fn atom(&mut self) {
        while let Some(b) = self.peek() {
            let comment = b == b'/' && matches!(self.peek_at(1), Some(b'/' | b'*'));
            if b.is_ascii_whitespace() || b",:()[]{}\"'".contains(&b) || comment {
                break;
            }
            self.advance(1);
        }
    }

    /// A value.
    fn value(&mut self) -> Result<Node, String> {
        self.trivia()?;
        let start = self.at;
        match (self.peek(), self.peek_at(1)) {
            (None, _) => self.fail("the text ends where a value should be"),
            (Some(b'('), _) => self.container(Delim::Paren, None, start),
            (Some(b'['), _) => self.container(Delim::Bracket, None, start),
            (Some(b'{'), _) => self.container(Delim::Brace, None, start),
            (Some(b'"'), _) => {
                self.string()?;
                Ok(self.scalar(start))
            }
            (Some(b'\''), _) => {
                self.char_literal()?;
                Ok(self.scalar(start))
            }
            (Some(b'r'), Some(b'"' | b'#')) => {
                self.raw_string()?;
                Ok(self.scalar(start))
            }
            (Some(b'b'), Some(b'"')) => {
                self.advance(1);
                self.string()?;
                Ok(self.scalar(start))
            }
            (Some(b), _) if b",:)]}".contains(&b) => {
                self.fail(&format!("`{}` where a value should be", char::from(b)))
            }
            _ => {
                self.atom();
                let end = self.at;
                self.trivia()?;
                if self.peek() == Some(b'(') {
                    self.container(Delim::Paren, Some((start, end)), start)
                } else {
                    self.at = end;
                    Ok(self.scalar(start))
                }
            }
        }
    }

    fn scalar(&self, start: usize) -> Node {
        Node {
            start,
            end: self.at,
            container: None,
        }
    }

    /// A field's name and its `:`, if one is next: the name's end.
    fn field_name(&mut self) -> Result<Option<usize>, String> {
        let start = self.at;
        if !self
            .peek()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        {
            return Ok(None);
        }
        while self
            .peek()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            self.advance(1);
        }
        let end = self.at;
        self.trivia()?;
        if self.peek() == Some(b':') {
            self.advance(1);
            Ok(Some(end))
        } else {
            self.at = start;
            Ok(None)
        }
    }

    /// A container, at its opening bracket.
    fn container(
        &mut self,
        delim: Delim,
        name: Option<(usize, usize)>,
        start: usize,
    ) -> Result<Node, String> {
        let open = self.at;
        self.advance(1);
        let close_byte = delim.close();
        let mut items = Vec::new();
        let mut trailing_comma = false;
        loop {
            let lead = self.at;
            self.trivia()?;
            match self.peek() {
                None => return self.fail("a bracket never closed"),
                Some(b) if b == close_byte => break,
                Some(_) => {}
            }
            let item_start = self.at;
            let key = match delim {
                Delim::Brace => {
                    let key = self.value()?;
                    self.trivia()?;
                    if self.peek() != Some(b':') {
                        return self.fail("a map entry with no `:`");
                    }
                    self.advance(1);
                    Some((key.start, key.end))
                }
                Delim::Paren => self.field_name()?.map(|end| (item_start, end)),
                Delim::Bracket => None,
            };
            let value = self.value()?;
            items.push(Item {
                lead,
                start: item_start,
                key,
                value,
            });
            trailing_comma = false;
            self.trivia()?;
            match self.peek() {
                Some(b',') => {
                    self.advance(1);
                    trailing_comma = true;
                }
                Some(b) if b == close_byte => break,
                _ => {
                    return self.fail(&format!("expected `,` or `{}`", char::from(close_byte)));
                }
            }
        }
        let close = self.at;
        self.advance(1);
        Ok(Node {
            start,
            end: self.at,
            container: Some(Container {
                delim,
                name,
                open,
                close,
                items,
                trailing_comma,
            }),
        })
    }
}

/// The value a whole RON text holds, after any `#![enable(…)]`.
fn parse(text: &str) -> Result<Node, String> {
    let mut parser = Parser::new(text);
    parser.trivia()?;
    while text[parser.at..].starts_with("#!") {
        match text[parser.at..].find(']') {
            Some(end) => parser.advance(end + 1),
            None => return parser.fail("an attribute never closed"),
        }
        parser.trivia()?;
    }
    let node = parser.value()?;
    parser.trivia()?;
    if parser.at < text.len() {
        return parser.fail("more after the value");
    }
    Ok(node)
}

/// `text` with no spaces or comments outside its strings, to compare.
fn normalized(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                while let Some(c) = chars.next() {
                    out.push(c);
                    if c == '"' {
                        break;
                    }
                    if c == '\\'
                        && let Some(escaped) = chars.next()
                    {
                        out.push(escaped);
                    }
                }
            }
            '/' if chars.peek() == Some(&'/') => while chars.next_if(|c| *c != '\n').is_some() {},
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            c if c.is_ascii_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}

/// A value's text on one line, cut short, for an error.
fn shown(text: &str) -> String {
    let one_line: Vec<&str> = text.split_whitespace().collect();
    let one_line = one_line.join(" ");
    if one_line.chars().count() > 60 {
        let cut: String = one_line.chars().take(57).collect();
        format!("{cut}...")
    } else {
        one_line
    }
}

/// The spaces that start the line `at` is on.
fn indent_at(text: &str, at: usize) -> String {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    text[line_start..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect()
}

/// `item`, a value's text whose first line was indented `from`, with its
/// other lines indented `to` in its place.
fn reindent(item: &str, from: &str, to: &str) -> String {
    let mut lines = item.split('\n');
    let mut out = lines.next().unwrap_or_default().to_owned();
    for line in lines {
        out.push('\n');
        if let Some(rest) = line.strip_prefix(from) {
            out.push_str(to);
            out.push_str(rest);
        } else if !line.trim().is_empty() {
            out.push_str(line);
        }
    }
    out
}

/// The comment lines just above `item` in `text`, and whether a blank
/// line comes before them.
fn leading<'t>(text: &'t str, item: &Item) -> (bool, Vec<&'t str>) {
    let mut lines: Vec<&str> = text[item.lead..item.start].split('\n').collect();
    // The end of the line before (after its comma), and the start of the
    // item's own line: on one line, the item has nothing above it.
    lines.remove(0);
    lines.pop();
    let blank = lines.first().is_some_and(|line| line.trim().is_empty());
    let comments = lines
        .into_iter()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    (blank, comments)
}

/// The merge of one text into another: the two texts and the insertions
/// into the first, by where they go.
struct Merge<'a> {
    base: &'a str,
    other: &'a str,
    edits: Vec<(usize, String)>,
}

impl Merge<'_> {
    /// `other` merged into `base`, both at `at`.
    fn nodes(&mut self, base: &Node, other: &Node, at: &str) -> Result<(), (String, String)> {
        if let (Some(b), Some(o)) = (&base.container, &other.container)
            && self.mergeable(b, o)
        {
            return self.containers(b, o, at);
        }
        let (b, o) = (
            &self.base[base.start..base.end],
            &self.other[other.start..other.end],
        );
        if normalized(b) == normalized(o) {
            Ok(())
        } else {
            Err((
                at.to_owned(),
                format!("`{}` in one, `{}` in another", shown(b), shown(o)),
            ))
        }
    }

    /// Whether two containers merge, rather than having to be the same:
    /// two lists, two maps, or two structs of one name.
    fn mergeable(&self, base: &Container, other: &Container) -> bool {
        if base.delim != other.delim {
            return false;
        }
        if base.delim != Delim::Paren {
            return true;
        }
        let keyed = |c: &Container| c.items.iter().all(|item| item.key.is_some());
        let name = |text: &str, c: &Container| c.name.map(|(s, e)| text[s..e].to_owned());
        let names_agree = match (name(self.base, base), name(self.other, other)) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        };
        keyed(base) && keyed(other) && names_agree
    }

    /// `other`'s items merged into `base`'s.
    fn containers(
        &mut self,
        base: &Container,
        other: &Container,
        at: &str,
    ) -> Result<(), (String, String)> {
        let mut added: Vec<&Item> = Vec::new();
        for item in &other.items {
            match item.key {
                None => {
                    let text = normalized(&self.other[item.start..item.value.end]);
                    let there = base
                        .items
                        .iter()
                        .any(|b| normalized(&self.base[b.start..b.value.end]) == text)
                        || added
                            .iter()
                            .any(|a| normalized(&self.other[a.start..a.value.end]) == text);
                    if !there {
                        added.push(item);
                    }
                }
                Some((start, end)) => {
                    let key = normalized(&self.other[start..end]);
                    let here = match base.delim {
                        Delim::Brace => format!("{at}[{key}]"),
                        _ if at.is_empty() => key.clone(),
                        _ => format!("{at}.{key}"),
                    };
                    let same = base.items.iter().find(|b| {
                        b.key
                            .is_some_and(|(s, e)| normalized(&self.base[s..e]) == key)
                    });
                    match same {
                        Some(b) => self.nodes(&b.value, &item.value, &here)?,
                        None => added.push(item),
                    }
                }
            }
        }
        if !added.is_empty() {
            self.insert(base, &added);
        }
        Ok(())
    }

    /// `items` of the other text added at the end of `container`.
    fn insert(&mut self, container: &Container, items: &[&Item]) {
        let multiline = self.base[container.open..container.close].contains('\n');
        if !multiline {
            let mut at = container.open + 1;
            let mut first = container.items.is_empty();
            if let Some(last) = container.items.last() {
                at = last.value.end;
            }
            for item in items {
                let text = &self.other[item.start..item.value.end];
                let text = if first {
                    text.to_owned()
                } else {
                    format!(", {text}")
                };
                first = false;
                self.edits.push((at, text));
            }
            return;
        }
        let indent = match container.items.first() {
            Some(first) => indent_at(self.base, first.start),
            None => format!("{}    ", indent_at(self.base, container.close)),
        };
        if let Some(last) = container.items.last()
            && !container.trailing_comma
        {
            self.edits.push((last.value.end, ",".to_owned()));
        }
        let line_start = self.base[..container.close]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let own_line = self.base[line_start..container.close].trim().is_empty();
        let mut text = if own_line {
            String::new()
        } else {
            "\n".to_owned()
        };
        // Commented items set apart by blank lines in the first text are
        // set apart the same.
        let spaced = self.base[container.open..container.close].contains(
            "

",
        );
        for item in items {
            let (blank, comments) = leading(self.other, item);
            if (blank || spaced && !comments.is_empty()) && !container.items.is_empty() {
                text.push('\n');
            }
            for comment in comments {
                text.push_str(&indent);
                text.push_str(comment);
                text.push('\n');
            }
            let own = indent_at(self.other, item.start);
            text.push_str(&indent);
            text.push_str(&reindent(
                &self.other[item.start..item.value.end],
                &own,
                &indent,
            ));
            text.push_str(",\n");
        }
        let at = if own_line {
            line_start
        } else {
            container.close
        };
        self.edits.push((at, text));
    }
}

/// `other` merged into `base` (see the module's doc); or where they differ
/// (a path such as `check_keys.rust`, or `""`) and how.
pub(super) fn merge(base: &str, other: &str) -> Result<String, (String, String)> {
    let base_node = parse(base).map_err(|e| (String::new(), format!("not RON: {e}")))?;
    let other_node = parse(other).map_err(|e| (String::new(), format!("not RON: {e}")))?;
    let mut merge = Merge {
        base,
        other,
        edits: Vec::new(),
    };
    merge.nodes(&base_node, &other_node, "")?;
    let mut edits = merge.edits;
    // Stable: insertions at one place keep their order.
    edits.sort_by_key(|(at, _)| *at);
    let mut out = String::with_capacity(base.len());
    let mut from = 0;
    for (at, text) in edits {
        out.push_str(&base[from..at]);
        out.push_str(&text);
        from = at;
    }
    out.push_str(&base[from..]);
    Ok(out)
}

#[cfg(test)]
mod tests;
