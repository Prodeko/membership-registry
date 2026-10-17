import { X as XIcon } from "lucide-react";
import { useGetAttributeDefinitions } from "@/lib/api";
import { MemberAttributeFilter } from "@/common/types";
import MultipleSelector, { Option } from "../ui/multiple-selector";
import { Button } from "../ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";

// Values match exactly, so labels show them as stored, without the
// capitalization the shared option helper applies.
const toOptions = (values: string[]): Option[] =>
  values.map((v) => ({ label: v, value: v }));

interface Props {
  value: MemberAttributeFilter[];
  onChange: (filters: MemberAttributeFilter[]) => void;
}

/// One row per filtered attribute. A member must match every row; within a
/// row any selected value matches, and no values means "has any value".
export default function AttributeFilters({ value, onChange }: Props) {
  const { data: definitions } = useGetAttributeDefinitions();
  const unused = (definitions ?? []).filter(
    (d) => !value.some((f) => f.name === d.name),
  );

  const setValues = (name: string, values: string[]) =>
    onChange(value.map((f) => (f.name === name ? { ...f, values } : f)));

  return (
    <>
      {value.map((filter) => {
        const definition = definitions?.find((d) => d.name === filter.name);
        const allowed = definition?.allowed_values ?? [];
        return (
          <div key={filter.name} className="flex items-center gap-2">
            <span className="text-xs font-medium text-muted-foreground">
              {filter.name}
            </span>
            <MultipleSelector
              options={toOptions(allowed)}
              value={toOptions(filter.values)}
              onChange={(opts) =>
                setValues(
                  filter.name,
                  opts.map((o) => o.value),
                )
              }
              // Free-text attributes (and "other" answers) need typed values.
              creatable={allowed.length === 0 || !!definition?.allow_other}
              placeholder="Any value"
              className="w-48 bg-background"
            />
            <Button
              size="icon"
              variant="ghost"
              className="h-7 w-7"
              aria-label={`Remove ${filter.name} filter`}
              onClick={() =>
                onChange(value.filter((f) => f.name !== filter.name))
              }
            >
              <XIcon className="h-3.5 w-3.5" />
            </Button>
          </div>
        );
      })}
      {unused.length > 0 && (
        <Select
          value=""
          onValueChange={(name) => onChange([...value, { name, values: [] }])}
        >
          <SelectTrigger className="w-40 h-8 bg-background text-xs">
            <SelectValue placeholder="Add attribute filter" />
          </SelectTrigger>
          <SelectContent>
            {unused.map((d) => (
              <SelectItem key={d.name} value={d.name}>
                {d.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      )}
    </>
  );
}
