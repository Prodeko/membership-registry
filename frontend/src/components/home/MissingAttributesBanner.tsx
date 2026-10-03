import { MemberAttribute } from "@/common/types";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Button } from "../ui/button";
import { Card, CardContent } from "../ui/card";

interface MissingAttributesBannerProps {
  attributes: MemberAttribute[];
}

/** Asks the member to fill required attributes they can edit but lack. */
const MissingAttributesBanner = ({
  attributes,
}: MissingAttributesBannerProps) => {
  const { t } = useTranslation();

  return (
    <Card
      className="border-0 bg-primary"
      data-testid="missing-attributes-banner"
    >
      <CardContent className="flex flex-col gap-4 p-6 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <p className="font-semibold text-primary-foreground">
            {t("home.missing_attributes.title")}
          </p>
          <p className="text-sm text-primary-foreground/80">
            {t("home.missing_attributes.description", {
              names: attributes.map((a) => a.name).join(", "),
            })}
          </p>
        </div>
        <Button
          asChild
          className="w-full bg-white text-primary hover:bg-white/90 sm:w-auto sm:shrink-0"
          data-testid="missing-attributes-button"
        >
          <Link to="/profile/edit">
            {t("home.missing_attributes.fill_button")}
          </Link>
        </Button>
      </CardContent>
    </Card>
  );
};

export default MissingAttributesBanner;
