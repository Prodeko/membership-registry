use crate::application::ports::template_renderer_port::TemplateRendererPort;

pub struct SimpleTemplateRenderer;

impl TemplateRendererPort for SimpleTemplateRenderer {
    fn render(&self, template: &str, variables: &[(&str, &str)]) -> String {
        let mut result = template.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("{{{key}}}"), &ammonia::clean(value));
        }
        result
    }
}
