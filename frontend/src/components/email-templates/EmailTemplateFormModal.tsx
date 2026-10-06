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
import { AlertTriangle, Check, Loader2, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { describeError } from "@/lib/utils";

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
  const { mutate: upsertTranslation, isPending: saving } =
    useUpsertEmailTemplateTranslation();
  // What is stored on the server per locale, to tell saved from edited.
  const [saved, setSaved] = useState<
    Record<string, { subject: string; body_html: string }>
  >({});
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
  // "a", "a and b", "a, b and c"
  const listed = (items: string[]) =>
    items.length <= 1
      ? (items[0] ?? "")
      : `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
  const applicationEmails = (targetableRoles ?? []).flatMap((r) => [
    ...(r.approved_email_template === templateName
      ? [`the approval email of the "${r.role_name}" role`]
      : []),
    ...(r.rejected_email_template === templateName
      ? [`the rejection email of the "${r.role_name}" role`]
      : []),
  ]);
  const placeholders = listed(renewalOnlyUsed.map((p) => `{${p}}`));
  const quoted = listed(renewalOnlyUsed.map((p) => `"{${p}}"`));
  const one = renewalOnlyUsed.length === 1;
  const placeholderWarning =
    renewalOnlyUsed.length === 0
      ? null
      : applicationEmails.length > 0
        ? `${placeholders} won't be filled in here: this template is used as ${listed(applicationEmails)}, and ${applicationEmails.length === 1 ? "that email only fills" : "those emails only fill"} {name} and {role_name}. Recipients would see ${quoted} as plain text. Remove ${one ? "it" : "them"}, or use a separate template for renewal reminders.`
        : renewalRoles.length === 0
          ? `${placeholders} ${one ? "is" : "are"} only filled in renewal reminders, and this template isn't any role's renewal reminder. Wherever else it's used, recipients would see ${quoted} as plain text.`
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
      setSaved(map);
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

  const isDirty = (locale: string) => {
    const t = translations[locale];
    const s = saved[locale];
    return (
      (t?.subject ?? "") !== (s?.subject ?? "") ||
      (t?.body_html ?? "") !== (s?.body_html ?? "")
    );
  };

  const handleSaveTranslation = (locale: string) => {
    if (!template) return;
    const t = translations[locale];
    if (!t?.subject || !t?.body_html) {
      toast.error("Fill in both the subject and the body before saving.");
      return;
    }

    upsertTranslation(
      {
        name: template.name,
        locale,
        subject: t.subject,
        body_html: t.body_html,
      },
      {
        onSuccess: () => {
          setSaved((prev) => ({ ...prev, [locale]: { ...t } }));
          toast.success(`${locale.toUpperCase()} version saved`);
          invalidate();
        },
        onError: (e) =>
          toast.error(
            `Saving the ${locale.toUpperCase()} version failed: ${describeError(e)}`,
          ),
      },
    );
  };

  const handleDeleteTranslation = (locale: string) => {
    if (!template) return;
    deleteTranslation(
      { name: template.name, locale },
      {
        onSuccess: () => {
          const drop = (
            prev: Record<string, { subject: string; body_html: string }>,
          ) => {
            const next = { ...prev };
            delete next[locale];
            return next;
          };
          setTranslations(drop);
          setSaved(drop);
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
                  {isDirty(locale) ? (
                    <span
                      className="ml-1 h-2 w-2 rounded-full bg-amber-500"
                      title="Unsaved changes"
                    />
                  ) : (
                    saved[locale] && (
                      <Badge variant="secondary" className="ml-1 text-xs">
                        ✓
                      </Badge>
                    )
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
              <div
                role="alert"
                className="flex gap-2 rounded-md border border-amber-500/70 bg-amber-500/15 px-3 py-2 text-sm text-foreground"
              >
                <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-500" />
                <p>{placeholderWarning}</p>
              </div>
            )}
            <div className="flex gap-2">
              <Button
                onClick={() => handleSaveTranslation(activeLocale)}
                className="flex-1"
                variant={isDirty(activeLocale) ? "default" : "secondary"}
                disabled={saving || !isDirty(activeLocale)}
              >
                {saving ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Saving…
                  </>
                ) : isDirty(activeLocale) ? (
                  `Save ${activeLocale.toUpperCase()}`
                ) : (
                  <>
                    <Check className="mr-2 h-4 w-4" />
                    Saved
                  </>
                )}
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
