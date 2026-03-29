#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleName(pub String);

impl From<RoleName> for String {
    fn from(name: RoleName) -> Self {
        name.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub name: RoleName,
    pub color: Option<String>,
    pub description: Option<String>,
    pub renewable: bool,
    pub renewal_payment_link: Option<String>,
    pub renewal_period_months: Option<i32>,
    pub renewal_email_template: Option<String>,
    pub renewal_notification_days: Vec<i32>,
}
