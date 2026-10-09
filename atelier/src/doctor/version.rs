pub(super) fn version(text: &str) -> Option<(u32, u32)> {
    let digits = text.trim_start_matches(|c: char| !c.is_ascii_digit());
    let mut parts = digits.split(|c: char| !c.is_ascii_digit());
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

pub(super) fn dotted((major, minor): (u32, u32)) -> String {
    format!("{major}.{minor}")
}
