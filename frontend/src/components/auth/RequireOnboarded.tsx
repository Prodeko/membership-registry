import { useGetMeMember } from "@/lib/api";
import { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Navigate } from "react-router-dom";

const RequireOnboarded = ({ children }: { children: ReactNode }) => {
  const { t } = useTranslation();
  const { data: member, isLoading } = useGetMeMember();

  if (isLoading) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">{t("common.loading")}</p>
      </main>
    );
  }

  if (member && !member.home_municipality) {
    return <Navigate to="/onboarding" replace />;
  }

  return <>{children}</>;
};

export default RequireOnboarded;
