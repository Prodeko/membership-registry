use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{EmailTemplate, EmailTemplateTranslation};

#[derive(Serialize, TS)]
#[ts(export)]
pub struct EmailTemplateDTO {
    pub name: String,
}

impl From<EmailTemplate> for EmailTemplateDTO {
    fn from(t: EmailTemplate) -> Self {
        Self { name: t.name }
    }
}

#[derive(Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailTemplateTranslationDTO {
    pub template_name: String,
    pub locale: String,
    pub subject: String,
    pub body_html: String,
}

impl From<EmailTemplateTranslation> for EmailTemplateTranslationDTO {
    fn from(t: EmailTemplateTranslation) -> Self {
        Self {
            template_name: t.template_name,
            locale: t.locale,
            subject: t.subject,
            body_html: t.body_html,
        }
    }
}
