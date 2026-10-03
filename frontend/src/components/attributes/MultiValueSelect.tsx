import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "../ui/input";
import MultipleSelector, { Option } from "../ui/multiple-selector";

// Attribute values are shown verbatim (unlike `stringsToOptions`, which
// capitalizes labels).
const toOptions = (values: string[]): Option[] =>
  values.map((v) => ({ value: v, label: v }));

interface Props {
  id?: string;
  values: string[];
  /** Restricts choices; when absent, any typed value can be added. */
  allowedValues: string[] | null;
  /**
   * With `allowedValues`, also offers a free-text input for one value
   * outside the list.
   */
  allowOther?: boolean;
  onChange: (values: string[]) => void;
  disabled?: boolean;
  "data-testid"?: string;
}

/** Input for a `multiple` attribute: pick (or type) any number of values. */
const MultiValueSelect = ({
  id,
  values,
  allowedValues,
  allowOther = false,
  onChange,
  disabled,
  "data-testid": testId,
}: Props) => {
  const { t } = useTranslation();
  const creatable = !allowedValues || allowedValues.length === 0;
  const withOther = allowOther && !creatable;
  // The "other" value is the (at most one) value outside the list.
  const listed = creatable
    ? values
    : values.filter((v) => allowedValues.includes(v));
  const other = withOther
    ? (values.find((v) => !allowedValues.includes(v)) ?? "")
    : "";
  const [otherDraft, setOtherDraft] = useState(other);

  const withOtherValue = (vs: string[], o: string) => (o ? [...vs, o] : vs);

  const commitOther = () => {
    const text = otherDraft.trim();
    if (text !== other) onChange(withOtherValue(listed, text));
  };

  return (
    <div data-testid={testId} className="space-y-2">
      <MultipleSelector
        value={toOptions(listed)}
        options={creatable ? undefined : toOptions(allowedValues ?? [])}
        onChange={(opts) =>
          onChange(
            withOtherValue(
              opts.map((o) => o.value),
              other,
            ),
          )
        }
        creatable={creatable}
        disabled={disabled}
        placeholder={
          creatable
            ? t("attributes.multi_add_placeholder")
            : t("attributes.not_set_placeholder")
        }
        hidePlaceholderWhenSelected
        inputProps={{ id }}
      />
      {withOther && (
        <Input
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

export default MultiValueSelect;
