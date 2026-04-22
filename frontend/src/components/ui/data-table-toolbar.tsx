"use client";

import { Cross2Icon, PlusIcon } from "@radix-ui/react-icons";
import { Table } from "@tanstack/react-table";
import { Search, SlidersHorizontal, ChevronDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import { DataTableViewOptions } from "@/components/ui/data-table-view-options";
import { Input } from "@/components/ui/input";

import { useDeleteSavedFilter, useGetSavedFilters } from "@/lib/api";
import { NewSavedFilter, SavedFilter } from "@/common/types";
import CreateSavedFilterModal from "../saved-filter/CreateSavedFilterModal";
import { ReactNode, useState } from "react";
import { cn } from "@/lib/utils";

interface DataTableToolbarProps<TData> {
  table: Table<TData>;
  searchColumn?: string;
  modelName: string;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  customFilters?: any;
  setFilter: (filter: SavedFilter) => void;
  filterVisible?: boolean;
  setFilterVisible?: (v: boolean) => void;
  filterContent?: ReactNode;
  enableSavedFilters?: boolean;
}

export function DataTableToolbar<TData>({
  table,
  searchColumn,
  modelName,
  customFilters,
  setFilter,
  filterVisible = false,
  setFilterVisible,
  filterContent,
  enableSavedFilters = false,
}: DataTableToolbarProps<TData>) {
  const [selectedFilter, setSelectedFilter] = useState<SavedFilter | null>(
    null,
  );
  const isFiltered = table.getState().columnFilters.length > 0;
  const savedFilters = useGetSavedFilters(modelName);
  const deleteSavedFilter = useDeleteSavedFilter();

  const newSavedFilter: NewSavedFilter = {
    name: "",
    filtered_model: modelName,
    visible_for_all: false,
    search: searchColumn
      ? ((table.getColumn(searchColumn)?.getFilterValue() as string) ?? null)
      : null,
    sorting_col: table.getState().sorting[0]?.id ?? null,
    sorting_desc: table.getState().sorting[0]?.desc ?? false,
    custom_filters: customFilters ?? null,
  };

  const handleFilterSelection = (filter: SavedFilter) => {
    setFilter(filter);
    setSelectedFilter(filter);
  };

  return (
    <div className="space-y-2">
      {/* Row 1: search + filter toggle — always visible */}
      <div className="flex items-center gap-2">
        {searchColumn && (
          <div className="relative w-60">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground pointer-events-none" />
            <Input
              placeholder={`Search ${modelName}…`}
              value={
                (table.getColumn(searchColumn!)?.getFilterValue() as string) ??
                ""
              }
              onChange={(e) =>
                table.getColumn(searchColumn!)?.setFilterValue(e.target.value)
              }
              className="pl-8"
            />
          </div>
        )}
        {setFilterVisible && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setFilterVisible(!filterVisible)}
          >
            <SlidersHorizontal className="h-3.5 w-3.5 mr-1.5" />
            Filters
            <ChevronDown
              className={cn(
                "h-3 w-3 ml-1 transition-transform",
                filterVisible && "rotate-180",
              )}
            />
          </Button>
        )}
        {isFiltered && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => table.resetColumnFilters()}
          >
            Reset <Cross2Icon className="ml-1 h-3.5 w-3.5" />
          </Button>
        )}
        <div className="ml-auto">
          <DataTableViewOptions table={table} />
        </div>
      </div>

      {/* Row 2: filter strip — rendered below search when open */}
      {filterVisible && filterContent}

      {/* Row 3: saved filter chips — only for pages that support saved filters */}
      {enableSavedFilters && (
        <div className="flex items-center gap-1.5 flex-wrap">
          {savedFilters.data?.map((filter) => {
            const isActive = selectedFilter?.name === filter.name;
            return (
              <span
                key={filter.name}
                onClick={() => handleFilterSelection(filter)}
                className={cn(
                  "group inline-flex items-center gap-1 px-3 py-0.5 rounded-full text-xs font-medium cursor-pointer border transition-all select-none",
                  isActive
                    ? "bg-primary text-primary-foreground border-primary"
                    : "border-border text-muted-foreground hover:border-primary/50 hover:text-foreground",
                )}
              >
                {filter.name}
                <Cross2Icon
                  className="h-2.5 w-2.5 opacity-0 group-hover:opacity-60 transition-opacity"
                  onClick={(e) => {
                    e.stopPropagation();
                    deleteSavedFilter.mutate(filter.name, {
                      onSuccess: () => savedFilters.refetch(),
                    });
                  }}
                />
              </span>
            );
          })}
          <CreateSavedFilterModal
            newSavedFilter={newSavedFilter}
            refetchSavedFilters={savedFilters.refetch}
            trigger={
              <span className="inline-flex items-center gap-1 px-3 py-0.5 rounded-full text-xs font-medium cursor-pointer border border-dashed border-border text-muted-foreground hover:text-foreground transition-colors select-none">
                <PlusIcon className="h-2.5 w-2.5" /> Save current
              </span>
            }
          />
        </div>
      )}
    </div>
  );
}
