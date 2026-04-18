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
        <div>{t("profile.fields.first_name")}:</div>
        <div>{member.first_name}</div>
        <div>{t("profile.fields.last_name")}:</div>
        <div>{member.last_name}</div>
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
      </div>
    </div>
  );
};

export default RenderMemberData;
