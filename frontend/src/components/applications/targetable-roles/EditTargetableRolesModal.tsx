import { ApplicationTargetableRole, PutTargetableRole } from "@/common/types";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import MultipleSelector, { Option } from "@/components/ui/multiple-selector";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  QueryKey,
  useGetAttributeDefinitions,
  useGetEmailTemplates,
  useGetRoles,
  useUpdateTargetableRole,
} from "@/lib/api";
import { describeError, stringsToOptions } from "@/lib/utils";
import { useQueryClient } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import { toast } from "sonner";

interface Props {
  role: ApplicationTargetableRole | null;
  onClose: () => void;
}

// Sentinel so the Select can encode "no template" — Radix Select forbids
// an empty-string SelectItem value.
const TEMPLATE_NONE = "__none__";

const EditTargetableRolesModal = ({ role, onClose }: Props) => {
  const [active, setActive] = useState<boolean>(role?.active ?? true);
  const [paymentLink, setPaymentLink] = useState<string>(
    role?.payment_link ?? "",
  );
  const [approvedTemplate, setApprovedTemplate] = useState<string>(
    role?.approved_email_template ?? "",
  );
  const [rejectedTemplate, setRejectedTemplate] = useState<string>(
    role?.rejected_email_template ?? "",
  );
  const [optionalRoles, setOptionalRoles] = useState<string[]>(
    role?.optional_roles ?? [],
  );
  const [formAttributes, setFormAttributes] = useState<string[]>(
    role?.form_attributes ?? [],
  );

  const { data: attributeDefs } = useGetAttributeDefinitions();
  const { data: emailTemplates } = useGetEmailTemplates();
  const { data: roles } = useGetRoles();
  const { mutate: updateTargetableRole, isPending } = useUpdateTargetableRole();
  const queryClient = useQueryClient();

  // Reset local state whenever a different row is opened. Without this the
  // dialog would surface the previous row's values for one frame.
  useEffect(() => {
    if (role) {
      setActive(role.active);
      setPaymentLink(role.payment_link ?? "");
      setApprovedTemplate(role.approved_email_template ?? "");
      setRejectedTemplate(role.rejected_email_template ?? "");
      setOptionalRoles(role.optional_roles ?? []);
      setFormAttributes(role.form_attributes);
    }
  }, [role]);

  if (!role) return null;

  const arraysEqual = (a: string[], b: string[]) =>
    a.length === b.length && a.every((v, i) => v === b[i]);

  const handleSubmit = () => {
    // Build a sparse Patch payload: omit fields that match the loaded
    // value so the wire-level Patch::Leave semantics fire and concurrent
    // admins editing different fields don't clobber each other.
    const body: PutTargetableRole = {
      role_name: role.role_name,
      valid_until: role.valid_until,
    };

    if (active !== role.active) {
      body.active = active;
    }

    const nextPaymentLink = paymentLink.trim() || null;
    if (nextPaymentLink !== (role.payment_link ?? null)) {
      body.payment_link = nextPaymentLink;
    }

    const nextApproved = approvedTemplate || null;
    if (nextApproved !== (role.approved_email_template ?? null)) {
      body.approved_email_template = nextApproved;
    }

    const nextRejected = rejectedTemplate || null;
    if (nextRejected !== (role.rejected_email_template ?? null)) {
      body.rejected_email_template = nextRejected;
    }

    const initialOptional = role.optional_roles ?? [];
    if (!arraysEqual(optionalRoles, initialOptional)) {
      body.optional_roles = optionalRoles.length > 0 ? optionalRoles : null;
    }

    if (!arraysEqual(formAttributes, role.form_attributes)) {
      body.form_attributes = formAttributes;
    }

    updateTargetableRole(body, {
      onSuccess: () => {
        toast.success(`Updated ${role.role_name}`);
        queryClient.invalidateQueries({
          queryKey: [QueryKey.TARGETABLE_ROLES],
        });
        onClose();
      },
      onError: (e) => {
        toast.error(`Failed to update: ${describeError(e)}`);
      },
    });
  };

  return (
    <Dialog open={!!role} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            Edit {role.role_name} ({role.valid_until})
          </DialogTitle>
          <DialogDescription>
            Role name and valid_until form the primary key and cannot be
            changed. Delete and recreate to rename.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <Label htmlFor="targetable-active">Active</Label>
            <Switch
              id="targetable-active"
              checked={active}
              onCheckedChange={setActive}
            />
          </div>

          <div className="space-y-1">
            <Label htmlFor="targetable-payment-link">Payment link</Label>
            <Input
              id="targetable-payment-link"
              placeholder="https://..."
              value={paymentLink}
              onChange={(e) => setPaymentLink(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Leave empty for no payment requirement.
            </p>
          </div>

          <div className="space-y-1">
            <Label>Approved email template</Label>
            <Select
              value={approvedTemplate || TEMPLATE_NONE}
              onValueChange={(v) =>
                setApprovedTemplate(v === TEMPLATE_NONE ? "" : v)
              }
            >
              <SelectTrigger>
                <SelectValue placeholder="None" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={TEMPLATE_NONE}>None</SelectItem>
                {emailTemplates?.map((t) => (
                  <SelectItem key={t.name} value={t.name}>
                    {t.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1">
            <Label>Rejected email template</Label>
            <Select
              value={rejectedTemplate || TEMPLATE_NONE}
              onValueChange={(v) =>
                setRejectedTemplate(v === TEMPLATE_NONE ? "" : v)
              }
            >
              <SelectTrigger>
                <SelectValue placeholder="None" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={TEMPLATE_NONE}>None</SelectItem>
                {emailTemplates?.map((t) => (
                  <SelectItem key={t.name} value={t.name}>
                    {t.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1">
            <Label>Optional roles</Label>
            <MultipleSelector
              value={stringsToOptions(optionalRoles)}
              options={stringsToOptions(roles?.map((r) => r.name) ?? [])}
              onChange={(selected: Option[]) =>
                setOptionalRoles(selected.map((s) => s.value))
              }
              placeholder="Optional roles granted with this membership"
            />
          </div>

          <div className="space-y-1">
            <Label>Application form attributes</Label>
            <MultipleSelector
              value={stringsToOptions(formAttributes)}
              options={stringsToOptions(
                attributeDefs?.map((d) => d.name) ?? [],
              )}
              onChange={(selected: Option[]) =>
                setFormAttributes(selected.map((s) => s.value))
              }
              placeholder="Attributes shown on the application form"
            />
            <p className="text-xs text-muted-foreground">
              Order is preserved as the applicant&apos;s field order.
            </p>
          </div>
        </div>

        <DialogFooter>
          <DialogClose asChild>
            <Button variant="outline">Cancel</Button>
          </DialogClose>
          <Button onClick={handleSubmit} disabled={isPending}>
            {isPending ? "Saving..." : "Save"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default EditTargetableRolesModal;
