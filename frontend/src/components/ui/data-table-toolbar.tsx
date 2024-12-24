"use client";

import { Cross2Icon } from "@radix-ui/react-icons";
import { Table } from "@tanstack/react-table";

import { Button } from "@/components/ui/button";
import { DataTableViewOptions } from "@/components/ui/data-table-view-options";
import { Input } from "@/components/ui/input";

import { ActionElement } from "./data-table";
import { useDeleteSavedFilter, useGetSavedFilters } from "@/lib/api";
import { Badge } from "./badge";
import { NewSavedFilter, SavedFilter } from "@/common/types";
import CreateSavedFilterModal from "../saved-filter/CreateSavedFilterModal";
import { useState } from "react";

interface DataTableToolbarProps<TData> {
  table: Table<TData>;
  searchColumn?: string;
  multipleRowActionElements?: ActionElement<TData>[];
  modelName: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  customFilters?: any;
  setFilter: (filter: SavedFilter) => void;
}

export function DataTableToolbar<TData>({
  table,
  searchColumn,
  multipleRowActionElements,
  modelName,
  customFilters,
  setFilter,
}: DataTableToolbarProps<TData>) {
  const [selectedFilter, setSelectedFilter] = useState<SavedFilter | null>();
  const isFiltered = table.getState().columnFilters.length > 0;
  const savedFilters = useGetSavedFilters(modelName);
  const deleteSavedFilter = useDeleteSavedFilter();

  const newSavedFilter: NewSavedFilter = {
    name: "",
    filtered_model: modelName,
    visible_for_all: false,
    search: searchColumn
      ? (table.getColumn(searchColumn)?.getFilterValue() as string)
      : undefined,
    sorting_col: table.getState().sorting[0]?.id,
    sorting_desc: table.getState().sorting[0]?.desc ?? false,
    custom_filters: customFilters,
  };

  const parseRowsFromSelection = () => {
    return Object.entries(table.getState().rowSelection)
      .filter(([, isSelected]) => isSelected)
      .map(([id]) => id);
  };

  const handleFilterSelection = (filter: SavedFilter) => {
    setFilter(filter);
    setSelectedFilter(filter);
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
        <CreateSavedFilterModal
          newSavedFilter={newSavedFilter}
          refetchSavedFilters={savedFilters.refetch}
        />
      </div>
      <div>
        {savedFilters.data?.map((filter) => {
          const isSelected = selectedFilter?.name === filter.name;
          return (
            <Badge
              key={filter.name}
              variant={isSelected ? "default" : "outline"}
              onClick={() => handleFilterSelection(filter)}
            >
              {filter.name}
              <Button
                variant="ghost"
                className="h-8 px-2 lg:px-3"
                onClick={(event) => {
                  event.stopPropagation();
                  deleteSavedFilter.mutate(filter.name, {
                    onSuccess: () => savedFilters.refetch(),
                  });
                }}
              >
                <Cross2Icon className="h-4 w-4" />
              </Button>
            </Badge>
          );
        })}
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
