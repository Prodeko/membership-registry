import { useStartRenewal } from "@/lib/api";
import { kebabCaseToTitleCase } from "@/lib/utils";
import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/button";
import { Card, CardContent } from "../ui/card";

interface RenewalBannerProps {
  roleName: string;
  validUntil: Date;
  renewalDeadline: Date | null;
}

const RenewalBanner = ({
  roleName,
  validUntil,
  renewalDeadline,
}: RenewalBannerProps) => {
  const { t } = useTranslation();
  const { mutate: startRenewal, isPending } = useStartRenewal();
  const isExpired = validUntil < new Date();

  return (
    <Card
      className="border-orange-500"
      data-testid={`renewal-banner-${roleName}`}
    >
      <CardContent className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <AlertTriangle className="h-6 w-6 shrink-0 text-orange-600" />
          <div>
            <p className="font-semibold">
              {t("home.renewal.title", {
                role: kebabCaseToTitleCase(roleName),
              })}
            </p>
            <p className="text-sm text-muted-foreground">
              {isExpired
                ? t("home.renewal.expired_text", {
                    date: validUntil.toLocaleDateString(),
                    deadline: renewalDeadline?.toLocaleDateString(),
                  })
                : t("home.renewal.expires_text", {
                    date: validUntil.toLocaleDateString(),
                  })}
            </p>
          </div>
        </div>
        <Button
          disabled={isPending}
          data-testid={`renewal-banner-button-${roleName}`}
          onClick={() =>
            startRenewal(roleName, {
              onSuccess: (data) => window.location.assign(data.payment_url),
            })
          }
        >
          {t("home.renewal.renew_button")}
        </Button>
      </CardContent>
    </Card>
  );
};

export default RenewalBanner;
