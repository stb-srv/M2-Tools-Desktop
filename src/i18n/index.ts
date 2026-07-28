import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import de from "./locales/de.json";

const STORAGE_KEY = "m2manager-language";

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    de: { translation: de },
  },
  lng: localStorage.getItem(STORAGE_KEY) ?? "de",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

i18n.on("languageChanged", (lng) => {
  localStorage.setItem(STORAGE_KEY, lng);
});

export default i18n;
