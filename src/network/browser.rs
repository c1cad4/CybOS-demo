pub use cyb_browser::*;

/// Turn address-bar input into a URL without interpreting shell syntax.
pub(crate) fn resolve_input(input: &str) -> String {
    let input = input.trim();
    if input.contains("://") { return input.into(); }
    if !input.contains(char::is_whitespace) && input.contains('.') {
        return format!("https://{input}");
    }
    let encoded: String = input.as_bytes().iter().map(|byte| {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(byte) { (*byte as char).to_string() }
        else { format!("%{byte:02X}") }
    }).collect();
    format!("https://duckduckgo.com/?q={encoded}")
}
#[cfg(test)]
mod tests {
    #[test]
    fn address_and_unicode_search_inputs() {
        assert_eq!(super::resolve_input("example.com"), "https://example.com");
        assert_eq!(super::resolve_input("https://example.com/a"), "https://example.com/a");
        assert_eq!(super::resolve_input("a & б"), "https://duckduckgo.com/?q=a%20%26%20%D0%B1");
    }
}
