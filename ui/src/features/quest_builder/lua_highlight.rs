//! Syntax-Hervorhebung für Quest-Skripte - Nachbau des Lua-Modus, den der
//! frühere CodeMirror-Editor nutzte (`@codemirror/legacy-modes/mode/lua`):
//! Schlüsselwörter, eingebaute Funktionen, Strings (auch `[[...]]`),
//! Kommentare (`--`, `--[[...]]`) und Zahlen. Liefert fertiges HTML
//! (escaped) für die Overlay-Ebene des Code-Editors.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenKind {
    Plain,
    Keyword,
    Builtin,
    String,
    Comment,
    Number,
}

impl TokenKind {
    /// Farben wie CodeMirrors Standard-Hell- bzw. One-Dark-Thema.
    pub fn class(self) -> &'static str {
        match self {
            TokenKind::Plain => "",
            TokenKind::Keyword => "text-[#708] dark:text-[#c678dd]",
            TokenKind::Builtin => "text-[#219] dark:text-[#e5c07b]",
            TokenKind::String => "text-[#a11] dark:text-[#98c379]",
            TokenKind::Comment => "italic text-[#940] dark:text-[#7d8799]",
            TokenKind::Number => "text-[#164] dark:text-[#d19a66]",
        }
    }
}

const KEYWORDS: &[&str] = &[
    "and", "break", "elseif", "false", "nil", "not", "or", "return", "true", "function", "end", "if", "then", "else", "do",
    "while", "repeat", "until", "for", "in", "local",
];

const BUILTINS: &[&str] = &[
    "_G", "_VERSION", "assert", "collectgarbage", "dofile", "error", "getfenv", "getmetatable", "ipairs", "load",
    "loadfile", "loadstring", "module", "next", "pairs", "pcall", "print", "rawequal", "rawget", "rawset", "require",
    "select", "setfenv", "setmetatable", "tonumber", "tostring", "type", "unpack", "xpcall",
    "coroutine.create", "coroutine.resume", "coroutine.running", "coroutine.status", "coroutine.wrap", "coroutine.yield",
    "debug.debug", "debug.getfenv", "debug.gethook", "debug.getinfo", "debug.getlocal", "debug.getmetatable",
    "debug.getregistry", "debug.getupvalue", "debug.setfenv", "debug.sethook", "debug.setlocal", "debug.setmetatable",
    "debug.setupvalue", "debug.traceback",
    "close", "flush", "lines", "read", "seek", "setvbuf", "write",
    "io.close", "io.flush", "io.input", "io.lines", "io.open", "io.output", "io.popen", "io.read", "io.stderr",
    "io.stdin", "io.stdout", "io.tmpfile", "io.type", "io.write",
    "math.abs", "math.acos", "math.asin", "math.atan", "math.atan2", "math.ceil", "math.cos", "math.cosh", "math.deg",
    "math.exp", "math.floor", "math.fmod", "math.frexp", "math.huge", "math.ldexp", "math.log", "math.log10",
    "math.max", "math.min", "math.modf", "math.pi", "math.pow", "math.rad", "math.random", "math.randomseed",
    "math.sin", "math.sinh", "math.sqrt", "math.tan", "math.tanh",
    "os.clock", "os.date", "os.difftime", "os.execute", "os.exit", "os.getenv", "os.remove", "os.rename",
    "os.setlocale", "os.time", "os.tmpname",
    "package.cpath", "package.loaded", "package.loaders", "package.loadlib", "package.path", "package.preload",
    "package.seeall",
    "string.byte", "string.char", "string.dump", "string.find", "string.format", "string.gmatch", "string.gsub",
    "string.len", "string.lower", "string.match", "string.rep", "string.reverse", "string.sub", "string.upper",
    "table.concat", "table.insert", "table.maxn", "table.remove", "table.sort",
];

/// `[` gefolgt von `=`* und `[` → Anzahl der `=` (Lua-Langklammer).
fn long_bracket_level(b: &[u8], i: usize) -> Option<usize> {
    if b.get(i) != Some(&b'[') {
        return None;
    }
    let mut j = i + 1;
    while b.get(j) == Some(&b'=') {
        j += 1;
    }
    (b.get(j) == Some(&b'[')).then_some(j - i - 1)
}

/// Ende (exklusiv) einer Langklammer ab `from` - oder Textende.
fn long_bracket_end(text: &str, from: usize, level: usize) -> usize {
    let close = format!("]{}]", "=".repeat(level));
    text[from..].find(&close).map(|p| from + p + close.len()).unwrap_or(text.len())
}

