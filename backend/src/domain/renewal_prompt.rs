/// Per-role, per-locale renewal banner texts with {var} placeholders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenewalPrompt {
    pub locale: String,
    pub title: String,
    pub body: String,
    pub button_label: String,
}

impl RenewalPrompt {
    /// Substitute `{key}` placeholders in title, body and button label.
    /// Unknown placeholders are left as-is.
    pub fn rendered(&self, vars: &[(&str, &str)]) -> RenewalPrompt {
        let render = |s: &str| {
            let mut out = s.to_string();
            for (key, value) in vars {
                out = out.replace(&format!("{{{key}}}"), value);
            }
            out
        };
        RenewalPrompt {
            locale: self.locale.clone(),
            title: render(&self.title),
            body: render(&self.body),
            button_label: render(&self.button_label),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(title: &str, body: &str, button: &str) -> RenewalPrompt {
        RenewalPrompt {
            locale: "fi".to_string(),
            title: title.to_string(),
            body: body.to_string(),
            button_label: button.to_string(),
        }
    }

    #[test]
    fn substitutes_all_fields() {
        let p = prompt(
            "Jäsenmaksu vuodelle {year}",
            "Et ole maksanut jäsenmaksua vuodelle {year}. Jäsenyys päättyy {valid_until}.",
            "Maksa {role_name}-maksu",
        );
        let r = p.rendered(&[
            ("year", "2027"),
            ("valid_until", "31.12.2026"),
            ("role_name", "membership"),
        ]);
        assert_eq!(r.title, "Jäsenmaksu vuodelle 2027");
        assert_eq!(
            r.body,
            "Et ole maksanut jäsenmaksua vuodelle 2027. Jäsenyys päättyy 31.12.2026."
        );
        assert_eq!(r.button_label, "Maksa membership-maksu");
    }

    #[test]
    fn leaves_unknown_placeholders_untouched() {
        let p = prompt("{unknown}", "text", "btn");
        let r = p.rendered(&[("year", "2027")]);
        assert_eq!(r.title, "{unknown}");
    }
}
