//! A metric counting up from zero in the value's own format: its prefix and suffix, decimals and
//! digit grouping.

/// Characters that group digits or mark the decimals inside a number.
const SEPARATORS: [char; 5] = [' ', '\u{a0}', '\u{202f}', ',', '.'];

/// `value` (as written: `1 200`, `60 %`, `3.5x`, `$1,200`) at `p` (0 to 1) of its count-up; the
/// value itself at 1, unchanged when it holds no number.
pub(crate) fn count(value: &str, p: f64) -> String {
    let Some(start) = value.find(|c: char| c.is_ascii_digit()) else {
        return value.to_string();
    };
    if p >= 1.0 {
        return value.to_string();
    }
    // the number: digits, and separators between digits
    let chars: Vec<(usize, char)> = value[start..].char_indices().collect();
    let mut end = 0;
    for (i, &(at, c)) in chars.iter().enumerate() {
        let inner =
            SEPARATORS.contains(&c) && chars.get(i + 1).is_some_and(|(_, d)| d.is_ascii_digit());
        if !(c.is_ascii_digit() || inner) {
            break;
        }
        end = at + c.len_utf8();
    }
    let number = &value[start..start + end];
    // a last '.' or ',' not followed by three digits marks the decimals
    let point = number
        .rfind(['.', ','])
        .filter(|&at| number.len() - at - 1 != 3);
    let (whole, decimals) = point.map_or((number, ""), |at| (&number[..at], &number[at + 1..]));
    let group = whole.chars().find(|c| !c.is_ascii_digit());
    let digits = |s: &str| s.chars().filter(char::is_ascii_digit).collect::<String>();
    let target: f64 = format!("{}.{}0", digits(whole), digits(decimals))
        .parse()
        .unwrap_or(0.0);
    let now = format!("{:.*}", decimals.len(), target * p.max(0.0));
    let (int, frac) = now.split_once('.').unwrap_or((&now, ""));
    let mut out = value[..start].to_string();
    for (i, d) in int.chars().enumerate() {
        if let Some(g) = group.filter(|_| i > 0 && (int.len() - i).is_multiple_of(3)) {
            out.push(g);
        }
        out.push(d);
    }
    if let Some(at) = point {
        out.push_str(&number[at..=at]);
        out.push_str(frac);
    }
    out + &value[start + end..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_keeps_the_format_and_lands_on_the_value() {
        assert_eq!(count("1 200", 0.5), "600");
        assert_eq!(count("1 200", 0.0), "0");
        assert_eq!(count("12 000 kg", 0.5), "6 000 kg");
        assert_eq!(count("$1,200,000", 0.25), "$300,000");
        assert_eq!(count("60 %", 0.5), "30 %");
        assert_eq!(count("3.5x", 0.6), "2.1x");
        assert_eq!(count("3,5", 0.0), "0,0");
        for value in ["1 200", "60 %", "3.5x", "none", "v2"] {
            assert_eq!(count(value, 1.0), value);
        }
    }
}
