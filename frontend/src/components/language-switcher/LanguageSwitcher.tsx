import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useUpdateMember, useGetMeMember } from "@/lib/api";

export function LanguageSwitcher() {
  const { i18n } = useTranslation();
  const { data: member } = useGetMeMember();
  const updateMember = useUpdateMember();

  const currentLang = i18n.language;
  const nextLang = currentLang === "fi" ? "en" : "fi";
  const label = nextLang === "fi" ? "Suomi" : "English";

  const switchLanguage = () => {
    i18n.changeLanguage(nextLang);
    localStorage.setItem("lang", nextLang);

    if (member) {
      updateMember.mutate({
        userId: member.user_id,
        data: {
          first_name: member.first_name,
          last_name: member.last_name,
          home_municipality: member.home_municipality,
          has_accepted_policies: member.has_accepted_policies,
          email_notifications: member.email_notifications,
          language: nextLang,
        },
      });
    }
  };

  return (
    <Button variant="ghost" size="sm" onClick={switchLanguage}>
      {label}
    </Button>
  );
}
