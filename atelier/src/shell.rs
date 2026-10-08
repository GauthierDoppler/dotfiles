pub fn quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}
