import {
  useGetMarketingPreferences,
  useUpdateMarketingPreferences,
} from "@/lib/api";
import { SubscriptionState, TagPreference } from "@/common/types";
import { Card } from "../ui/card";
import { Button } from "../ui/button";
import { Switch } from "../ui/switch";
import { Label } from "../ui/label";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

const MarketingPreferences = () => {
  const { t } = useTranslation();
  const { data: prefs, isLoading, error } = useGetMarketingPreferences();
  const { mutate: updatePrefs, isPending } = useUpdateMarketingPreferences();

  if (isLoading) {
    return (
      <Card className="p-10 space-y-4">
        <h2 className="text-xl font-bold">{t("profile.marketing.title")}</h2>
        <p className="text-muted-foreground">{t("common.loading")}</p>
      </Card>
    );
  }

  if (error || !prefs) {
    return (
      <Card className="p-10 space-y-4">
        <h2 className="text-xl font-bold">{t("profile.marketing.title")}</h2>
        <p className="text-muted-foreground">
          {t("profile.marketing.load_error")}
        </p>
      </Card>
    );
  }

  const stateLabel = (state: SubscriptionState): string => {
    switch (state) {
      case "subscribed":
        return t("profile.marketing.state_subscribed");
      case "pending":
        return t("profile.marketing.state_pending");
      case "unsubscribed":
        return t("profile.marketing.state_unsubscribed");
      case "not_a_contact":
        return t("profile.marketing.state_not_a_contact");
    }
  };

  const handleSubscribe = () => {
    updatePrefs(
      { type: "subscribe" },
      {
        onSuccess: () => {
          toast.info(t("profile.marketing.pending_hint"), { duration: 10000 });
        },
        onError: () => toast.error(t("profile.marketing.save_error")),
      },
    );
  };

  // No unsubscribe-all button: users unsubscribe from everything via the
  // Mailchimp email footer. Individual tag toggles below are the only
  // opt-out affordance in-app.

  const handleToggleTag = (tagName: string, active: boolean) => {
    const updatedTags: TagPreference[] = prefs.tags.map((t) =>
      t.name === tagName ? { ...t, active } : t,
    );
    updatePrefs(
      { type: "set_tags", tags: updatedTags },
      { onError: () => toast.error(t("profile.marketing.save_error")) },
    );
  };

  const isSubscribed = prefs.state === "subscribed";
  const isUnsubscribedOrMissing =
    prefs.state === "unsubscribed" || prefs.state === "not_a_contact";

  return (
    <Card className="p-10 space-y-4">
      <h2 className="text-xl font-bold">{t("profile.marketing.title")}</h2>
      <p className="text-muted-foreground text-sm">
        {t("profile.marketing.description")}
      </p>

      <div className="flex items-center gap-3">
        <span className="text-sm font-medium">{stateLabel(prefs.state)}</span>
      </div>

      <div>
        {isUnsubscribedOrMissing && (
          <Button onClick={handleSubscribe} disabled={isPending}>
            {t("profile.marketing.action_subscribe")}
          </Button>
        )}
        {prefs.state === "pending" && (
          <p className="text-sm text-muted-foreground">
            {t("profile.marketing.pending_hint")}
          </p>
        )}
      </div>

      {isSubscribed && (
        <div className="space-y-3 border-t pt-4">
          {prefs.tags.map((tag) => (
            <div key={tag.name} className="flex items-center justify-between">
              <Label htmlFor={`tag-${tag.name}`}>
                {t(`profile.marketing.tags.${tag.name}`)}
              </Label>
              <Switch
                id={`tag-${tag.name}`}
                checked={tag.active}
                disabled={isPending}
                onCheckedChange={(checked) =>
                  handleToggleTag(tag.name, checked)
                }
              />
            </div>
          ))}
        </div>
      )}
    </Card>
  );
};

export default MarketingPreferences;
