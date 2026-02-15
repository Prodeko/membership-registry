/* eslint-disable @typescript-eslint/no-explicit-any */
import { Check, ChevronsUpDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandInput,
  CommandItem,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { REGIONS } from "@/lib/constants";
import { cn } from "@/lib/utils";
import { CommandList } from "cmdk";
import { FormControl } from "../ui/form";

const MunicipalitySelect = ({ field, form }: { field: any; form: any }) => {
  return (
    <Popover>
      <PopoverTrigger asChild>
        <FormControl>
          <Button
            variant="outline"
            role="combobox"
            className={cn(
              "w-[200px] justify-between",
              !field && "text-muted-foreground",
            )}
          >
            {field?.value?.toString() || "Select region"}
            <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
          </Button>
        </FormControl>
      </PopoverTrigger>
      <PopoverContent className="w-[200px] p-0">
        <Command>
          <CommandInput placeholder="Search region..." />
          <CommandEmpty>No region found.</CommandEmpty>
          <CommandList>
            {REGIONS.map((region) => (
              <CommandItem
                asChild={false}
                value={region}
                key={region}
                onSelect={() => {
                  form.setValue("home_municipality", region);
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
            ))}
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  );
};

export default MunicipalitySelect;
