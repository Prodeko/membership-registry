"use client";

import {
  ColumnDef,
  ColumnFiltersState,
  Row,
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

import { UseQueryResult } from "@tanstack/react-query";
import { DataTablePagination } from "./data-table-pagination";
import { DataTableToolbar } from "./data-table-toolbar";
import { Button } from "./button";


export interface Action<TData> {
  label: string;
  onClick: (rows: TData[]) => void;
}
interface DataTableProps<TData, TValue> {
  columns: ColumnDef<TData, TValue>[];
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  useFetchData: (x: any) => UseQueryResult<TData[], Error>;
  initialColumnVisibility?: VisibilityState;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  customFilters?: any;
  multipleRowActions?: Action<TData>[];
}

export function DataTable<TData, TValue>({
  columns,
  useFetchData,
  initialColumnVisibility,
  customFilters,
  multipleRowActions,
}: DataTableProps<TData, TValue>) {
  const [tableData, setTableData] = React.useState<TData[]>([]);
  const [rowSelection, setRowSelection] = React.useState({});
  const [columnVisibility, setColumnVisibility] =
    React.useState<VisibilityState>(initialColumnVisibility ?? {});
  const [columnFilters, setColumnFilters] = React.useState<ColumnFiltersState>(
    []
  );
  const [sorting, setSorting] = React.useState<SortingState>([]);

  const table = useReactTable({
    data: tableData,
    columns,
    state: {
      sorting,
      columnVisibility,
      rowSelection,
      columnFilters,
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
    onColumnFiltersChange: setColumnFilters,
    onColumnVisibilityChange: setColumnVisibility,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFacetedRowModel: getFacetedRowModel(),
    getFacetedUniqueValues: getFacetedUniqueValues(),
  });

  const {
    data: fetchedData,
    isLoading,
    error,
  } = useFetchData({
    pageSize: table.getState().pagination.pageSize,
    offset: table.getState().pagination.pageIndex,
    search: table.getColumn("first_name")?.getFilterValue() as string,
    sorting: table.getState().sorting[0]?.id,
    sort_desc: table.getState().sorting[0]?.desc,
    customFilters,
  });

  console.log(table.getState().sorting);
  React.useEffect(() => {
    if (fetchedData) {
      setTableData(fetchedData);
    }
  }, [fetchedData]);

  const handleMultipleRowAction = (action: Action<TData>) => {
    const rowSelection = table.getState().rowSelection;
    const rows = table
      .getRowModel()
      .rows.filter((row) => rowSelection[row.id])
      .map((row) => row.original)
    action.onClick(rows);
    setRowSelection({});
  }

  return (
    <div className="space-y-4 m-8">
      <DataTableToolbar table={table} />
      {multipleRowActions &&
        multipleRowActions.map((action) => (
          <Button
            key={action.label}
            disabled={!Object.values(table.getState().rowSelection).some(Boolean)}
            variant={"outline"}
            onClick={() => handleMultipleRowAction(action)}
          >
            {action.label}
          </Button>
        ))}
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
