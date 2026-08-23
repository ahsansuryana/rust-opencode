//! Ported from: packages/core/src/util/wildcard.ts (Wildcard.match)

/// Ported from: core/util/wildcard.ts:3-13
/// Pola glob sederhana: `*` (nol+ karakter), `?` (satu karakter), sisanya
/// literal. Backslash dinormalisasi ke `/`. Pattern berakhiran ` *` membuat
/// spasi-trailing opsional (`( .*)?`). Case-insensitive hanya di Windows
/// (flag regex "i" pada win32 di source asli).
pub fn r#match(input: &str, pattern: &str) -> bool {
    let normalized = input.replace('\\', "/");
    let pattern = pattern.replace('\\', "/");

    // special case: escaped.endsWith(" .*") → `^base( .*)?$`
    // alternatif: input == base (glob), atau base + spasi + apa pun.
    if let Some(base) = pattern.strip_suffix(" *") {
        return glob_match(&normalized, base) || glob_match(&normalized, &pattern);
    }
    glob_match(&normalized, &pattern)
}

fn fold_eq(a: char, b: char) -> bool {
    if cfg!(windows) {
        a.eq_ignore_ascii_case(&b)
    } else {
        a == b
    }
}

/// Klasik two-pointer glob dengan backtracking untuk `*`.
fn glob_match(input: &str, pattern: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let input: Vec<char> = input.chars().collect();
    let mut p = 0usize;
    let mut i = 0usize;
    let mut star: Option<usize> = None;
    let mut star_input = 0usize;

    while i < input.len() {
        if p < pattern.len() && pattern[p] == '*' {
            star = Some(p);
            star_input = i;
            p += 1;
        } else if p < pattern.len() && (pattern[p] == '?' || fold_eq(pattern[p], input[i])) {
            p += 1;
            i += 1;
        } else if let Some(star_pos) = star {
            p = star_pos + 1;
            star_input += 1;
            i = star_input;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

#[cfg(test)]
mod tests {
    use super::r#match;

    #[test]
    fn basic_wildcards() {
        assert!(r#match("bash", "*"));
        assert!(r#match("edit", "edit"));
        assert!(!r#match("edit", "edt"));
        assert!(r#match("a/b/c", "a/*/c"));
        assert!(r#match("git push origin", "git *"));
    }

    #[test]
    fn backslash_normalized() {
        assert!(r#match("C:\\repo\\file", "C:/repo/*"));
    }

    #[test]
    fn trailing_space_star_is_optional() {
        // "git *" juga cocok persis "git" tanpa trailing space
        assert!(r#match("git", "git *"));
        assert!(r#match("git ", "git *"));
        assert!(r#match("git push", "git *"));
        assert!(!r#match("gitx", "git *"));
    }

    #[test]
    fn question_mark_matches_single_char() {
        assert!(r#match("ab", "a?"));
        assert!(!r#match("abc", "a?"));
        assert!(r#match("a\nb", "a?b")); // flag "s"
    }
}
