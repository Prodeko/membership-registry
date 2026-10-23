import { useLogout } from "@/lib/api";
import { DropdownMenuItem } from "../ui/dropdown-menu";

/// A plain menu item, so it matches the other items in the person menu.
export const Logout = () => {
  const { mutate: logout, isPending } = useLogout();
  return (
    <DropdownMenuItem disabled={isPending} onSelect={() => logout()}>
      {isPending ? "wait..." : "Log out"}
    </DropdownMenuItem>
  );
};
