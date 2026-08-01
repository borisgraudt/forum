/// Build a URL-safe slug from free text.
/// Supports basic Cyrillic → Latin transliteration so Russian names don't
/// collapse to the same `"item"` fallback.
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;

    for ch in input.chars() {
        if let Some(mapped) = translit_char(ch) {
            for c in mapped.chars() {
                if c == '-' {
                    if !prev_dash && !out.is_empty() {
                        out.push('-');
                        prev_dash = true;
                    }
                } else {
                    out.push(c);
                    prev_dash = false;
                }
            }
        } else if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }

    if out.is_empty() {
        "category".into()
    } else {
        out.chars().take(80).collect()
    }
}

fn translit_char(ch: char) -> Option<&'static str> {
    let lower = ch.to_lowercase().next().unwrap_or(ch);
    match lower {
        'а' => Some("a"),
        'б' => Some("b"),
        'в' => Some("v"),
        'г' => Some("g"),
        'д' => Some("d"),
        'е' | 'ё' => Some("e"),
        'ж' => Some("zh"),
        'з' => Some("z"),
        'и' | 'й' => Some("i"),
        'к' => Some("k"),
        'л' => Some("l"),
        'м' => Some("m"),
        'н' => Some("n"),
        'о' => Some("o"),
        'п' => Some("p"),
        'р' => Some("r"),
        'с' => Some("s"),
        'т' => Some("t"),
        'у' => Some("u"),
        'ф' => Some("f"),
        'х' => Some("h"),
        'ц' => Some("ts"),
        'ч' => Some("ch"),
        'ш' => Some("sh"),
        'щ' => Some("sch"),
        'ъ' | 'ь' => Some(""),
        'ы' => Some("y"),
        'э' => Some("e"),
        'ю' => Some("yu"),
        'я' => Some("ya"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("  Foo   Bar  "), "foo-bar");
        assert_eq!(slugify("***"), "category");
    }

    #[test]
    fn slugify_cyrillic() {
        assert_eq!(
            slugify("Информационная безопасность"),
            "informatsionnaya-bezopasnost"
        );
        assert_eq!(slugify("Сети"), "seti");
        assert_ne!(
            slugify("Коммутации"),
            slugify("Информационная безопасность")
        );
    }
}
