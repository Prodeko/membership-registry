import {
  QueryKey,
  useCreateTargetableRole,
  useGetAttributeDefinitions,
  useGetEmailTemplates,
  useGetRoles,
} from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { DatePicker } from "../../ui/date-picker";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../../ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../../ui/select";
import { Button } from "../../ui/button";
import { Link } from "react-router";
import { stringsToOptions } from "@/lib/utils";
import MultipleSelector, { Option } from "@/components/ui/multiple-selector";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

const CreateTargetableRolesModal = () => {
  const [selectedRole, setSelectedRole] = useState<string>("");
  const [selectedValidUntil, setSelectedValidUntil] = useState<
    Date | undefined
  >(undefined);
  const [selectedPaymentLink, setSelectedPaymentLink] = useState<string>("");
  const [approvedTemplate, setApprovedTemplate] = useState<string>("");
  const [rejectedTemplate, setRejectedTemplate] = useState<string>("");
  const [, setSelectedRoles] = useState<string[]>([]);
  const [formAttributes, setFormAttributes] = useState<string[]>([]);
  const { data: roles } = useGetRoles();
  const { data: emailTemplates } = useGetEmailTemplates();
  const { data: attributeDefs } = useGetAttributeDefinitions();

  const { mutate: createTargetableRole } = useCreateTargetableRole();

  const queryClient = useQueryClient();

  const handleSubmit = () => {
    if (selectedRole && selectedValidUntil) {
      // TODO: Add validation for payment link
      createTargetableRole(
        {
          role_name: selectedRole,
          valid_until: selectedValidUntil.toISOString().split("T")[0],
          payment_link: selectedPaymentLink || null,
          approved_email_template: approvedTemplate || null,
          rejected_email_template: rejectedTemplate || null,
          form_attributes: formAttributes,
        },
        {
          onSuccess: () => {
            queryClient.invalidateQueries({
              queryKey: [QueryKey.TARGETABLE_ROLES],
            });
          },
        },
      );
    }
  };

  const onRoleChange = (selectedRoles: Option[]) => {
    setSelectedRoles(selectedRoles.map((r) => r.value));
  };

  const onFormAttributesChange = (selected: Option[]) => {
    // MultipleSelector preserves user-selection order — that's the order
    // applicants will see fields rendered in, so persist it as-is.
    setFormAttributes(selected.map((s) => s.value));
  };

  return (
    <Dialog>
      <DialogTrigger asChild>
        <Button>Create Application Targetable Role</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create Application Targetable Role</DialogTitle>
          <DialogClose />
        </DialogHeader>
        <DialogDescription>
          Add a new role that can be targeted by applications. Add more roles{" "}
          <Link to={"/roles"}>here</Link>
        </DialogDescription>
        <div className="flex justify-between">
          <Select onValueChange={setSelectedRole}>
            <SelectTrigger>
              <SelectValue placeholder="Select role" />
            </SelectTrigger>
            <SelectContent>
              {roles?.map((role) => (
                <SelectItem key={role.name} value={role.name}>
                  {role.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <DatePicker
            onSelect={(day: Date | undefined) => setSelectedValidUntil(day)}
            title="Valid until"
          />
        </div>
        <Input
          type="text"
          placeholder="Payment link"
          value={selectedPaymentLink}
          onChange={(e) => setSelectedPaymentLink(e.target.value)}
        />
        <Select
          onValueChange={(v) => setApprovedTemplate(v === "__none__" ? "" : v)}
        >
          <SelectTrigger>
            <SelectValue placeholder="Approved email template (optional)" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__none__">None</SelectItem>
            {emailTemplates?.map((t) => (
              <SelectItem key={t.name} value={t.name}>
                {t.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select
          onValueChange={(v) => setRejectedTemplate(v === "__none__" ? "" : v)}
        >
          <SelectTrigger>
            <SelectValue placeholder="Rejected email template (optional)" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__none__">None</SelectItem>
            {emailTemplates?.map((t) => (
              <SelectItem key={t.name} value={t.name}>
                {t.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <MultipleSelector
          options={stringsToOptions(roles?.map((r) => r.name) ?? [])}
          onChange={onRoleChange}
          placeholder="Optional roles"
        />
        <div className="space-y-1">
          <Label>Application form attributes</Label>
          <MultipleSelector
            options={stringsToOptions(attributeDefs?.map((d) => d.name) ?? [])}
            onChange={onFormAttributesChange}
            placeholder="Attributes shown on the application form"
          />
          <p className="text-xs text-muted-foreground">
            Order is preserved as the applicant&apos;s field order.
          </p>
        </div>
        <DialogClose asChild>
          <Button onClick={handleSubmit}>Create</Button>
        </DialogClose>
      </DialogContent>
    </Dialog>
  );
};

export default CreateTargetableRolesModal;
