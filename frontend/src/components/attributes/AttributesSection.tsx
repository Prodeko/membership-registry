import { useState } from "react";
import { toast } from "sonner";
import { MemberAttribute } from "@/common/types";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";

const CLEAR_VALUE = "__attribute_clear__";

interface Props {
  attributes: MemberAttribute[] | undefined;
  onSet: (input: { name: string; value: string }) => void;
  onDelete: (name: string) => void;
  isLoading?: boolean;
  isMutating?: boolean;
  /** Heading shown above the list. Pass falsy to hide. */
  heading?: string;
  /** When true, render an empty-state message instead of nothing if there are no defined attributes. */
  emptyMessage?: string;
}

const AttributesSection = ({
  attributes,
  onSet,
  onDelete,
  isLoading,
  isMutating,
  heading = "Attributes",
  emptyMessage = "No attributes defined.",
}: Props) => {
  if (isLoading) {
    return <div className="text-muted-foreground">Loading attributes...</div>;
  }
  if (!attributes || attributes.length === 0) {
    return (
      <div className="space-y-2">
        {heading && <h3 className="text-lg font-semibold">{heading}</h3>}
        <p className="text-sm text-muted-foreground">{emptyMessage}</p>
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
          />
        ))}
      </div>
    </div>
  );
};

interface RowProps {
  attr: MemberAttribute;
  onSet: (input: { name: string; value: string }) => void;
  onDelete: (name: string) => void;
  isMutating: boolean;
}

const AttributeRow = ({ attr, onSet, onDelete, isMutating }: RowProps) => {
  const [draft, setDraft] = useState<string>(attr.value ?? "");

  const isDirty = draft !== (attr.value ?? "");
  const hasEnum = attr.allowed_values && attr.allowed_values.length > 0;

  const save = () => {
    if (!draft) {
      if (attr.value) onDelete(attr.name);
      return;
    }
    onSet(
      { name: attr.name, value: draft },
    );
  };

  const handleSelectChange = (v: string) => {
    if (v === CLEAR_VALUE) {
      setDraft("");
      if (attr.value) onDelete(attr.name);
      return;
    }
    setDraft(v);
    onSet({ name: attr.name, value: v });
  };

  return (
    <div className="space-y-1">
      <div className="flex items-baseline justify-between">
        <Label htmlFor={`attr-${attr.name}`} className="font-mono text-sm">
          {attr.name}
        </Label>
        {!attr.editable && (
          <span className="text-xs text-muted-foreground italic">
            read-only
          </span>
        )}
      </div>
      {attr.description && (
        <p className="text-xs text-muted-foreground">{attr.description}</p>
      )}

      {!attr.editable ? (
        <p className="text-sm">
          {attr.value ?? <span className="text-muted-foreground">— not set</span>}
        </p>
      ) : hasEnum ? (
        <div className="flex gap-2">
          <Select
            value={attr.value ?? CLEAR_VALUE}
            onValueChange={handleSelectChange}
            disabled={isMutating}
          >
            <SelectTrigger id={`attr-${attr.name}`} className="w-64">
              <SelectValue placeholder="Not set" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={CLEAR_VALUE}>— not set —</SelectItem>
              {attr.allowed_values?.map((v) => (
                <SelectItem key={v} value={v}>
                  {v}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      ) : (
        <div className="flex gap-2">
          <Input
            id={`attr-${attr.name}`}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => {
              if (isDirty) save();
            }}
            disabled={isMutating}
            placeholder="Not set"
          />
          {attr.value && (
            <Button
              variant="outline"
              onClick={() => {
                setDraft("");
                onDelete(attr.name);
                toast.success(`Cleared ${attr.name}`);
              }}
              disabled={isMutating}
            >
              Clear
            </Button>
          )}
        </div>
      )}
    </div>
  );
};

export default AttributesSection;
