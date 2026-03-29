import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useGetMeMember } from "@/lib/api";

/**
 * Syncs i18n language from the authenticated user's profile.
 * Call once in a top-level authenticated layout/page.
 */
export function useLanguageSync() {
  const { i18n } = useTranslation();
  const { data: member } = useGetMeMember();

  useEffect(() => {
    if (member?.language && member.language !== i18n.language) {
      i18n.changeLanguage(member.language);
      localStorage.setItem("lang", member.language);
    }
  }, [member?.language, i18n]);
}
