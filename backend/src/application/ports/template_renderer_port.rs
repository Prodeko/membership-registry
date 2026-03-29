pub trait TemplateRendererPort: Send + Sync {
    fn render(&self, template: &str, variables: &[(&str, &str)]) -> String;
}
