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

  const hasActiveApplicationForAllRoles = (() => {
    if (!targetableRoles || targetableRoles.length === 0) return true;
    const activeTargetableRoles = targetableRoles.filter((r) => r.active);
    if (activeTargetableRoles.length === 0) return true;
    return activeTargetableRoles.every((role) => {
      const hasApplication = applications?.some(
        (app) =>
          app.status !== "rejected" &&
          app.role_name === role.role_name &&
          app.valid_until === role.valid_until,
      );
      const hasActiveRole = roles?.some((r) => {
        if (r.role_name !== role.role_name) return false;
        const validUntil = r.valid_until ? new Date(r.valid_until) : null;
        return !validUntil || validUntil >= new Date();
      });
      return hasApplication || hasActiveRole;
    });
  })();

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

        {applications && applications.length > 0 && (
          <Card data-testid="home-applications-card">
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
              <div className="space-y-3">
                {applications.map((app) => {
                  const role = targetableRoles?.find(
                    (r) =>
                      r.role_name === app.role_name &&
                      r.valid_until === app.valid_until,
                  );
                  return (
                    <div key={app.application_id} data-testid={`application-row-${app.application_id}`}>
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="font-medium" data-testid="application-role-name">
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
                              data-testid={`withdraw-button-${app.application_id}`}
                              onClick={() => setWithdrawId(app.application_id)}
                            >
                              {t("home.applications.withdraw")}
                            </Button>
                          )}
                          <Badge variant={statusVariant[app.status]} data-testid={`application-status-${app.application_id}`}>
                            {t(`status.${app.status}`)}
                          </Badge>
                        </div>
                      </div>
                      <Separator className="mt-3" />
                    </div>
                  );
                })}
              </div>
            </CardContent>
          </Card>
        )}

        <Card data-testid="home-roles-card">
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
                {(() => {
                  const now = new Date();
                  const grouped = new Map<
                    string,
                    {
                      role_name: string;
                      validFrom: Date;
                      validUntil: Date | null;
                      renewable: boolean;
                      renewal_payment_link: string | null;
                      pending_renewal_id: string | null;
                    }
                  >();

                  for (const role of roles) {
                    const vf = new Date(role.valid_from);
                    const vu = role.valid_until
                      ? new Date(role.valid_until)
                      : null;
                    const existing = grouped.get(role.role_name);

                    if (!existing) {
                      grouped.set(role.role_name, {
                        role_name: role.role_name,
                        validFrom: vf,
                        validUntil: vu,
                        renewable: role.renewable,
                        renewal_payment_link: role.renewal_payment_link,
                        pending_renewal_id: role.pending_renewal_id,
                      });
                    } else {
                      if (vf < existing.validFrom) existing.validFrom = vf;
                      if (vu === null) {
                        existing.validUntil = null;
                      } else if (
                        existing.validUntil !== null &&
                        vu > existing.validUntil
                      ) {
                        existing.validUntil = vu;
                      }
                      if (role.pending_renewal_id) {
                        existing.pending_renewal_id = role.pending_renewal_id;
                        existing.renewal_payment_link =
                          role.renewal_payment_link;
                      }
                    }
                  }

                  return [...grouped.values()].map((role) => {
                    const isExpired =
                      role.validUntil && role.validUntil < now;
                    const daysUntilExpiry = role.validUntil
                      ? Math.ceil(
                          (role.validUntil.getTime() - now.getTime()) /
                            (1000 * 60 * 60 * 24),
                        )
                      : null;
                    const isExpiringSoon =
                      role.renewable &&
                      !isExpired &&
                      daysUntilExpiry !== null &&
                      daysUntilExpiry <= 30;
                    return (
                      <div key={role.role_name} data-testid={`role-row-${role.role_name}`}>
                        <div className="flex items-center justify-between">
                          <div>
                            <p className="font-medium" data-testid="role-name">
                              {kebabCaseToTitleCase(role.role_name)}
                            </p>
                            <p className="text-sm text-muted-foreground flex items-center gap-1">
                              <Clock className="h-3 w-3" />
                              {role.validFrom.toLocaleDateString()}
                              {" - "}
                              {role.validUntil
                                ? role.validUntil.toLocaleDateString()
                                : t("home.roles.no_expiry")}
                            </p>
                          </div>
                          <Badge
                            variant={isExpired ? "destructive" : "default"}
                            data-testid={`role-status-${role.role_name}`}
                          >
                            {isExpired
                              ? t("status.expired")
                              : t("status.active")}
                          </Badge>
                        </div>
                        {isExpiringSoon && (
                          <div className="flex items-center justify-between mt-2 text-sm text-orange-600" data-testid={`role-expiring-warning-${role.role_name}`}>
                            <span>
                              {t("home.roles.expiring_soon", {
                                days: daysUntilExpiry,
                              })}
                            </span>
                            {role.renewal_payment_link &&
                              role.pending_renewal_id && (
                                <a
                                  href={`${role.renewal_payment_link}?client_reference_id=${role.pending_renewal_id}`}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="text-blue-500 hover:underline"
                                  data-testid={`role-renewal-link-${role.role_name}`}
                                >
                                  {t("home.roles.renew")}
                                </a>
                              )}
                          </div>
                        )}
                        <Separator className="mt-3" />
                      </div>
                    );
                  });
                })()}
              </div>
            )}
          </CardContent>
        </Card>

        <Card data-testid="home-profile-card">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle className="text-lg">
                {t("home.profile.title")}
              </CardTitle>
              <Button variant="ghost" size="sm" asChild data-testid="edit-profile-button">
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
              <span data-testid="profile-name-value">{member.full_name}</span>
              <span className="text-muted-foreground">
                {t("profile.fields.email")}
              </span>
              <span>{member.email}</span>
              <span className="text-muted-foreground">
                {t("profile.fields.municipality")}
              </span>
              <span data-testid="profile-municipality-value">{member.home_municipality ?? "-"}</span>
              <span className="text-muted-foreground">
                {t("profile.fields.email_notifications")}
              </span>
              <span data-testid="profile-notifications-value">
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

        {!hasActiveApplicationForAllRoles && (
          <Button asChild className="w-full" data-testid="apply-button">
            <Link to="/apply">{t("home.apply_button")}</Link>
          </Button>
        )}
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
              <Button variant="outline" data-testid="withdraw-cancel-button">{t("dialogs.cancel")}</Button>
            </DialogClose>
            <Button
              variant="destructive"
              data-testid="withdraw-confirm-button"
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
