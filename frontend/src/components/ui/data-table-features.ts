import {
  columnFacetingFeature,
  columnFilteringFeature,
  columnVisibilityFeature,
  createFacetedRowModel,
  createFacetedUniqueValues,
  createFilteredRowModel,
  createSortedRowModel,
  rowPaginationFeature,
  rowSelectionFeature,
  rowSortingFeature,
  tableFeatures,
} from "@tanstack/react-table";
import type {
  Column,
  ColumnDef,
  ReactTable,
  RowData,
} from "@tanstack/react-table";

/// The table features every DataTable uses. Table v9 only exposes the APIs of
/// features registered here, so a column or toolbar that calls a new feature
/// API needs it added first.
export const dataTableFeatures = tableFeatures({
  columnFilteringFeature,
  columnFacetingFeature,
  columnVisibilityFeature,
  rowPaginationFeature,
  rowSelectionFeature,
  rowSortingFeature,
  filteredRowModel: createFilteredRowModel(),
  sortedRowModel: createSortedRowModel(),
  facetedRowModel: createFacetedRowModel(),
  facetedUniqueValues: createFacetedUniqueValues(),
});

export type DataTableFeatures = typeof dataTableFeatures;

export type DataTableColumnDef<TData extends RowData, TValue = unknown> =
  ColumnDef<DataTableFeatures, TData, TValue>;
export type DataTableColumn<TData extends RowData, TValue = unknown> = Column<
  DataTableFeatures,
  TData,
  TValue
>;
/// The table as `useTable` returns it, including its reactive `state`.
export type DataTableInstance<TData extends RowData> = ReactTable<
  DataTableFeatures,
  TData
>;
