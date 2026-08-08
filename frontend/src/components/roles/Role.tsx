import {
  QueryKey,
  useGetEmailTemplates,
  useGetRole,
  useGetRoleMembers,
  useUpdateRole,
} from "@/lib/api";
import { useNavigate, useParams, Link } from "react-router";
import { Card } from "../ui/card";
import { Button } from "../ui/button";
import RoleBadge from "../ui/role-badge";
import { Switch } from "../ui/switch";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import React from "react";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";

const Role = () => {
  const { id: roleName = "" } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const {
    data: role,
    isLoading: isRoleLoading,
    error: roleError,
  } = useGetRole(roleName, { enabled: roleName.length > 0 });

  const {
    data: members,
    isLoading: isMembersLoading,
    error: membersError,
  } = useGetRoleMembers(roleName, { enabled: roleName.length > 0 });

  const { data: emailTemplates } = useGetEmailTemplates();
  const { mutate: updateRole, isPending } = useUpdateRole();

  const [renewable, setRenewable] = React.useState(false);
  const [renewalPaymentLink, setRenewalPaymentLink] = React.useState("");
  const [renewalPeriodMonths, setRenewalPeriodMonths] = React.useState("12");
  const [renewalEmailTemplate, setRenewalEmailTemplate] = React.useState("");
  const [renewalWindowDays, setRenewalWindowDays] = React.useState("30");
  const [gracePeriodDays, setGracePeriodDays] = React.useState("0");
  const [notificationDays, setNotificationDays] = React.useState("30, 7, 1");
  const [initialized, setInitialized] = React.useState(false);

  type PromptDraft = { title: string; body: string; button_label: string };
  const emptyPrompt: PromptDraft = { title: "", body: "", button_label: "" };
  const [prompts, setPrompts] = React.useState<
    Record<"fi" | "en", PromptDraft>
  >({
    fi: emptyPrompt,
    en: emptyPrompt,
  });

  React.useEffect(() => {
    if (role && !initialized) {
      setRenewable(role.renewable);
      setRenewalPaymentLink(role.renewal_payment_link ?? "");
      setRenewalPeriodMonths(role.renewal_period_months?.toString() ?? "12");
      setRenewalEmailTemplate(role.renewal_email_template ?? "");
      setRenewalWindowDays(role.renewal_window_days.toString());
      setGracePeriodDays(role.grace_period_days.toString());
      setNotificationDays(role.renewal_notification_days.join(", "));
      const byLocale = Object.fromEntries(
        role.renewal_prompts.map((p) => [p.locale, p]),
      );
      setPrompts({
        fi: { ...emptyPrompt, ...byLocale["fi"] },
        en: { ...emptyPrompt, ...byLocale["en"] },
      });
      setInitialized(true);
    }
  }, [role, initialized]);

  if (isRoleLoading || isMembersLoading) {
    return <div>Loading...</div>;
  }

  if (roleError || membersError) {
    return (
      <div>
        Error: {roleError?.message ?? ""}, {membersError?.message ?? ""}
      </div>
    );
  }

  if (!role) {
    return <div>Role not found</div>;
  }

  const handleSave = () => {
    const windowDays = parseInt(renewalWindowDays, 10);
    const graceDays = parseInt(gracePeriodDays, 10);
    const reminders = notificationDays
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length > 0)
      .map((s) => parseInt(s, 10));

    if (!Number.isFinite(windowDays) || windowDays < 1) {
      toast.error("Renewal window must be a positive number of days");
      return;
    }
    if (!Number.isFinite(graceDays) || graceDays < 0) {
      toast.error("Grace period must be zero or more days");
      return;
    }
    if (reminders.some((n) => !Number.isFinite(n) || n < 1)) {
      toast.error("Reminder days must be positive numbers separated by commas");
      return;
    }

    updateRole(
      {
        roleName: role.name,
        data: {
          color: role.color,
          description: role.description,
          renewable,
          renewal_payment_link: renewalPaymentLink || null,
          renewal_period_months: renewable
            ? parseInt(renewalPeriodMonths) || null
            : null,
          renewal_email_template: renewalEmailTemplate || null,
          renewal_notification_days: reminders,
          renewal_window_days: windowDays,
          grace_period_days: graceDays,
          renewal_prompts: (["fi", "en"] as const)
            .filter((locale) =>
              [
                prompts[locale].title,
                prompts[locale].body,
                prompts[locale].button_label,
              ].every((v) => v.trim().length > 0),
            )
            .map((locale) => ({ locale, ...prompts[locale] })),
        },
      },
      {
        // Validation errors surface through the global mutation error toast,
        // which shows the server's message.
        onSuccess: () => {
          toast.success("Role updated");
          queryClient.invalidateQueries({
            queryKey: [QueryKey.ROLES, roleName],
          });
        },
      },
    );
  };

  return (
    <div className="p-8 space-y-6 max-w-4xl mx-auto">
      <Button variant="outline" onClick={() => navigate("/roles")}>
        Back to roles
      </Button>

      <Card className="p-8 space-y-6">
        <div className="space-y-2">
          <h1 className="text-3xl font-bold">{role.name}</h1>
          <div className="flex items-center gap-3">
            <RoleBadge role={role.name} />
            {role.color && (
              <span
                className="inline-block w-4 h-4 rounded-full border"
                style={{ backgroundColor: role.color }}
              />
            )}
          </div>
          {role.description && (
            <p className="text-muted-foreground">{role.description}</p>
          )}
        </div>

        <div className="space-y-4 border-t pt-6">
          <h2 className="text-xl font-semibold">Renewal Settings</h2>

          <div className="flex items-center gap-3">
            <Switch
              checked={renewable}
              onCheckedChange={setRenewable}
              id="renewable"
            />
            <Label htmlFor="renewable">
              Auto-renewable (conditional on payment)
            </Label>
          </div>

          {renewable && (
            <div className="space-y-4 pl-1">
              <div className="space-y-2">
                <Label htmlFor="payment-link">Payment link (Stripe)</Label>
                <Input
                  id="payment-link"
                  value={renewalPaymentLink}
                  onChange={(e) => setRenewalPaymentLink(e.target.value)}
                  placeholder="https://buy.stripe.com/..."
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="period-months">Renewal period (months)</Label>
                <Input
                  id="period-months"
                  type="number"
                  value={renewalPeriodMonths}
                  onChange={(e) => setRenewalPeriodMonths(e.target.value)}
                  className="w-32"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="email-template">Renewal email template</Label>
                <Select
                  value={renewalEmailTemplate}
                  onValueChange={setRenewalEmailTemplate}
                >
                  <SelectTrigger className="w-64">
                    <SelectValue placeholder="Select template" />
                  </SelectTrigger>
                  <SelectContent>
                    {emailTemplates?.map((t) => (
                      <SelectItem key={t.name} value={t.name}>
                        {t.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <p className="text-xs text-muted-foreground">
                  Available placeholders: {"{name}"}, {"{role_name}"},{" "}
                  {"{payment_link}"}, {"{valid_until}"}
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="renewal-window-days">
                  Renewal window (days before expiry)
                </Label>
                <Input
                  id="renewal-window-days"
                  data-testid="renewal-window-days"
                  type="number"
                  min={1}
                  value={renewalWindowDays}
                  onChange={(e) => setRenewalWindowDays(e.target.value)}
                  className="w-32"
                />
                <p className="text-xs text-muted-foreground">
                  Members see the renewal prompt and can pay starting this many
                  days before their membership expires.
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="grace-period-days">Grace period (days)</Label>
                <Input
                  id="grace-period-days"
                  data-testid="grace-period-days"
                  type="number"
                  min={0}
                  value={gracePeriodDays}
                  onChange={(e) => setGracePeriodDays(e.target.value)}
                  className="w-32"
                />
                <p className="text-xs text-muted-foreground">
                  How long after expiry renewal stays open.
                </p>
              </div>

              <div className="space-y-2">
                <Label htmlFor="notification-days">
                  Reminder emails (days before expiry)
                </Label>
                <Input
                  id="notification-days"
                  data-testid="notification-days"
                  value={notificationDays}
                  onChange={(e) => setNotificationDays(e.target.value)}
                  placeholder="30, 7, 1"
                  className="w-64"
                />
                <p className="text-xs text-muted-foreground">
                  Comma-separated day offsets, e.g. 30, 7, 1.
                </p>
              </div>

              <div className="space-y-2">
                <Label>Renewal banner texts</Label>
                <p className="text-xs text-muted-foreground">
                  Shown on the member home page while renewal is open. Fill all
                  three fields for a language, or leave them empty to use the
                  default texts. Placeholders: {"{year}"}, {"{valid_until}"},{" "}
                  {"{deadline}"}, {"{role_name}"}.
                </p>
                <div className="grid grid-cols-2 gap-4">
                  {(["fi", "en"] as const).map((locale) => (
                    <div key={locale} className="space-y-2">
                      <p className="text-sm font-medium">
                        {locale === "fi" ? "Suomi" : "English"}
                      </p>
                      <Input
                        data-testid={`prompt-${locale}-title`}
                        placeholder="Title"
                        value={prompts[locale].title}
                        onChange={(e) =>
                          setPrompts((prev) => ({
                            ...prev,
                            [locale]: {
                              ...prev[locale],
                              title: e.target.value,
                            },
                          }))
                        }
                      />
                      <Input
                        data-testid={`prompt-${locale}-body`}
                        placeholder="Body text"
                        value={prompts[locale].body}
                        onChange={(e) =>
                          setPrompts((prev) => ({
                            ...prev,
                            [locale]: { ...prev[locale], body: e.target.value },
                          }))
                        }
                      />
                      <Input
                        data-testid={`prompt-${locale}-button`}
                        placeholder="Button label"
                        value={prompts[locale].button_label}
                        onChange={(e) =>
                          setPrompts((prev) => ({
                            ...prev,
                            [locale]: {
                              ...prev[locale],
                              button_label: e.target.value,
                            },
                          }))
                        }
                      />
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>

        <div className="border-t pt-6">
          <Button onClick={handleSave} disabled={isPending}>
            {isPending ? "Saving..." : "Save"}
          </Button>
        </div>

        <div className="space-y-3 border-t pt-6">
          <h2 className="text-xl font-semibold">
            Members ({members?.length ?? 0})
          </h2>
          {members?.length ? (
            <div className="rounded-md border">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="text-left p-3 font-medium">Name</th>
                    <th className="text-left p-3 font-medium">Email</th>
                    <th className="text-left p-3 font-medium">Municipality</th>
                  </tr>
                </thead>
                <tbody>
                  {members.map((member) => (
                    <tr key={member.user_id} className="border-b last:border-0">
                      <td className="p-3">
                        <Link
                          to={`/members/${member.user_id}`}
                          className="text-blue-600 hover:underline"
                        >
                          {member.full_name ?? "N/A"}
                        </Link>
                      </td>
                      <td className="p-3">{member.email ?? "N/A"}</td>
                      <td className="p-3">
                        {member.home_municipality ?? "N/A"}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p className="text-muted-foreground">No members</p>
          )}
        </div>
      </Card>
    </div>
  );
};

export default Role;
