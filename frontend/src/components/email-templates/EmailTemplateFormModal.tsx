import {
  QueryKey,
  useCreateEmailTemplate,
  useUpdateEmailTemplate,
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

interface Props {
  template?: EmailTemplate;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

const EmailTemplateFormModal = ({ template, open, onOpenChange }: Props) => {
  const isEdit = !!template;
  const [internalOpen, setInternalOpen] = useState(false);
  const [name, setName] = useState(template?.name ?? "");
  const [subject, setSubject] = useState(template?.subject ?? "");
  const [bodyHtml, setBodyHtml] = useState(template?.body_html ?? "");
  const { mutate: createTemplate } = useCreateEmailTemplate();
  const { mutate: updateTemplate } = useUpdateEmailTemplate();
  const queryClient = useQueryClient();

  const dialogOpen = isEdit ? open : internalOpen;
  const setDialogOpen = isEdit
    ? onOpenChange ?? (() => {})
    : setInternalOpen;

  useEffect(() => {
    if (template) {
      setName(template.name);
      setSubject(template.subject);
      setBodyHtml(template.body_html);
    }
  }, [template]);

  const handleSubmit = () => {
    if (!subject || !bodyHtml) return;

    const onSuccess = () => {
      queryClient.invalidateQueries({ queryKey: [QueryKey.EMAIL_TEMPLATES] });
      setDialogOpen(false);
      if (!isEdit) {
        setName("");
        setSubject("");
        setBodyHtml("");
      }
    };

    if (isEdit) {
      updateTemplate(
        { name: template.name, subject, body_html: bodyHtml },
        { onSuccess },
      );
    } else {
      if (!name) return;
      createTemplate({ name, subject, body_html: bodyHtml }, { onSuccess });
    }
  };

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
            {isEdit ? `Edit Template: ${template.name}` : "Create Email Template"}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>
        <div className="space-y-4">
          {!isEdit && (
            <div className="space-y-2">
              <Label htmlFor="name">Name</Label>
              <Input
                id="name"
                placeholder="e.g. application_approved"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>
          )}
          <div className="space-y-2">
            <Label htmlFor="subject">Subject</Label>
            <Input
              id="subject"
              placeholder="e.g. Your application for {role_name} has been approved"
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="body_html">Body (HTML)</Label>
            <Textarea
              id="body_html"
              placeholder="HTML body with {name} and {role_name} placeholders"
              value={bodyHtml}
              onChange={(e) => setBodyHtml(e.target.value)}
              rows={8}
            />
          </div>
          <p className="text-sm text-muted-foreground">
            Available placeholders: {"{name}"}, {"{role_name}"}
          </p>
          <Button onClick={handleSubmit} className="w-full">
            {isEdit ? "Save" : "Create"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default EmailTemplateFormModal;
