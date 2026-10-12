pub(super) const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
                   img-src * data: blob:; font-src 'self' data:; connect-src 'self'; \
                   base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

pub(super) const PAGE: &[u8] = include_bytes!("../../../assets/preview/index.html");
pub(super) const APP: &[u8] = include_bytes!("../../../assets/preview/app.js");

const LIBS: &[(&str, &[u8])] = &[
    (
        "markdown-it.js",
        include_bytes!("../../../assets/preview/lib/markdown-it.js"),
    ),
    (
        "markdown-it-anchor.js",
        include_bytes!("../../../assets/preview/lib/markdown-it-anchor.js"),
    ),
    (
        "markdown-it-footnote.js",
        include_bytes!("../../../assets/preview/lib/markdown-it-footnote.js"),
    ),
    (
        "markdown-it-task-lists.js",
        include_bytes!("../../../assets/preview/lib/markdown-it-task-lists.js"),
    ),
    (
        "highlight.js",
        include_bytes!("../../../assets/preview/lib/highlight.js"),
    ),
    (
        "mermaid.js",
        include_bytes!("../../../assets/preview/lib/mermaid.js"),
    ),
    (
        "purify.js",
        include_bytes!("../../../assets/preview/lib/purify.js"),
    ),
    (
        "js-yaml.js",
        include_bytes!("../../../assets/preview/lib/js-yaml.js"),
    ),
];

pub(super) fn lib(name: &str) -> Option<&'static [u8]> {
    LIBS.iter()
        .find(|(lib, _)| *lib == name)
        .map(|(_, bytes)| *bytes)
}
