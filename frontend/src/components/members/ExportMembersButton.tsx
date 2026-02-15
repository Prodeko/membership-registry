import { MemberWithRoles } from "@/common/types";
import { useExportMembersWithRoles } from "@/lib/api";
import { Table } from "@tanstack/react-table";
import { Button } from "../ui/button";

interface Props {
  table: Table<MemberWithRoles>;
  customFilters: {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;
  };
}

const ExportMembersButton = ({ table, customFilters }: Props) => {
  const { mutate: exportMembersMutation } = useExportMembersWithRoles();
  return (
    <Button
      variant="outline"
      onClick={() => {
        exportMembersMutation({
          pageSize: 0,
          offset: 0,
          search: table.getColumn("first_name")?.getFilterValue() as string,
          sorting: table.getState().sorting[0]?.id,
          sort_desc: table.getState().sorting[0]?.desc,
          customFilters,
        });
      }}
    >
      Export
    </Button>
  );
};

export default ExportMembersButton;
