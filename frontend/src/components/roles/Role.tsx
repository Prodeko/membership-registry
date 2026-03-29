import {
  QueryKey,
  useGetEmailTemplates,
  useGetRole,
  useGetRoleMembers,
  useUpdateRole,
} from "@/lib/api";
import { useNavigate, useParams, Link } from "react-router-dom";
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
  const [initialized, setInitialized] = React.useState(false);

  React.useEffect(() => {
    if (role && !initialized) {
      setRenewable(role.renewable);
      setRenewalPaymentLink(role.renewal_payment_link ?? "");
      setRenewalPeriodMonths(role.renewal_period_months?.toString() ?? "12");
      setRenewalEmailTemplate(role.renewal_email_template ?? "");
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
          renewal_notification_days: role.renewal_notification_days,
        },
      },
      {
        onSuccess: () => {
          toast.success("Role updated");
          queryClient.invalidateQueries({
            queryKey: [QueryKey.ROLES, roleName],
          });
        },
        onError: () => {
          toast.error("Failed to update role");
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

              <div className="text-sm text-muted-foreground">
                Notifications sent at:{" "}
                {role.renewal_notification_days.join(", ")} days before expiry
              </div>
            </div>
          )}

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
