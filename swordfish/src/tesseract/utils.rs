use swordfish_common::trace;

const ALLOWED_CHARS: [char; 14] = [
    ' ', '-', '.', '!', ':', '(', ')', '\'', '/', '\'', '@', '&', '_', 'é',
];
const REGEX_CHARS: [char; 4] = ['[', ']', ')', '('];

fn replace_string(text: &mut String, from: &str, to: &str) -> bool {
    match text.find(from) {
        Some(i) => {
            text.replace_range(i..i + from.len(), to);
            true
        }
        None => false,
    }
}

pub fn fix_tesseract_string(text: &mut String) {
    // Remove the \n
    trace!("Text: {}", text);
    if text.ends_with("\n") {
        text.pop();
    }
    // Workaround for a bug the text
    // e.g. "We Never Learn\nN" -> "We Never Learn"
    trace!("Text: {}", text);
    if text.ends_with("\nN") {
        text.truncate(text.len() - 2);
    }
    // Replace first (to prevent "byte index 13 is not a char boundary; it is inside '—' (bytes 11..14)")
    while replace_string(text, "—", "-") {
        trace!("Replacing '—' with '-'");
    }
    // Workaround for a bug the text
    trace!("Text: {}", text);
    if text.starts_with("- ") || text.starts_with("-.") {
        text.drain(0..2);
    }
    // Remove the first character if it is not alphanumeric
    if !text.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        text.remove(0);
    }
    // Workaround IR -> Ik
    // Maybe it only occurs if Ik is in the start of the string?
    // e.g. "IReda" -> "Ikeda"
    trace!("Text: {}", text);
    replace_string(text, "IR", "Ik");
    // Workaround for "A\n"
    // This is usually the corner of the card
    trace!("Text: {}", text);
    replace_string(text, "A\n", "");
    // Workaround for '“NO'
    // This is usually the left bottom corner of the card
    trace!("Text: {}", text);
    if text.ends_with(r##"“NO"##) {
        text.drain(text.len() - 4..text.len());
    }
    // Workaround for "\n." (and others in the future)
    let text_clone = text.clone();
    let mut clone_chars = text_clone.chars();
    for (i, c) in clone_chars.clone().enumerate() {
        if c != '\n' {
            continue;
        }
        let prev_char = match clone_chars.nth(i - 1) {
            Some(c) => c,
            None => continue,
        };
        let mut rm_prev: i8 = 0;
        trace!("Prev char: {}", prev_char);
        if ['-'].contains(&prev_char) {
            rm_prev = 1;
            text.remove(i - 1);
        }
        // Fix for "Asobi ni Iku lo Asobi ni Oide" -> "Asobi ni Iku yo! Asobi ni Oide"
        if prev_char == 'l' {
            let prev_prev_char = match clone_chars.nth(i - 2) {
                Some(c) => c,
                None => continue,
            };
            trace!("Prev prev char: {}", prev_prev_char);
            if prev_prev_char == 'o' {
                rm_prev = -1;
                text.drain(i - 3..i - 1);
                text.insert_str(i - 2, "yo!")
            }
        }
        let next_char = match clone_chars.nth(i + 1) {
            Some(c) => c,
            None => break,
        };
        trace!("Next char: {}", next_char);
        if ['.'].contains(&next_char) {
            text.remove((i as i8 + 1 - rm_prev) as usize);
        }
    }
    // Replace "\n" with " "
    trace!("Text: {}", text);
    while replace_string(text, "\n", " ") {
        trace!("Replacing '\\n' with ' '");
    }
    // Remove all non-alphanumeric characters
    trace!("Text: {}", text);
    text.retain(|c| ALLOWED_CHARS.contains(&c) || c.is_ascii_alphanumeric());
    // Fix "mn" -> "III"
    trace!("Text: {}", text);
    if text.ends_with("mn") {
        text.pop();
        text.pop();
        text.push_str("III");
    }
    // Fix "1ll" -> "III"
    trace!("Text: {}", text);
    replace_string(text, "1ll", "III");
    // Fix "lll" -> "!!!"
    trace!("Text: {}", text);
    replace_string(text, "lll", "!!!");
    // Fix "Il" -> "II" in the end of the string
    trace!("Text: {}", text);
    if text.ends_with("Il") {
        text.pop();
        text.pop();
        text.push_str("II");
    }
    // Replace multiple spaces with one space
    trace!("Text: {}", text);
    while replace_string(text, "  ", " ") {
        trace!("Removing multiple spaces");
    }
    // Remove the last character if it is a dash
    if text.ends_with("-") {
        text.pop();
    }
    // Workaround if the first character is a space
    trace!("Text: {}", text);
    while text.starts_with(|c: char| c.is_whitespace()) {
        trace!("Removing leading space");
        text.remove(0);
    }
    // Workaround if the last character is a space
    trace!("Text: {}", text);
    while text.ends_with(|c: char| c.is_whitespace()) {
        trace!("Removing ending space");
        text.pop();
    }
    trace!("Text (final): {}", text);
}

