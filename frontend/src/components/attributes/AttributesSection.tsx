import { useState } from "react";
import { useTranslation } from "react-i18next";
import { MemberAttribute } from "@/common/types";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import MultiValueSelect from "./MultiValueSelect";
import SingleValueSelect from "./SingleValueSelect";

interface Props {
  attributes: MemberAttribute[] | undefined;
  onSet: (input: { name: string; values: string[] }) => void;
  onDelete: (name: string) => void;
  isLoading?: boolean;
  isMutating?: boolean;
  /** Heading shown above the list. Pass falsy to hide. */
  heading?: string;
  /** When true, render an empty-state message instead of nothing if there are no defined attributes. */
  emptyMessage?: string;
  /** Offer clearing required attributes too. Only admins may clear them. */
  canClearRequired?: boolean;
}

const AttributesSection = ({
  attributes,
  onSet,
  onDelete,
  isLoading,
  isMutating,
  heading,
  emptyMessage,
  canClearRequired,
}: Props) => {
  const { t } = useTranslation();
  if (isLoading) {
    return (
      <div className="text-muted-foreground">{t("attributes.loading")}</div>
    );
  }
  if (!attributes || attributes.length === 0) {
    return (
      <div className="space-y-2">
        {heading && <h3 className="text-lg font-semibold">{heading}</h3>}
        {emptyMessage && (
          <p className="text-sm text-muted-foreground">{emptyMessage}</p>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {heading && <h3 className="text-lg font-semibold">{heading}</h3>}
      <div className="space-y-3">
        {attributes.map((attr) => (
          <AttributeRow
            key={attr.name}
            attr={attr}
            onSet={onSet}
            onDelete={onDelete}
            isMutating={!!isMutating}
            canClear={!attr.required || !!canClearRequired}
          />
        ))}
      </div>
    </div>
  );
};

interface RowProps {
  attr: MemberAttribute;
  onSet: (input: { name: string; values: string[] }) => void;
  onDelete: (name: string) => void;
  isMutating: boolean;
  canClear: boolean;
}

const AttributeRow = ({
  attr,
  onSet,
  onDelete,
  isMutating,
  canClear,
}: RowProps) => {
  const { t } = useTranslation();
  // Single-valued attributes hold at most one element.
  const current = attr.values[0] ?? "";
  const [draft, setDraft] = useState<string>(current);
  const [multiDraft, setMultiDraft] = useState<string[]>(attr.values);

  const isDirty = draft !== current;
  const hasEnum = attr.allowed_values && attr.allowed_values.length > 0;

  // Blur-triggered save only writes non-empty values. Clearing requires the
  // explicit Clear button so a typo + tab can't destructively delete the
  // existing value with no undo path.
  const saveOnBlur = () => {
    if (draft) {
      onSet({ name: attr.name, values: [draft] });
    }
  };

  const handleSelectChange = (v: string) => {
    if (!v) {
      setDraft("");
      if (current) onDelete(attr.name);
      return;
    }
    setDraft(v);
    onSet({ name: attr.name, values: [v] });
  };

  // Each add/remove saves immediately, like the single-value dropdown.
  // Removing the last value clears the attribute, unless it can't be cleared:
  // then the removal is undone (a fresh array re-syncs the selector).
  const handleMultiChange = (values: string[]) => {
    if (values.length === 0) {
      if (!canClear) {
        setMultiDraft([...multiDraft]);
        return;
      }
      setMultiDraft([]);
      if (attr.values.length > 0) onDelete(attr.name);
      return;
    }
    setMultiDraft(values);
    onSet({ name: attr.name, values });
  };

  return (
    <div className="space-y-1">
      <div className="flex items-baseline justify-between">
        <Label htmlFor={`attr-${attr.name}`} className="font-mono text-sm">
          {attr.name}
          {attr.required && (
            <span
              className="ml-0.5 text-destructive"
              title={t("attributes.required")}
            >
              *
            </span>
          )}
        </Label>
        {!attr.editable && (
          <span className="text-xs text-muted-foreground italic">
            {t("attributes.read_only")}
          </span>
        )}
      </div>
      {attr.description && (
        <p className="text-xs text-muted-foreground">{attr.description}</p>
      )}

      {!attr.editable ? (
        <p className="text-sm">
          {attr.values.length > 0 ? (
            attr.values.join(", ")
          ) : (
            <span className="text-muted-foreground">
              {t("attributes.not_set")}
            </span>
          )}
        </p>
      ) : attr.multiple ? (
        <MultiValueSelect
          id={`attr-${attr.name}`}
          values={multiDraft}
          allowedValues={attr.allowed_values}
          allowOther={attr.allow_other}
          onChange={handleMultiChange}
          disabled={isMutating}
        />
      ) : hasEnum ? (
        <SingleValueSelect
          id={`attr-${attr.name}`}
          value={current}
          allowedValues={attr.allowed_values ?? []}
          allowOther={attr.allow_other}
          onChange={handleSelectChange}
          clearLabel={
            canClear || !current ? t("attributes.not_set_option") : undefined
          }
          placeholder={t("attributes.not_set_placeholder")}
          contentClassName="z-[300]"
          disabled={isMutating}
        />
      ) : (
        <div className="flex gap-2">
          <Input
            id={`attr-${attr.name}`}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => {
              if (isDirty) saveOnBlur();
            }}
            disabled={isMutating}
            placeholder={t("attributes.not_set_placeholder")}
          />
          {current && canClear && (
            <Button
              variant="outline"
              onClick={() => {
                setDraft("");
                onDelete(attr.name);
              }}
              disabled={isMutating}
            >
              {t("attributes.clear")}
            </Button>
          )}
        </div>
      )}
    </div>
  );
};

export default AttributesSection;
