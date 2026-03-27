use crate::i18n::Locale;
use leptos_i18n::Locale as LocaleTrait;

pub fn lp(locale: Locale, href: &str) -> String {
    let h = href.trim();

    if h.is_empty() {
        return format!("/{}/", locale.as_str());
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
        format!("/{}/", locale.as_str())
    } else {
        format!("/{}/{}", locale.as_str(), p)
    }
}
