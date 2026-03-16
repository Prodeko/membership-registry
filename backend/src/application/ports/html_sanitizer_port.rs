pub trait HtmlSanitizerPort: Send + Sync {
    fn sanitize(&self, html: &str) -> String;
}
