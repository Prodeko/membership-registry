import { useStartRenewal } from "@/lib/api";
import { RenewalPrompt } from "@/common/types";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/button";
import { Card, CardContent } from "../ui/card";

interface RenewalBannerProps {
  roleName: string;
  validUntil: Date;
  renewalDeadline: Date | null;
  prompts: RenewalPrompt[];
}

const RenewalBanner = ({
  roleName,
  validUntil,
  renewalDeadline,
  prompts,
}: RenewalBannerProps) => {
  const { t, i18n } = useTranslation();
  const { mutate: startRenewal, isPending } = useStartRenewal();
  const isExpired = validUntil < new Date();

  const language = i18n.language.split("-")[0];
  const prompt = prompts.find((p) => p.locale === language);
  const title = prompt?.title ?? t("home.renewal.title");
  const body =
    prompt?.body ??
    (isExpired
      ? t("home.renewal.expired_text", {
          date: validUntil.toLocaleDateString(),
          deadline: renewalDeadline?.toLocaleDateString(),
        })
      : t("home.renewal.expires_text", {
          date: validUntil.toLocaleDateString(),
        }));
  const buttonLabel = prompt?.button_label ?? t("home.renewal.renew_button");

  return (
    <Card
      className="border-0 bg-primary"
      data-testid={`renewal-banner-${roleName}`}
    >
      <CardContent className="flex items-center justify-between gap-4 p-6">
        <div>
          <p className="font-semibold text-primary-foreground">{title}</p>
          <p className="text-sm text-primary-foreground/80">{body}</p>
        </div>
        <Button
          disabled={isPending}
          className="shrink-0 bg-white text-primary hover:bg-white/90"
          data-testid={`renewal-banner-button-${roleName}`}
          onClick={() =>
            startRenewal(roleName, {
              onSuccess: (data) => window.location.assign(data.payment_url),
            })
          }
        >
          {buttonLabel}
        </Button>
      </CardContent>
    </Card>
  );
};

export default RenewalBanner;
