"use client";

import {
  ColumnDef,
  PaginationState,
  RowSelectionState,
  SortingState,
  VisibilityState,
  flexRender,
  getCoreRowModel,
  getFacetedRowModel,
  getFacetedUniqueValues,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import * as React from "react";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

import type { Table as TableType } from "@tanstack/react-table";

import { UseQueryResult } from "@tanstack/react-query";
import { DataTablePagination } from "./data-table-pagination";
import { DataTableToolbar } from "./data-table-toolbar";
import { SavedFilter } from "@/common/types";

export type ActionElement<TData> = (
  table: TableType<TData>,
  selectedRows: string[]
) => React.ReactNode;

// Stable fallback so the count hook can be called unconditionally (rules of
// hooks) when no `useCount` prop is supplied.
const useNoCount = () =>
  ({ data: undefined }) as UseQueryResult<{ total: number }, Error>;
interface DataTableProps<TData, TValue> {
  modelName: string;
  columns: ColumnDef<TData, TValue>[];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  useFetchData: (x: any) => UseQueryResult<TData[], Error>;
  // Optional hook returning the total row count for the current search/filters.
  // When provided, pagination reflects the true number of pages instead of a
  // hardcoded cap.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  useCount?: (x: any) => UseQueryResult<{ total: number }, Error>;
  initialColumnVisibility?: VisibilityState;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  customFilters?: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  setCustomFilters?: (x: any) => void;
  multipleRowActionElements?: ActionElement<TData>[];
  getRowId?: (row: TData) => string;
  searchColumn?: string;
  filterVisible?: boolean;
  setFilterVisible?: (v: boolean) => void;
  filterContent?: React.ReactNode;
  rowSelection?: RowSelectionState;
  onRowSelectionChange?: (s: RowSelectionState) => void;
  enableSavedFilters?: boolean;
  selectedSavedFilter?: string | null;
  onSelectedSavedFilterChange?: (name: string | null) => void;
  // Resets the page's own filters; search and sorting are reset here.
  resetCustomFilters?: () => void;
  onRowClick?: (row: TData) => void;
}

export function DataTable<TData, TValue>({
  modelName,
  columns,
  useFetchData,
  useCount,
  initialColumnVisibility,
  customFilters,
  setCustomFilters,
  getRowId,
  searchColumn,
  filterVisible,
  setFilterVisible,
  filterContent,
  rowSelection: controlledRowSelection,
  onRowSelectionChange,
  enableSavedFilters = false,
  selectedSavedFilter,
  onSelectedSavedFilterChange,
  resetCustomFilters,
  onRowClick,
}: DataTableProps<TData, TValue>) {
  const [tableData, setTableData] = React.useState<TData[]>([]);
  const [internalRowSelection, setInternalRowSelection] = React.useState<RowSelectionState>({});
  const rowSelection = controlledRowSelection ?? internalRowSelection;
  const setRowSelection: (
    updater: RowSelectionState | ((old: RowSelectionState) => RowSelectionState),
  ) => void = (updater) => {
    const next =
      typeof updater === "function"
        ? (updater as (old: RowSelectionState) => RowSelectionState)(rowSelection)
        : updater;
    if (onRowSelectionChange) onRowSelectionChange(next);
    else setInternalRowSelection(next);
  };
  const [columnVisibility, setColumnVisibility] =
    React.useState<VisibilityState>(initialColumnVisibility ?? {});
  const [sorting, setSorting] = React.useState<SortingState>([]);
  const [rowCount, setRowCount] = React.useState<number | undefined>(undefined);
  const [pagination, setPagination] = React.useState<PaginationState>({
    pageSize: 10,
    pageIndex: 0,
  });

  // A response longer than pageSize means the endpoint ignores paging and
  // returned every row at once; page those rows here so the page size and
  // page controls still work. Server-paged endpoints never return more than
  // pageSize rows, so they are unaffected.
  const pagedLocally = tableData.length > pagination.pageSize;
  const pageStart = pagination.pageIndex * pagination.pageSize;
  const pageRows = pagedLocally
    ? tableData.slice(pageStart, pageStart + pagination.pageSize)
    : tableData;

  // Without a total, infer the page count from the page we got back. A short
  // page is the last one. Only an exactly full page leaves the count unknown
  // (-1).
  const inferredPageCount =
    tableData.length < pagination.pageSize ? pagination.pageIndex + 1 : -1;

  const table = useReactTable({
    data: pageRows,
    columns,
    state: {
      sorting,
      columnVisibility,
      rowSelection,
      pagination,
    },
    enableRowSelection: true,
    manualPagination: true,
    manualFiltering: true,
    manualSorting: true,
    // When a real total is known, let the table derive the page count from it.
    rowCount: pagedLocally ? tableData.length : rowCount,
    pageCount:
      pagedLocally || rowCount !== undefined ? undefined : inferredPageCount,
    initialState: {
      columnVisibility: initialColumnVisibility,
    },
    onPaginationChange: setPagination,
    onRowSelectionChange: setRowSelection,
    onSortingChange: setSorting,
    onColumnVisibilityChange: setColumnVisibility,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFacetedRowModel: getFacetedRowModel(),
    getFacetedUniqueValues: getFacetedUniqueValues(),
    getRowId,
  });

  const search = searchColumn
    ? (table.getColumn(searchColumn)?.getFilterValue() as string)
    : undefined;

  const { data: fetchedData, error } = useFetchData({
    pageSize: table.getState().pagination.pageSize,
    offset: table.getState().pagination.pageIndex,
    search,
    sorting: table.getState().sorting[0]?.id,
    sort_desc: table.getState().sorting[0]?.desc,
    customFilters,
  });

  const { data: countData } = (useCount ?? useNoCount)({
    search,
    customFilters,
  });

  React.useEffect(() => {
    setRowCount(countData?.total);
  }, [countData?.total]);

  const setFilter = (savedFilter: SavedFilter) => {
    if (searchColumn) {
      table.getColumn(searchColumn)?.setFilterValue(savedFilter.search ?? "");
    }
    if (savedFilter.sorting_col) {
      table.setSorting([
        {
          id: savedFilter.sorting_col,
          desc: savedFilter.sorting_desc,
        },
      ]);
    }
    if (setCustomFilters) {
      setCustomFilters(savedFilter.custom_filters);
    }
  }

  const clearFilters = () => {
    if (searchColumn) table.getColumn(searchColumn)?.setFilterValue("");
    table.setSorting([]);
    resetCustomFilters?.();
  };

  React.useEffect(() => {
    if (fetchedData) {
      setTableData(fetchedData);
    }
  }, [fetchedData]);

  if (error) {
    return (
      <div className="rounded-md border border-destructive bg-destructive/10 p-4 text-destructive">
        Failed to load data: {error.message}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <DataTableToolbar
        table={table}
        searchColumn={searchColumn}
        modelName={modelName}
        customFilters={customFilters}
        setFilter={setFilter}
        filterVisible={filterVisible}
        setFilterVisible={setFilterVisible}
        filterContent={filterContent}
        enableSavedFilters={enableSavedFilters}
        selectedSavedFilter={selectedSavedFilter}
        onSelectedSavedFilterChange={onSelectedSavedFilterChange}
        onClearFilters={clearFilters}
      />
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id}>
                {headerGroup.headers.map((header) => {
                  return (
                    <TableHead key={header.id} colSpan={header.colSpan}>
                      {header.isPlaceholder
                        ? null
                        : flexRender(
                            header.column.columnDef.header,
                            header.getContext()
                          )}
                    </TableHead>
                  );
                })}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {table.getRowModel().rows?.length ? (
              table.getRowModel().rows.map((row) => (
                <TableRow
                  key={row.id}
                  data-state={row.getIsSelected() && "selected"}
                  onClick={() => onRowClick?.(row.original)}
                  className={onRowClick ? "cursor-pointer" : ""}
                >
                  {row.getVisibleCells().map((cell) => (
                    <TableCell key={cell.id}>
                      {flexRender(
                        cell.column.columnDef.cell,
                        cell.getContext()
                      )}
                    </TableCell>
                  ))}
                </TableRow>
              ))
            ) : (
              <TableRow>
                <TableCell
                  colSpan={columns.length}
                  className="h-24 text-center"
                >
                  No results.
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>
      <DataTablePagination table={table} />
    </div>
  );
}
