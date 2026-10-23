import { PersonIcon } from "@radix-ui/react-icons";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { useGetAccessPass, useLogout } from "@/lib/api";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";

/// The person menu on the member home page. Admins get a link back to the
/// admin view, mirroring the "My page" link in the admin header.
export default function UserMenu() {
  const { t } = useTranslation();
  const { data: pass } = useGetAccessPass();
  const { mutate: logout, isPending } = useLogout();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger className="p-2" aria-label={t("home.menu.label")}>
        <PersonIcon className="h-6 w-6" />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {pass?.is_admin && (
          <DropdownMenuItem asChild>
            <Link to="/">{t("home.menu.admin_view")}</Link>
          </DropdownMenuItem>
        )}
        <DropdownMenuItem disabled={isPending} onSelect={() => logout()}>
          {t("home.menu.logout")}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
