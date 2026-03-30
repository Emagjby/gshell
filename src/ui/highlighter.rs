use nu_ansi_term::{Color, Style};
use reedline::{Highlighter, StyledText};

use crate::builtins::BuiltinRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlighterPalette {
    command_color: Color,
    builtin_color: Color,
    argument_color: Color,
    flag_color: Color,
    operator_color: Color,
    redirect_color: Color,
}

impl Default for HighlighterPalette {
    fn default() -> Self {
        Self {
            command_color: Color::Cyan,
            builtin_color: Color::Cyan,
            argument_color: Color::Green,
            flag_color: Color::Blue,
            operator_color: Color::Purple,
            redirect_color: Color::Purple,
        }
    }
}

impl HighlighterPalette {
    pub fn new(
        command_color: Color,
        builtin_color: Color,
        argument_color: Color,
        flag_color: Color,
        operator_color: Color,
        redirect_color: Color,
    ) -> Self {
        Self {
            command_color,
            builtin_color,
            argument_color,
            flag_color,
            operator_color,
            redirect_color,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ShellHighlighter {
    palette: HighlighterPalette,
}

impl ShellHighlighter {
    pub fn new(palette: HighlighterPalette) -> Self {
        Self { palette }
    }
}

impl Highlighter for ShellHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        highlight_line(line, self.palette)
    }
}

fn highlight_line(line: &str, palette: HighlighterPalette) -> StyledText {
    let mut out = StyledText::new();

    let plain = Style::new();
    let op_style = Style::new().fg(palette.operator_color).dimmed();
    let redir_style = Style::new().fg(palette.redirect_color).dimmed();
    let cmd_style = Style::new().fg(palette.command_color).italic();
    let builtin_style = Style::new().fg(palette.builtin_color).bold();
    let arg_style = Style::new().fg(palette.argument_color);
    let flag_style = Style::new().fg(palette.flag_color).bold();

    let mut i = 0usize;
    let mut command_expected = true;

    while i < line.len() {
        if let Some((ch, ch_len)) = next_char(line, i)
            && ch.is_whitespace()
        {
            let start = i;
            i += ch_len;
            while i < line.len() {
                let Some((c, n)) = next_char(line, i) else {
                    break;
                };
                if !c.is_whitespace() {
                    break;
                }
                i += n;
            }
            out.push((plain, line[start..i].to_string()));
            continue;
        }

        if let Some((tok, end, kind)) = read_operator_token(line, i) {
            let style = match kind {
                OpKind::Redirect => redir_style,
                OpKind::Control => op_style,
            };

            out.push((style, tok.to_string()));

            match tok {
                "|" | "&&" | "||" | ";" | "(" => command_expected = true,
                ")" => command_expected = false,
                _ => {}
            }

            i = end;
            continue;
        }

        let start = i;
        i = read_word_end(line, i);

        let token = &line[start..i];
        let style = if command_expected {
            let candidate = dequote_and_unescape(token);
            if BuiltinRegistry::defaults().contains(&candidate) {
                builtin_style
            } else {
                cmd_style
            }
        } else if is_flag_token(token) {
            flag_style
        } else {
            arg_style
        };

        out.push((style, token.to_string()));

        if command_expected {
            command_expected = false;
        }
    }

    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpKind {
    Control,
    Redirect,
}

fn read_operator_token(line: &str, i: usize) -> Option<(&str, usize, OpKind)> {
    let (ch, ch_len) = next_char(line, i)?;

    if ch == '&' {
        if peek_char(line, i + ch_len) == Some('&') {
            return Some(("&&", i + ch_len + 1, OpKind::Control));
        }
        return Some(("&", i + ch_len, OpKind::Control));
    }

    if ch == '|' {
        if peek_char(line, i + ch_len) == Some('|') {
            return Some(("||", i + ch_len + 1, OpKind::Control));
        }
        return Some(("|", i + ch_len, OpKind::Control));
    }

    if ch == '>' {
        if peek_char(line, i + ch_len) == Some('>') {
            return Some((">>", i + ch_len + 1, OpKind::Redirect));
        }
        return Some((">", i + ch_len, OpKind::Redirect));
    }

    if ch == '<' {
        return Some(("<", i + ch_len, OpKind::Redirect));
    }

    if ch == ';' {
        return Some((";", i + ch_len, OpKind::Control));
    }

    if ch == '(' {
        return Some(("(", i + ch_len, OpKind::Control));
    }

    if ch == ')' {
        return Some((")", i + ch_len, OpKind::Control));
    }

    if ch.is_ascii_digit() {
        let mut j = i;
        while j < line.len() {
            let (c, n) = next_char(line, j)?;
            if !c.is_ascii_digit() {
                break;
            }
            j += n;
        }

        let next = peek_char(line, j)?;
        if next == '>' || next == '<' {
            let op_start = j;
            let (op_ch, op_len) = next_char(line, op_start)?;
            if op_ch == '>' && peek_char(line, op_start + op_len) == Some('>') {
                let end = op_start + op_len + 1;
                return Some((&line[i..end], end, OpKind::Redirect));
            }

            let end = op_start + op_len;
            return Some((&line[i..end], end, OpKind::Redirect));
        }
    }

    None
}

fn read_word_end(line: &str, mut i: usize) -> usize {
    while i < line.len() {
        let (ch, ch_len) = match next_char(line, i) {
            Some(x) => x,
            None => break,
        };

        if ch.is_whitespace() || is_operator_break(ch) {
            break;
        }

        if ch == '\'' {
            i += ch_len;
            while i < line.len() {
                let Some((c, n)) = next_char(line, i) else {
                    break;
                };
                i += n;
                if c == '\'' {
                    break;
                }
            }
            continue;
        }

        if ch == '"' {
            i += ch_len;
            while i < line.len() {
                let Some((c, n)) = next_char(line, i) else {
                    break;
                };
                if c == '\\' {
                    i += n;
                    if i < line.len() {
                        if let Some((_, n2)) = next_char(line, i) {
                            i += n2;
                        } else {
                            break;
                        }
                    }
                    continue;
                }
                i += n;
                if c == '"' {
                    break;
                }
            }
            continue;
        }

        if ch == '\\' {
            i += ch_len;
            if i < line.len() {
                if let Some((_, n2)) = next_char(line, i) {
                    i += n2;
                } else {
                    break;
                }
            }
            continue;
        }

        i += ch_len;
    }

    i
}

fn is_operator_break(ch: char) -> bool {
    matches!(ch, '|' | '&' | ';' | '(' | ')' | '<' | '>')
}

fn is_flag_token(token: &str) -> bool {
    let token = token.trim();
    token.starts_with('-') && token != "-" && !token.starts_with("->")
}

fn dequote_and_unescape(token: &str) -> String {
    let mut out = String::new();
    let mut i = 0usize;

    while i < token.len() {
        let Some((ch, n)) = next_char(token, i) else {
            break;
        };

        match ch {
            '\'' => {
                i += n;
                while i < token.len() {
                    let Some((c, n2)) = next_char(token, i) else {
                        break;
                    };
                    i += n2;
                    if c == '\'' {
                        break;
                    }
                    out.push(c);
                }
            }
            '"' => {
                i += n;
                while i < token.len() {
                    let Some((c, n2)) = next_char(token, i) else {
                        break;
                    };
                    if c == '\\' {
                        i += n2;
                        if i < token.len() {
                            if let Some((c3, n3)) = next_char(token, i) {
                                i += n3;
                                out.push(c3);
                            } else {
                                break;
                            }
                        }
                        continue;
                    }
                    i += n2;
                    if c == '"' {
                        break;
                    }
                    out.push(c);
                }
            }
            '\\' => {
                i += n;
                if i < token.len() {
                    if let Some((c2, n2)) = next_char(token, i) {
                        i += n2;
                        out.push(c2);
                    } else {
                        break;
                    }
                }
            }
            _ => {
                out.push(ch);
                i += n;
            }
        }
    }

    out
}

fn next_char(s: &str, i: usize) -> Option<(char, usize)> {
    s.get(i..)?.chars().next().map(|ch| (ch, ch.len_utf8()))
}

fn peek_char(s: &str, i: usize) -> Option<char> {
    s.get(i..)?.chars().next()
}

#[cfg(test)]
mod tests {
    use nu_ansi_term::{Color, Style};

    use super::{HighlighterPalette, highlight_line};

    fn palette() -> HighlighterPalette {
        HighlighterPalette::new(
            Color::Cyan,
            Color::LightCyan,
            Color::Green,
            Color::Blue,
            Color::Purple,
            Color::Yellow,
        )
    }

    #[test]
    fn styles_commands_builtins_args_flags_and_operators() {
        let highlighted = highlight_line("ls -la file | cd", palette());

        assert_eq!(
            highlighted.buffer,
            vec![
                (Style::new().fg(Color::Cyan).italic(), "ls".to_string()),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::Blue).bold(), "-la".to_string()),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::Green), "file".to_string()),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::Purple).dimmed(), "|".to_string()),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::LightCyan).bold(), "cd".to_string()),
            ]
        );
    }

    #[test]
    fn styles_redirections_separately_from_arguments() {
        let highlighted = highlight_line("echo hello 2>> output.txt", palette());

        assert_eq!(
            highlighted.buffer,
            vec![
                (Style::new().fg(Color::LightCyan).bold(), "echo".to_string()),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::Green), "hello".to_string()),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::Yellow).dimmed(), "2>>".to_string()),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::Green), "output.txt".to_string()),
            ]
        );
    }

    #[test]
    fn keeps_quoted_and_escaped_arguments_as_single_tokens() {
        let highlighted = highlight_line("printf \"hello world\" some\\ file", palette());

        assert_eq!(
            highlighted.buffer,
            vec![
                (Style::new().fg(Color::Cyan).italic(), "printf".to_string()),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::Green), "\"hello world\"".to_string(),),
                (Style::new(), " ".to_string()),
                (Style::new().fg(Color::Green), "some\\ file".to_string()),
            ]
        );
    }
}
