import {
  QueryKey,
  useCreateEmailTemplate,
  useGetEmailTemplateTranslations,
  useUpsertEmailTemplateTranslation,
  useDeleteEmailTemplateTranslation,
  useGetRoles,
  useGetTargetableRoles,
} from "@/lib/api";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import { Button } from "../ui/button";
import { useState, useEffect } from "react";
import { Input } from "../ui/input";
import { Textarea } from "../ui/textarea";
import { Label } from "../ui/label";
import { useQueryClient } from "@tanstack/react-query";
import { EmailTemplate } from "@/common/types";
import { Badge } from "../ui/badge";
import { Trash2 } from "lucide-react";

const LOCALES = ["fi", "en"] as const;

// Filled only when the template is sent as a renewal reminder; anywhere
// else they reach the recipient as literal text.
const RENEWAL_ONLY_PLACEHOLDERS = ["payment_link", "valid_until", "expires_in"];

interface Props {
  template?: EmailTemplate;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

const EmailTemplateFormModal = ({ template, open, onOpenChange }: Props) => {
  const isEdit = !!template;
  const [internalOpen, setInternalOpen] = useState(false);
  const [name, setName] = useState("");
  const [activeLocale, setActiveLocale] = useState<string>("fi");
  const [translations, setTranslations] = useState<
    Record<string, { subject: string; body_html: string }>
  >({});

  const { mutate: createTemplate } = useCreateEmailTemplate();
  const { mutate: upsertTranslation } = useUpsertEmailTemplateTranslation();
  const { mutate: deleteTranslation } = useDeleteEmailTemplateTranslation();
  const { data: existingTranslations } = useGetEmailTemplateTranslations(
    template?.name ?? "",
  );
  const queryClient = useQueryClient();
  const { data: roles } = useGetRoles();
  const { data: targetableRoles } = useGetTargetableRoles();

  // Where this template is used, to warn about renewal-only placeholders in
  // emails that never fill them.
  const templateName = template?.name ?? name;
  const renewalRoles = (roles ?? [])
    .filter((r) => r.renewal_email_template === templateName)
    .map((r) => r.name);
  const applicationUses = [
    ...new Set(
      (targetableRoles ?? []).flatMap((r) => [
        ...(r.approved_email_template === templateName
          ? [`${r.role_name} approval`]
          : []),
        ...(r.rejected_email_template === templateName
          ? [`${r.role_name} rejection`]
          : []),
      ]),
    ),
  ];
  const renewalOnlyUsed = RENEWAL_ONLY_PLACEHOLDERS.filter((p) =>
    Object.values(translations).some(
      (t) => t.subject.includes(`{${p}}`) || t.body_html.includes(`{${p}}`),
    ),
  );
  const placeholderWarning =
    renewalOnlyUsed.length === 0
      ? null
      : applicationUses.length > 0
        ? `This template is also used for application emails (${applicationUses.join(", ")}), where ${renewalOnlyUsed.map((p) => `{${p}}`).join(", ")} ${renewalOnlyUsed.length === 1 ? "is" : "are"} not filled in: recipients would see the placeholder as plain text.`
        : renewalRoles.length === 0
          ? `${renewalOnlyUsed.map((p) => `{${p}}`).join(", ")} ${renewalOnlyUsed.length === 1 ? "is" : "are"} only filled in renewal reminders, and this template isn't any role's renewal email. Wherever else it's used, recipients would see the placeholder as plain text.`
          : null;

  const dialogOpen = isEdit ? open : internalOpen;
  const setDialogOpen = isEdit ? (onOpenChange ?? (() => {})) : setInternalOpen;

  useEffect(() => {
    if (existingTranslations) {
      const map: Record<string, { subject: string; body_html: string }> = {};
      for (const t of existingTranslations) {
        map[t.locale] = { subject: t.subject, body_html: t.body_html };
      }
      setTranslations(map);
    }
  }, [existingTranslations]);

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: [QueryKey.EMAIL_TEMPLATES] });
    if (template) {
      queryClient.invalidateQueries({
        queryKey: [QueryKey.EMAIL_TEMPLATES, template.name, "translations"],
      });
    }
  };

  const handleCreate = () => {
    if (!name) return;
    createTemplate(
      { name },
      {
        onSuccess: () => {
          invalidate();
          setDialogOpen(false);
          setName("");
        },
      },
    );
  };

  const handleSaveTranslation = (locale: string) => {
    if (!template) return;
    const t = translations[locale];
    if (!t?.subject || !t?.body_html) return;

    upsertTranslation(
      {
        name: template.name,
        locale,
        subject: t.subject,
        body_html: t.body_html,
      },
      { onSuccess: invalidate },
    );
  };

  const handleDeleteTranslation = (locale: string) => {
    if (!template) return;
    deleteTranslation(
      { name: template.name, locale },
      {
        onSuccess: () => {
          setTranslations((prev) => {
            const next = { ...prev };
            delete next[locale];
            return next;
          });
          invalidate();
        },
      },
    );
  };

  const updateField = (
    locale: string,
    field: "subject" | "body_html",
    value: string,
  ) => {
    setTranslations((prev) => ({
      ...prev,
      // A translation typed into for the first time has only the edited
      // field; start from empty strings so the other one is never undefined.
      [locale]: {
        ...(prev[locale] ?? { subject: "", body_html: "" }),
        [field]: value,
      },
    }));
  };

  const current = translations[activeLocale] ?? { subject: "", body_html: "" };

  return (
    <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
      {!isEdit && (
        <DialogTrigger asChild>
          <Button>Create Template</Button>
        </DialogTrigger>
      )}
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {isEdit
              ? `Edit Template: ${template.name}`
              : "Create Email Template"}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>

        {!isEdit ? (
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                placeholder="e.g. application_approved"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>
            <Button onClick={handleCreate} className="w-full">
              Create
            </Button>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="flex gap-2">
              {LOCALES.map((locale) => (
                <Button
                  key={locale}
                  variant={activeLocale === locale ? "default" : "outline"}
                  size="sm"
                  onClick={() => setActiveLocale(locale)}
                >
                  {locale.toUpperCase()}
                  {translations[locale] && (
                    <Badge variant="secondary" className="ml-1 text-xs">
                      ✓
                    </Badge>
                  )}
                </Button>
              ))}
            </div>

            <div className="space-y-2">
              <Label htmlFor="subject">Subject</Label>
              <Input
                id="subject"
                placeholder="e.g. Your application for {role_name} has been approved"
                value={current.subject}
                onChange={(e) =>
                  updateField(activeLocale, "subject", e.target.value)
                }
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="body_html">Body (HTML)</Label>
              <Textarea
                id="body_html"
                placeholder="HTML body with {name} and {role_name} placeholders"
                value={current.body_html}
                onChange={(e) =>
                  updateField(activeLocale, "body_html", e.target.value)
                }
                rows={8}
              />
            </div>
            <div className="text-sm text-muted-foreground space-y-1">
              <p>
                Available placeholders: {"{name}"}, {"{role_name}"}
              </p>
              <p>
                In renewal reminder emails also: {"{payment_link}"},{" "}
                {"{valid_until}"} (current membership end date),{" "}
                {"{expires_in}"} (days until it ends)
              </p>
              {(renewalRoles.length > 0 || applicationUses.length > 0) && (
                <p>
                  Used as:{" "}
                  {[
                    ...renewalRoles.map((r) => `${r} renewal reminder`),
                    ...applicationUses,
                  ].join(", ")}
                </p>
              )}
            </div>
            {placeholderWarning && (
              <p
                role="alert"
                className="rounded-md border border-destructive bg-destructive/10 px-3 py-2 text-sm text-destructive"
              >
                {placeholderWarning}
              </p>
            )}
            <div className="flex gap-2">
              <Button
                onClick={() => handleSaveTranslation(activeLocale)}
                className="flex-1"
              >
                Save {activeLocale.toUpperCase()}
              </Button>
              {translations[activeLocale] && (
                <Button
                  variant="destructive"
                  size="icon"
                  onClick={() => handleDeleteTranslation(activeLocale)}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              )}
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
};

export default EmailTemplateFormModal;
