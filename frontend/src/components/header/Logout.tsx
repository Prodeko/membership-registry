import { useLogout } from "@/lib/api";
import { Button } from "../ui/button";

export const Logout = () => {
  const { mutate: logout, isPending } = useLogout();
  const handleLogout = () => {
    logout();
  };
  return (
    <Button variant="ghost" onClick={handleLogout}>
      {isPending ? "wait..." : "Log out"}
    </Button>
  );
};
