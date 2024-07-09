"use client";

import { Cross2Icon } from "@radix-ui/react-icons";
import { Table } from "@tanstack/react-table";

import { Button } from "@/components/ui/button";
import { DataTableViewOptions } from "@/components/ui/data-table-view-options";
import { Input } from "@/components/ui/input";

import { ActionElement } from "./data-table";

interface DataTableToolbarProps<TData> {
  table: Table<TData>;
  multipleRowActionElements?: ActionElement<TData>[];
}

export function DataTableToolbar<TData>({
  table,
  multipleRowActionElements,
  customFilters
}: DataTableToolbarProps<TData>) {
  const isFiltered = table.getState().columnFilters.length > 0;

  const parseRowsFromSelection = () => {
    return Object.entries(table.getState().rowSelection)
      .filter(([, isSelected]) => isSelected)
      .map(([id]) => id);
  };

  return (
    <div className="flex items-center justify-between">
      <div className="flex flex-1 items-center space-x-2">
        <Input
          placeholder="Search"
          value={
            (table.getColumn("first_name")?.getFilterValue() as string) ?? ""
          }
          onChange={(event) =>
            table.getColumn("first_name")?.setFilterValue(event.target.value)
          }
          className="mr-2"
        />
        {isFiltered && (
          <Button
            variant="ghost"
            onClick={() => table.resetColumnFilters()}
            className="h-8 px-2 lg:px-3"
          >
            Reset
            <Cross2Icon className="ml-2 h-4 w-4" />
          </Button>
        )}
      </div>
      <div className="space-x-2 flex">
        {multipleRowActionElements &&
          multipleRowActionElements.map((element) =>
            element(table, parseRowsFromSelection())
          )}
        
        <DataTableViewOptions table={table} />
      </div>
    </div>
  );
}
