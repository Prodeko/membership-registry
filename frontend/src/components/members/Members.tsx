import React from 'react';
import { DataTable } from '../ui/data-table';
import { columns } from './columns';
import { useGetAllMembers } from '@/lib/api';
import { DataTablePagination } from '../ui/data-table-pagination';

const Members: React.FC = () => {
  const { data: members, isError, isLoading, error } = useGetAllMembers();

  if (isLoading) {
    return <div>Loading...</div>;
  }

  if (isError) {
    return <div>Error {error.message}</div>;
  }

  return (
    <div>
      <h1>Members</h1>
      <DataTable columns={columns} data={members} />
    </div>
  );
};

export default Members;