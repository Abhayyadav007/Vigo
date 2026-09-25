/// URL slug from free text: "Amul Taaza Milk (500 ml)" -> "amul-taaza-milk-500-ml".
/// Non-ASCII letters are dropped (Hindi names should set an explicit slug).
pub fn slugify(parts: &[&str]) -> String {
    let mut out = String::new();
    for part in parts {
        for ch in part.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
            } else if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        }
        if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_end_matches('-');
    trimmed
        .chars()
        .take(120)
        .collect::<String>()
        .trim_end_matches('-')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugs() {
        assert_eq!(
            slugify(&["Amul Taaza Milk (500 ml)"]),
            "amul-taaza-milk-500-ml"
        );
        assert_eq!(slugify(&["Amul", "Butter", "100 g"]), "amul-butter-100-g");
        assert_eq!(slugify(&["  Fruits & Vegetables  "]), "fruits-vegetables");
        assert_eq!(slugify(&["दूध"]), "");
    }
}
