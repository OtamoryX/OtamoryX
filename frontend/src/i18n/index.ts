import { createI18n } from "vue-i18n";
import enUS from "./locales/en-US";
import appearance from "./locales/appearance";
import zhCN from "./locales/zh-CN";

export const i18n = createI18n({
  legacy: false,
  locale: "zh-CN",
  fallbackLocale: "zh-CN",
  messages: {
    "zh-CN": { ...zhCN, appearance: appearance["zh-CN"] },
    "en-US": { ...enUS, appearance: appearance["en-US"] },
  },
});