/// Zerlegt `text` in (Bereich, Art)-Stücke, lückenlos.
pub fn tokenize(text: &str) -> Vec<(usize, usize, TokenKind)> {
    let b = text.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    let mut plain_start = 0;
    let push = |out: &mut Vec<(usize, usize, TokenKind)>, plain_start: &mut usize, s: usize, e: usize, k: TokenKind| {
        if *plain_start < s {
            out.push((*plain_start, s, TokenKind::Plain));
        }
        out.push((s, e, k));
        *plain_start = e;
    };
    while i < b.len() {
        let c = b[i];
        if c == b'-' && b.get(i + 1) == Some(&b'-') {
            let end = match long_bracket_level(b, i + 2) {
                Some(level) => long_bracket_end(text, i + 4 + level, level),
                None => text[i..].find('\n').map(|p| i + p).unwrap_or(text.len()),
            };
            push(&mut out, &mut plain_start, i, end, TokenKind::Comment);
            i = end;
        } else if let Some(level) = long_bracket_level(b, i) {
            let end = long_bracket_end(text, i + 2 + level, level);
            push(&mut out, &mut plain_start, i, end, TokenKind::String);
            i = end;
        } else if c == b'"' || c == b'\'' {
            let mut j = i + 1;
            while j < b.len() {
                match b[j] {
                    b'\\' => j += 2,
                    b'\n' => break,
                    x if x == c => {
                        j += 1;
                        break;
                    }
                    _ => j += 1,
                }
            }
            let end = j.min(b.len());
            push(&mut out, &mut plain_start, i, end, TokenKind::String);
            i = end;
        } else if c.is_ascii_digit() {
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'.' || b[j] == b'%') {
                j += 1;
            }
            push(&mut out, &mut plain_start, i, j, TokenKind::Number);
            i = j;
        } else if c.is_ascii_alphabetic() || c == b'_' {
            let mut j = i + 1;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || matches!(b[j], b'_' | b'\\' | b'-' | b'.')) {
                // "--" beginnt einen Kommentar, kein Teil des Bezeichners.
                if b[j] == b'-' && b.get(j + 1) == Some(&b'-') {
                    break;
                }
                j += 1;
            }
            let word = &text[i..j];
            if KEYWORDS.contains(&word) {
                push(&mut out, &mut plain_start, i, j, TokenKind::Keyword);
            } else if BUILTINS.contains(&word) {
                push(&mut out, &mut plain_start, i, j, TokenKind::Builtin);
            }
            i = j;
        } else {
            // ganze UTF-8-Zeichen überspringen
            i += text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        }
    }
    if plain_start < text.len() {
        out.push((plain_start, text.len(), TokenKind::Plain));
    }
    out
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Hervorgehobenes HTML für die Overlay-Ebene.
pub fn highlight_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for (s, e, k) in tokenize(text) {
        let part = escape_html(&text[s..e]);
        if k == TokenKind::Plain {
            out.push_str(&part);
        } else {
            out.push_str("<span class=\"");
            out.push_str(k.class());
            out.push_str("\">");
            out.push_str(&part);
            out.push_str("</span>");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(&str, TokenKind)> {
        tokenize(text).into_iter().filter(|t| t.2 != TokenKind::Plain).map(|(s, e, k)| (&text[s..e], k)).collect()
    }

    #[test]
    fn covers_whole_text() {
        let text = "quest x begin -- hi\n\tsay(\"ä\")\nend";
        let toks = tokenize(text);
        assert_eq!(toks.first().unwrap().0, 0);
        assert_eq!(toks.last().unwrap().1, text.len());
        for w in toks.windows(2) {
            assert_eq!(w[0].1, w[1].0);
        }
    }

    #[test]
    fn recognizes_tokens() {
        let k = kinds("if pc.count_item(100) >= 5 then say(\"a \\\" b\") end -- c\nlocal s = [[x\ny]] string.format('z')");
        assert_eq!(
            k,
            vec![
                ("if", TokenKind::Keyword),
                ("100", TokenKind::Number),
                ("5", TokenKind::Number),
                ("then", TokenKind::Keyword),
                ("\"a \\\" b\"", TokenKind::String),
                ("end", TokenKind::Keyword),
                ("-- c", TokenKind::Comment),
                ("local", TokenKind::Keyword),
                ("[[x\ny]]", TokenKind::String),
                ("string.format", TokenKind::Builtin),
                ("'z'", TokenKind::String),
            ]
        );
    }

    #[test]
    fn long_comment() {
        let k = kinds("--[==[ a ]] b ]==] end");
        assert_eq!(k, vec![("--[==[ a ]] b ]==]", TokenKind::Comment), ("end", TokenKind::Keyword)]);
    }

    #[test]
    fn escapes_html() {
        assert_eq!(highlight_html("a<b"), "a&lt;b");
    }
}
