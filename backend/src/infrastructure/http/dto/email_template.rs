use serde::Serialize;
use ts_rs::TS;

use crate::domain::EmailTemplate;

#[derive(Serialize, TS)]
#[ts(export)]
pub struct EmailTemplateDTO {
    pub name: String,
    pub subject: String,
    pub body_html: String,
}

impl From<EmailTemplate> for EmailTemplateDTO {
    fn from(t: EmailTemplate) -> Self {
        Self {
            name: t.name,
            subject: t.subject,
            body_html: t.body_html,
        }
    }
}
