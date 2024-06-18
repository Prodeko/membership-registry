import { useGetAllMembers } from "@/lib/api";
import React from "react";
import { DataTable } from "../ui/data-table";
import { columns } from "./columns";

const Members: React.FC = () => {
  return (
    <div>
      <h1>Members</h1>
      <DataTable columns={columns} useFetchData={useGetAllMembers} initialColumnVisibility={{"user_id": false, "has_accepted_policies": false}}/>
    </div>
  );
};

export default Members;
