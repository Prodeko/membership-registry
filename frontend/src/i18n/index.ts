import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import fi from "./locales/fi.json";
import en from "./locales/en.json";

const savedLang = localStorage.getItem("lang") || "fi";

i18n.use(initReactI18next).init({
  resources: {
    fi: { translation: fi },
    en: { translation: en },
  },
  lng: savedLang,
  fallbackLng: "fi",
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;
