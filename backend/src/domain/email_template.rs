pub struct EmailTemplate {
    pub name: String,
}

pub struct EmailTemplateTranslation {
    pub template_name: String,
    pub locale: String,
    pub subject: String,
    pub body_html: String,
}
