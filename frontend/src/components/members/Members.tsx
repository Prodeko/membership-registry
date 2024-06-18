import React from "react";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";
import { useGetAllMembers } from "@/lib/api";
import { DataTablePagination } from "../ui/data-table-pagination";

const Members: React.FC = () => {
  return (
    <div>
      <h1>Members</h1>
      <DataTable columns={columns} useFetchData={useGetAllMembers}/>
    </div>
  );
};

export default Members;
