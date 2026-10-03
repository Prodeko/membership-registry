import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "../ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";

const CLEAR_VALUE = "__attribute_clear__";
const OTHER_VALUE = "__attribute_other__";

interface Props {
  id: string;
  /** The committed value; empty when not set. */
  value: string;
  allowedValues: string[];
  /** Offers an "Other…" choice that reveals a free-text input. */
  allowOther: boolean;
  /**
   * Called with the new committed value; an empty string clears. Free text
   * is committed on blur, not per keystroke.
   */
  onChange: (value: string) => void;
  /** When set, a "not set" item is offered that commits an empty value. */
  clearLabel?: string;
  placeholder: string;
  contentClassName?: string;
  disabled?: boolean;
  "data-testid"?: string;
}

/** Input for a single-valued attribute with `allowed_values`. */
const SingleValueSelect = ({
  id,
  value,
  allowedValues,
  allowOther,
  onChange,
  clearLabel,
  placeholder,
  contentClassName,
  disabled,
  "data-testid": testId,
}: Props) => {
  const { t } = useTranslation();
  const isOther = value !== "" && !allowedValues.includes(value);
  const [otherMode, setOtherMode] = useState(isOther);
  const [otherDraft, setOtherDraft] = useState(isOther ? value : "");

  const handleSelect = (v: string) => {
    if (v === OTHER_VALUE) {
      setOtherMode(true);
      // Re-selecting "Other…" restores the text typed before.
      if (otherDraft.trim()) onChange(otherDraft.trim());
      return;
    }
    setOtherMode(false);
    onChange(v === CLEAR_VALUE ? "" : v);
  };

  const commitOther = () => {
    const text = otherDraft.trim();
    if (text && text !== value) onChange(text);
  };

  const selected = otherMode
    ? OTHER_VALUE
    : value || (clearLabel !== undefined ? CLEAR_VALUE : "");

  return (
    <div className="flex flex-wrap gap-2">
      <Select value={selected} onValueChange={handleSelect} disabled={disabled}>
        <SelectTrigger id={id} className="w-64" data-testid={testId}>
          <SelectValue placeholder={placeholder} />
        </SelectTrigger>
        <SelectContent className={contentClassName}>
          {clearLabel !== undefined && (
            <SelectItem value={CLEAR_VALUE}>{clearLabel}</SelectItem>
          )}
          {allowedValues.map((v) => (
            <SelectItem key={v} value={v}>
              {v}
            </SelectItem>
          ))}
          {allowOther && (
            <SelectItem value={OTHER_VALUE}>
              {t("attributes.other_option")}
            </SelectItem>
          )}
        </SelectContent>
      </Select>
      {otherMode && (
        <Input
          className="w-64"
          aria-label={t("attributes.other_option")}
          value={otherDraft}
          onChange={(e) => setOtherDraft(e.target.value)}
          onBlur={commitOther}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              commitOther();
            }
          }}
          placeholder={t("attributes.other_placeholder")}
          disabled={disabled}
          data-testid={testId && `${testId}-other`}
        />
      )}
    </div>
  );
};

export default SingleValueSelect;
