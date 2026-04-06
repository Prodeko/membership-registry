import { useCreateMarketingTag, useUpdateMarketingTag } from "@/lib/api";
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
import { Switch } from "../ui/switch";
import { MarketingTag } from "@/common/types";

interface Props {
  tag?: MarketingTag;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}

interface FormState {
  label: string;
  name_en: string;
  name_fi: string;
  desc_en: string;
  desc_fi: string;
  display_order: number;
  auto_apply: boolean;
}

const emptyForm: FormState = {
  label: "",
  name_en: "",
  name_fi: "",
  desc_en: "",
  desc_fi: "",
  display_order: 0,
  auto_apply: false,
};

const MarketingTagFormModal = ({ tag, open, onOpenChange }: Props) => {
  const isEdit = !!tag;
  const [internalOpen, setInternalOpen] = useState(false);
  const [form, setForm] = useState<FormState>(emptyForm);

  const { mutate: createTag, isPending: creating } = useCreateMarketingTag();
  const { mutate: updateTag, isPending: updating } = useUpdateMarketingTag();

  const dialogOpen = isEdit ? open : internalOpen;
  const setDialogOpen = isEdit ? (onOpenChange ?? (() => {})) : setInternalOpen;

  useEffect(() => {
    if (tag) {
      setForm({
        label: tag.label,
        name_en: tag.name_en,
        name_fi: tag.name_fi,
        desc_en: tag.desc_en,
        desc_fi: tag.desc_fi,
        display_order: tag.display_order,
        auto_apply: tag.auto_apply,
      });
    } else if (!dialogOpen) {
      setForm(emptyForm);
    }
  }, [tag, dialogOpen]);

  const handleSubmit = () => {
    if (!form.label || !form.name_en || !form.name_fi) return;
    if (isEdit && tag) {
      updateTag(
        {
          label: tag.label,
          data: {
            name_en: form.name_en,
            name_fi: form.name_fi,
            desc_en: form.desc_en,
            desc_fi: form.desc_fi,
            display_order: form.display_order,
            auto_apply: form.auto_apply,
          },
        },
        { onSuccess: () => setDialogOpen(false) },
      );
    } else {
      createTag(form, {
        onSuccess: () => {
          setDialogOpen(false);
          setForm(emptyForm);
        },
      });
    }
  };

  const updateField = <K extends keyof FormState>(
    key: K,
    value: FormState[K],
  ) => setForm((prev) => ({ ...prev, [key]: value }));

  return (
    <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
      {!isEdit && (
        <DialogTrigger asChild>
          <Button>Create tag</Button>
        </DialogTrigger>
      )}
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>
            {isEdit ? `Edit tag: ${tag.label}` : "Create marketing tag"}
          </DialogTitle>
          <DialogClose />
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="label">Label (Mailchimp key)</Label>
            <Input
              id="label"
              placeholder="e.g. weekly_newsletter"
              value={form.label}
              onChange={(e) => updateField("label", e.target.value)}
              disabled={isEdit}
            />
            {isEdit && (
              <p className="text-xs text-muted-foreground">
                The label is immutable — delete and recreate the row if you need
                to rename a tag.
              </p>
            )}
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="name_en">Name (EN)</Label>
              <Input
                id="name_en"
                value={form.name_en}
                onChange={(e) => updateField("name_en", e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="name_fi">Name (FI)</Label>
              <Input
                id="name_fi"
                value={form.name_fi}
                onChange={(e) => updateField("name_fi", e.target.value)}
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="desc_en">Description (EN)</Label>
              <Textarea
                id="desc_en"
                rows={3}
                value={form.desc_en}
                onChange={(e) => updateField("desc_en", e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="desc_fi">Description (FI)</Label>
              <Textarea
                id="desc_fi"
                rows={3}
                value={form.desc_fi}
                onChange={(e) => updateField("desc_fi", e.target.value)}
              />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="display_order">Display order</Label>
            <Input
              id="display_order"
              type="number"
              value={form.display_order}
              onChange={(e) =>
                updateField("display_order", Number(e.target.value))
              }
            />
            <p className="text-xs text-muted-foreground">
              Lower numbers appear first in the user UI.
            </p>
          </div>

          <div className="flex items-center justify-between">
            <div className="space-y-1">
              <Label htmlFor="auto_apply">Auto-apply</Label>
              <p className="text-xs text-muted-foreground">
                Activated automatically for new users on registration and on the
                "resubscribe" button. Does not backfill existing subscribers.
              </p>
            </div>
            <Switch
              id="auto_apply"
              checked={form.auto_apply}
              onCheckedChange={(v) => updateField("auto_apply", v)}
            />
          </div>

          <Button
            onClick={handleSubmit}
            className="w-full"
            disabled={creating || updating}
          >
            {isEdit ? "Save" : "Create"}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default MarketingTagFormModal;
