"use client";

import { Cross2Icon } from "@radix-ui/react-icons";
import { Table } from "@tanstack/react-table";

import { Button } from "@/components/ui/button";
import { DataTableViewOptions } from "@/components/ui/data-table-view-options";
import { Input } from "@/components/ui/input";

import { ActionElement } from "./data-table";
import { useCreateSavedFilter, useGetSavedFilters } from "@/lib/api";
import { Badge } from "./badge";

interface DataTableToolbarProps<TData> {
  table: Table<TData>;
  searchColumn?: string;
  multipleRowActionElements?: ActionElement<TData>[];
  modelName: string;
}

export function DataTableToolbar<TData>({
  table,
  searchColumn,
  multipleRowActionElements,
  modelName
}: DataTableToolbarProps<TData>) {
  const isFiltered = table.getState().columnFilters.length > 0;
  const savedFilters = useGetSavedFilters(modelName);
  const saveFilterMutation = useCreateSavedFilter();

  const saveFilter = async () => {
    const filter = {
      name: "test",
      visible_for_all: true,
      filtered_model: modelName,
      sorting_desc: false
    };
    await saveFilterMutation.mutateAsync(filter);
    savedFilters.refetch();
  };

  const parseRowsFromSelection = () => {
    return Object.entries(table.getState().rowSelection)
      .filter(([, isSelected]) => isSelected)
      .map(([id]) => id);
  };



  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="flex flex-1 items-center space-x-2">
          {searchColumn ? (
            <Input
              placeholder="Search"
              value={
                (table.getColumn(searchColumn!)?.getFilterValue() as string) ??
                ""
              }
              onChange={(event) =>
                table
                  .getColumn(searchColumn!)
                  ?.setFilterValue(event.target.value)
              }
              className="mr-2"
            />
          ) : null}
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
        <DataTableViewOptions table={table} />
        <Button onClick={saveFilter}>Save filter</Button>
      </div>
      <div>
          {savedFilters.data?.map((filter) => (
            <Badge key={filter.name} color="blue">
              {filter.name}
            </Badge>
          ))}
        </div>
      <div>

      {multipleRowActionElements &&
        (table.getSelectedRowModel().rows.length ? (
          <div className="space-x-2 flex">
            {multipleRowActionElements &&
              multipleRowActionElements.map((element) =>
                element(table, parseRowsFromSelection())
            )}
          </div>
        ) : (
          <div className="h-10"></div>
        ))}
        </div>
    </div>
  );
}
