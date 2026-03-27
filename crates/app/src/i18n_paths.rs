use crate::i18n::Locale;

pub fn lp(_locale: Locale, href: &str) -> String {
    let h = href.trim();

    if h.is_empty() {
        return "/".to_string();
    }

    if h.starts_with("http://")
        || h.starts_with("https://")
        || h.starts_with("mailto:")
        || h.starts_with("tel:")
        || h.starts_with('#')
    {
        return h.to_string();
    }

    let p = h.trim_start_matches('/');

    if p.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", p)
    }
}
