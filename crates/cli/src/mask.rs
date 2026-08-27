//! Filename masks: `*`, `?` and `[...]` over the entries of one directory.
//!
//! Written here rather than taken from the `glob` crate because what is
//! wanted is exactly one filename component. `glob` also walks directories
//! and understands `**`, which would then have to be suppressed to keep the
//! semantics this tool promises.

/// Characters that make an argument a mask rather than a path.
const META: [char; 3] = ['*', '?', '['];

/// Does this component ask to be matched rather than opened?
pub fn has_meta(text: &str) -> bool {
    text.contains(META)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MaskError {
    #[error("`**` is recursive; a mask matches one directory only")]
    Recursive,
    #[error("unclosed `[` in the mask")]
    UnclosedClass,
    #[error("reversed range `{from}-{to}` in the mask")]
    ReversedRange { from: char, to: char },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Item {
    Char(char),
    Range(char, char),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Char(char),
    Any,
    Star,
    Class { negated: bool, items: Vec<Item> },
}

impl Token {
    fn accepts(&self, c: char) -> bool {
        match self {
            Token::Char(want) => *want == c,
            Token::Any | Token::Star => true,
            Token::Class { negated, items } => {
                let hit = items.iter().any(|item| match item {
                    Item::Char(x) => *x == c,
                    Item::Range(lo, hi) => (*lo..=*hi).contains(&c),
                });
                hit != *negated
            }
        }
    }
}

/// A compiled filename mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mask {
    tokens: Vec<Token>,
}

impl Mask {
    pub fn new(pattern: &str) -> Result<Self, MaskError> {
        let src: Vec<char> = pattern.chars().collect();
        let mut tokens = Vec::with_capacity(src.len());
        let mut i = 0;
        while i < src.len() {
            match src[i] {
                '*' => {
                    if src.get(i + 1) == Some(&'*') {
                        return Err(MaskError::Recursive);
                    }
                    tokens.push(Token::Star);
                    i += 1;
                }
                '?' => {
                    tokens.push(Token::Any);
                    i += 1;
                }
                '[' => {
                    let (token, next) = parse_class(&src, i + 1)?;
                    tokens.push(token);
                    i = next;
                }
                c => {
                    tokens.push(Token::Char(c));
                    i += 1;
                }
            }
        }
        Ok(Self { tokens })
    }

    pub fn matches(&self, name: &str) -> bool {
        // The Unix convention: a leading dot is matched only by a literal
        // one, so a mask cannot sweep up dotfiles by accident.
        if name.starts_with('.') && self.tokens.first() != Some(&Token::Char('.')) {
            return false;
        }

        let chars: Vec<char> = name.chars().collect();
        let (mut token, mut at) = (0usize, 0usize);
        // Where to resume from if the tail after the last `*` does not fit.
        let (mut star, mut star_at) = (None, 0usize);

        while at < chars.len() {
            match self.tokens.get(token) {
                Some(Token::Star) => {
                    star = Some(token);
                    star_at = at;
                    token += 1;
                }
                Some(t) if t.accepts(chars[at]) => {
                    token += 1;
                    at += 1;
                }
                _ => match star {
                    Some(s) => {
                        token = s + 1;
                        star_at += 1;
                        at = star_at;
                    }
                    None => return false,
                },
            }
        }
        self.tokens[token..].iter().all(|t| *t == Token::Star)
    }
}

/// Parse `[...]` starting just past the bracket, returning the index after it.
fn parse_class(src: &[char], mut i: usize) -> Result<(Token, usize), MaskError> {
    let negated = matches!(src.get(i), Some('!' | '^'));
    if negated {
        i += 1;
    }
    let mut items = Vec::new();
    // A `]` in first position is a literal, as it is in a shell.
    if src.get(i) == Some(&']') {
        items.push(Item::Char(']'));
        i += 1;
    }
    loop {
        let c = *src.get(i).ok_or(MaskError::UnclosedClass)?;
        i += 1;
        if c == ']' {
            return Ok((Token::Class { negated, items }, i));
        }
        // `a-z` is a range; a `-` before the closing bracket is a literal.
        if src.get(i) == Some(&'-') && src.get(i + 1).is_some_and(|next| *next != ']') {
            let to = src[i + 1];
            if to < c {
                return Err(MaskError::ReversedRange { from: c, to });
            }
            items.push(Item::Range(c, to));
            i += 2;
        } else {
            items.push(Item::Char(c));
        }
    }
}

#[cfg(test)]
mod tests {
    include!("mask_tests.rs");
}
