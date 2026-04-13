/* eslint-disable @typescript-eslint/no-explicit-any */
import { Check, ChevronsUpDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  COUNTRIES,
  OTHER_MUNICIPALITIES,
  PRIORITY_MUNICIPALITIES,
} from "@/lib/constants";
import { cn } from "@/lib/utils";
import { CommandList } from "cmdk";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FormControl } from "../ui/form";

const RegionItem = ({
  region,
  field,
  form,
  onSelected,
}: {
  region: string;
  field: any;
  form: any;
  onSelected: () => void;
}) => (
  <CommandItem
    asChild={false}
    value={region}
    key={region}
    onSelect={() => {
      form.setValue("home_municipality", region);
      onSelected();
    }}
  >
    <Check
      className={cn(
        "mr-2 h-4 w-4",
        region === field?.value ? "opacity-100" : "opacity-0",
      )}
    />
    {region}
  </CommandItem>
);

const MunicipalitySelect = ({ field, form }: { field: any; form: any }) => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const close = () => setOpen(false);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <FormControl>
          <Button
            variant="outline"
            role="combobox"
            data-testid="municipality-combobox-trigger"
            className={cn(
              "w-[200px] justify-between",
              !field && "text-muted-foreground",
            )}
          >
            {field?.value?.toString() || t("municipality_select.placeholder")}
            <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
          </Button>
        </FormControl>
      </PopoverTrigger>
      <PopoverContent className="w-[200px] p-0">
        <Command>
          <CommandInput placeholder={t("municipality_select.search")} data-testid="municipality-search-input" />
          <CommandEmpty>{t("municipality_select.empty")}</CommandEmpty>
          <CommandList className="max-h-[300px] overflow-y-auto">
            <CommandGroup heading="—">
              {PRIORITY_MUNICIPALITIES.map((region) => (
                <RegionItem
                  key={region}
                  region={region}
                  field={field}
                  form={form}
                  onSelected={close}
                />
              ))}
            </CommandGroup>
            <CommandGroup heading={t("municipality_select.municipalities")}>
              {OTHER_MUNICIPALITIES.map((region) => (
                <RegionItem
                  key={region}
                  region={region}
                  field={field}
                  form={form}
                  onSelected={close}
                />
              ))}
            </CommandGroup>
            <CommandGroup heading={t("municipality_select.countries")}>
              {COUNTRIES.map((region) => (
                <RegionItem
                  key={region}
                  region={region}
                  field={field}
                  form={form}
                  onSelected={close}
                />
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
};

export default MunicipalitySelect;