pub fn regexify_text(text: &String) -> String {
    let partial_match: bool;
    let short_text = text.len() < 6;
    if text.len() > 23 {
        partial_match = true;
    } else {
        partial_match = false;
    }
    let mut regex = String::new();
    let mut ascii_text = String::new();
    let mut prev_chars: Vec<char> = Vec::new();
    for c in text.chars() {
        // Here comes the workaround...
        // The character "0" is sometimes used in place of "O" in names
        if ['0', 'O'].contains(&c) {
            ascii_text.push_str("[0O]");
        } else if ['u', 'v', 'y'].contains(&c) {
            ascii_text.push_str("[uvy]");
        } else if ['t'].contains(&c) {
            ascii_text.push_str("[ti]");
        } else if ['I', 'l', '!', '1'].contains(&c) {
            ascii_text.push_str("[Il!1i]");
        } else if ['.'].contains(&c) {
            if prev_chars.len() > 3 {
                let prev_char = prev_chars[prev_chars.len() - 1];
                let prev_prev_char = prev_chars[prev_chars.len() - 2];
                if prev_char.is_numeric() && prev_prev_char.is_whitespace() {
                    continue;
                }
            }
            ascii_text.push(' ');
        } else if ['R'].contains(&c) {
            ascii_text.push_str("[Rk]");
        } else if ['m'].contains(&c) {
            ascii_text.push_str("(m|ra)");
        } else if ['a'].contains(&c) {
            ascii_text.push_str("[ao]")
        } else if c.is_ascii_alphanumeric() {
            ascii_text.push(c);
        } else {
            ascii_text.push(' ');
        }
        prev_chars.push(c);
    }
    if ascii_text.ends_with(|c: char| c.is_ascii_digit()) {
        ascii_text.pop();
    }
    // Filter for short string.
    if short_text && !ascii_text.contains(|c: char| c.is_whitespace()) {
        ascii_text = ascii_text.to_lowercase();
        regex.push_str("^");
        let mut request_quantifier: bool = false;
        let mut regex_any: bool = false;
        let mut regex_any_from: usize = 0;
        for (i, char) in ascii_text.chars().enumerate() {
            trace!("Char: {}", char);
            if char == '[' {
                regex_any = true;
                regex_any_from = i;
                if i == 0 {
                    request_quantifier = true;
                }
                continue;
            } else if i == ascii_text.len() - 1 {
                regex.push_str(".*");
                regex.push(char);
                break;
            }
            if regex_any {
                if char == ']' {
                    regex_any = false;
                    regex.push_str(&ascii_text[regex_any_from..i + 1]);
                    if request_quantifier {
                        regex.push_str(".*");
                    }
                }
                continue;
            }
            regex.push(char);
            if i == 0 {
                regex.push_str(".*");
            }
        }
        regex.push_str("$");
        trace!("Regex (short string): {}", regex);
        return regex;
    }
    let split = ascii_text.split_whitespace();
    let len = split.clone().count();
    trace!("Partial match: {}", partial_match);
    for (i, word) in split.enumerate() {
        if word.len() < 2 {
            if i > 0 && i < len - 1 {
                continue;
            }
            if ["x", "X"].contains(&word) {
                continue;
            }
        }
        regex.push_str("(?=.*");
        let processed_word = word.to_lowercase();
        trace!("Processed word: {}", processed_word);
        if partial_match && processed_word.len() > 4 {
            // Remove first two and last two characters for "partial match"
            if !processed_word[0..3].contains(|c: char| REGEX_CHARS.contains(&c))
                && !processed_word[word.len() - 2..word.len()]
                    .contains(|c: char| REGEX_CHARS.contains(&c))
            {
                regex.push_str(&processed_word[2..word.len() - 2]);
            } else {
                regex.push_str(&processed_word.as_str());
            }
        } else {
            // Do not push word boundary if the word contains special characters like "!"
            trace!("Current processed word: {}", processed_word);
            if processed_word.chars().all(|c| c.is_ascii_alphanumeric()) {
                regex.push_str(format!("\\b{}\\b", &processed_word.as_str()).as_str());
            } else {
                regex.push_str(format!("{}", &processed_word.as_str()).as_str());
            }
        }
        regex.push_str(")");
    }
    regex.push_str(".+");
    trace!("Regex: {}", regex);
    regex
}
