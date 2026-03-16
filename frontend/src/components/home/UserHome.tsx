import {
  useGetMeMember,
  useGetMemberRoles,
  useGetPublicConfig,
  useGetTargetableRoles,
  useGetUserApplications,
  useWithdrawApplication,
} from "@/lib/api";
import { kebabCaseToTitleCase } from "@/lib/utils";
import { useLanguageSync } from "@/i18n/useLanguageSync";
import { Clock, ExternalLink, FileText, Pencil, Shield } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "../ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "../ui/dialog";
import { Separator } from "../ui/separator";
import { LanguageSwitcher } from "../language-switcher/LanguageSwitcher";

const statusVariant = {
  approved: "default" as const,
  rejected: "destructive" as const,
  pending: "secondary" as const,
  unpaid: "outline" as const,
};

const UserHome = () => {
  const { t } = useTranslation();
  useLanguageSync();

  const { data: member, isLoading: isMemberLoading } = useGetMeMember();
  const { data: applications, isLoading: isAppsLoading } =
    useGetUserApplications();
  const { data: targetableRoles } = useGetTargetableRoles();
  const { data: roles } = useGetMemberRoles(member?.user_id ?? "", {
    enabled: !!member,
  });
  const { data: publicConfig } = useGetPublicConfig();
  const { mutate: withdrawApplication } = useWithdrawApplication();
  const [withdrawId, setWithdrawId] = useState<string | null>(null);

  if (isMemberLoading || isAppsLoading) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">{t("common.loading")}</p>
      </main>
    );
  }

  if (!member) {
    return (
      <main className="flex justify-center min-h-screen w-screen px-4 py-20">
        <p className="text-muted-foreground">
          {t("errors.profile_load_failed")}
        </p>
      </main>
    );
  }

  return (
    <main className="flex justify-center min-h-screen w-screen px-4 py-20">
      <div className="max-w-2xl w-full space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold">
              {t("home.welcome", { name: member.first_name })}
            </h1>
            <p className="text-muted-foreground mt-1">{t("home.subtitle")}</p>
          </div>
          <LanguageSwitcher />
        </div>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <FileText className="h-5 w-5" />
              {t("home.applications.title")}
            </CardTitle>
            <CardDescription>
              {t("home.applications.description")}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {!applications || applications.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {t("home.applications.empty")}
              </p>
            ) : (
              <div className="space-y-3">
                {applications.map((app) => {
                  const role = targetableRoles?.find(
                    (r) =>
                      r.role_name === app.role_name &&
                      r.valid_until === app.valid_until,
                  );
                  return (
                    <div key={app.application_id}>
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="font-medium">
                            {kebabCaseToTitleCase(app.role_name)}
                          </p>
                          <p className="text-sm text-muted-foreground flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {t("home.applications.valid_until", {
                              date: new Date(
                                app.valid_until,
                              ).toLocaleDateString(),
                            })}
                          </p>
                        </div>
                        <div className="flex items-center gap-2">
                          {role?.payment_link && app.status === "unpaid" && (
                            <a
                              href={role.payment_link}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="text-sm text-blue-500 hover:underline"
                            >
                              {t("home.applications.pay_fee")}
                            </a>
                          )}
                          {app.status === "unpaid" && (
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => setWithdrawId(app.application_id)}
                            >
                              {t("home.applications.withdraw")}
                            </Button>
                          )}
                          <Badge variant={statusVariant[app.status]}>
                            {t(`status.${app.status}`)}
                          </Badge>
                        </div>
                      </div>
                      <Separator className="mt-3" />
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-lg flex items-center gap-2">
              <Shield className="h-5 w-5" />
              {t("home.roles.title")}
            </CardTitle>
            <CardDescription>{t("home.roles.description")}</CardDescription>
          </CardHeader>
          <CardContent>
            {!roles || roles.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {t("home.roles.empty")}
              </p>
            ) : (
              <div className="space-y-3">
                {roles.map((role) => {
                  const now = new Date();
                  const validUntil = role.valid_until
                    ? new Date(role.valid_until)
                    : null;
                  const isExpired = validUntil && validUntil < now;
                  return (
                    <div key={`${role.role_name}-${role.valid_from}`}>
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="font-medium">
                            {kebabCaseToTitleCase(role.role_name)}
                          </p>
                          <p className="text-sm text-muted-foreground flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {new Date(role.valid_from).toLocaleDateString()}
                            {" - "}
                            {validUntil
                              ? validUntil.toLocaleDateString()
                              : t("home.roles.no_expiry")}
                          </p>
                        </div>
                        <Badge variant={isExpired ? "destructive" : "default"}>
                          {isExpired ? t("status.expired") : t("status.active")}
                        </Badge>
                      </div>
                      <Separator className="mt-3" />
                    </div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-lg">
                {t("home.profile.title")}
              </CardTitle>
              <Button variant="ghost" size="sm" asChild>
                <Link to="/profile/edit">
                  <Pencil className="h-4 w-4 mr-1" />
                  {t("profile.edit_button")}
                </Link>
              </Button>
            </div>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
              <span className="text-muted-foreground">
                {t("profile.fields.name")}
              </span>
              <span>{member.full_name}</span>
              <span className="text-muted-foreground">
                {t("profile.fields.email")}
              </span>
              <span>{member.email}</span>
              <span className="text-muted-foreground">
                {t("profile.fields.municipality")}
              </span>
              <span>{member.home_municipality}</span>
              <span className="text-muted-foreground">
                {t("profile.fields.email_notifications")}
              </span>
              <span>
                {member.email_notifications
                  ? t("profile.fields.enabled")
                  : t("profile.fields.disabled")}
              </span>
            </div>
            {publicConfig?.keycloak_account_url && (
              <a
                href={publicConfig.keycloak_account_url}
                target="_blank"
                rel="noopener noreferrer"
                className="mt-4 inline-flex items-center gap-1 text-sm text-blue-500 hover:underline"
              >
                {t("profile.account_settings")}
                <ExternalLink className="h-3 w-3" />
              </a>
            )}
          </CardContent>
        </Card>

        <Button asChild className="w-full">
          <Link to="/apply">{t("home.apply_button")}</Link>
        </Button>
      </div>

      <Dialog
        open={withdrawId !== null}
        onOpenChange={(open) => !open && setWithdrawId(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("dialogs.withdraw.title")}</DialogTitle>
            <DialogDescription>
              {t("dialogs.withdraw.description")}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild>
              <Button variant="outline">{t("dialogs.cancel")}</Button>
            </DialogClose>
            <Button
              variant="destructive"
              onClick={() => {
                if (withdrawId) {
                  withdrawApplication(withdrawId, {
                    onSuccess: () => setWithdrawId(null),
                  });
                }
              }}
            >
              {t("dialogs.withdraw.confirm")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </main>
  );
};

export default UserHome;
