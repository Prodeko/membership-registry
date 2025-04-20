"use client";

import {
  ColumnDef,
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
interface DataTableProps<TData, TValue> {
  modelName: string;
  columns: ColumnDef<TData, TValue>[];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  useFetchData: (x: any) => UseQueryResult<TData[], Error>;
  initialColumnVisibility?: VisibilityState;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  customFilters?: any;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  setCustomFilters?: (x: any) => void;
  multipleRowActionElements?: ActionElement<TData>[];
  getRowId?: (row: TData) => string;
  searchColumn?: string;
  filterVisible?: boolean;
}

export function DataTable<TData, TValue>({
  modelName,
  columns,
  useFetchData,
  initialColumnVisibility,
  customFilters,
  setCustomFilters,
  multipleRowActionElements,
  getRowId,
  searchColumn,
  filterVisible,
}: DataTableProps<TData, TValue>) {
  const [tableData, setTableData] = React.useState<TData[]>([]);
  const [rowSelection, setRowSelection] = React.useState({});
  const [columnVisibility, setColumnVisibility] =
    React.useState<VisibilityState>(initialColumnVisibility ?? {});
  const [sorting, setSorting] = React.useState<SortingState>([]);

  const table = useReactTable({
    data: tableData,
    columns,
    state: {
      sorting,
      columnVisibility,
      rowSelection,
    },
    enableRowSelection: true,
    manualPagination: true,
    manualFiltering: true,
    manualSorting: true,
    pageCount: 10,
    initialState: {
      pagination: {
        pageSize: 10,
        pageIndex: 0,
      },
      columnVisibility: initialColumnVisibility,
    },
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

  const { data: fetchedData } = useFetchData({
    pageSize: table.getState().pagination.pageSize,
    offset: table.getState().pagination.pageIndex,
    search: searchColumn
      ? (table.getColumn(searchColumn)?.getFilterValue() as string)
      : undefined,
    sorting: table.getState().sorting[0]?.id,
    sort_desc: table.getState().sorting[0]?.desc,
    customFilters,
  });

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

  React.useEffect(() => {
    if (fetchedData) {
      setTableData(fetchedData);
    }
  }, [fetchedData]);

  return (
    <div className="space-y-4">
      <DataTableToolbar
        table={table}
        multipleRowActionElements={multipleRowActionElements}
        searchColumn={searchColumn}
        modelName={modelName}
        customFilters={customFilters}
        setFilter={setFilter}
        filterVisible={filterVisible}
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
