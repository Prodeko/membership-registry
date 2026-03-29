use crate::application::ports::html_sanitizer_port::HtmlSanitizerPort;

pub struct AmmoniaSanitizer;

impl HtmlSanitizerPort for AmmoniaSanitizer {
    fn sanitize(&self, html: &str) -> String {
        ammonia::clean(html)
    }
}
