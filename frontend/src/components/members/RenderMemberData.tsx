import { Member } from "@/common/types";
import { useTranslation } from "react-i18next";

const RenderMemberData = ({
  member,
  variant,
}: {
  member: Member;
  variant: "enduser" | "admin";
}) => {
  const { t } = useTranslation();

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-4">
        <div>{t("profile.fields.name")}:</div>
        <div>{member.full_name}</div>
        {variant === "admin" && (
          <>
            <div>User id:</div>
            <div>{member.user_id}</div>
          </>
        )}
        <div>{t("profile.fields.email")}:</div>
        <div>{member.email}</div>
        <div>{t("profile.fields.home_municipality")}:</div>
        <div>{member.home_municipality ?? "-"}</div>
        <div>{t("member.accepted_policies")}:</div>
        <div>
          {member.has_accepted_policies ? t("common.yes") : t("common.no")}
        </div>
      </div>
    </div>
  );
};

export default RenderMemberData;
