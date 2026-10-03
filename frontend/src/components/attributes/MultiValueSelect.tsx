import { useTranslation } from "react-i18next";
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
  onChange: (values: string[]) => void;
  disabled?: boolean;
  "data-testid"?: string;
}

/** Input for a `multiple` attribute: pick (or type) any number of values. */
const MultiValueSelect = ({
  id,
  values,
  allowedValues,
  onChange,
  disabled,
  "data-testid": testId,
}: Props) => {
  const { t } = useTranslation();
  const creatable = !allowedValues || allowedValues.length === 0;
  return (
    <div data-testid={testId}>
      <MultipleSelector
        value={toOptions(values)}
        options={creatable ? undefined : toOptions(allowedValues ?? [])}
        onChange={(opts) => onChange(opts.map((o) => o.value))}
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
    </div>
  );
};

export default MultiValueSelect;
