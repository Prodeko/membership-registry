import {
  NavigationMenu,
  NavigationMenuList,
  NavigationMenuLink,
} from "@radix-ui/react-navigation-menu";
import { Separator } from "../ui/separator";
import { Link } from "react-router";
import { navigationMenuTriggerStyle } from "../ui/navigation-menu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";
import {
  ExternalLinkIcon,
  HamburgerMenuIcon,
  PersonIcon,
} from "@radix-ui/react-icons";
import { Logout } from "./Logout";
import { useGetPublicConfig } from "../../lib/api";

const NAV_LINKS = [
  { to: "/members", label: "Member list" },
  { to: "/applications", label: "Applications" },
  { to: "/roles", label: "Roles" },
  { to: "/role-groups", label: "Groups" },
  { to: "/attributes", label: "Attributes" },
  { to: "/email-templates", label: "Templates" },
  { to: "/marketing-tags", label: "Marketing tags" },
  { to: "/access-control", label: "Access control" },
  { to: "/data", label: "Data" },
  { to: "/logs", label: "Logs" },
];

const Header = () => {
  const { data: config } = useGetPublicConfig();
  return (
    <NavigationMenu>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 px-4 py-4 lg:px-6">
          {/* Below lg the links don't fit on one row, so they go in a menu. */}
          <DropdownMenu>
            <DropdownMenuTrigger
              className="p-2 lg:hidden"
              aria-label="Navigation"
            >
              <HamburgerMenuIcon className="h-6 w-6" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="start">
              {NAV_LINKS.map(({ to, label }) => (
                <DropdownMenuItem key={to} asChild>
                  <Link to={to}>{label}</Link>
                </DropdownMenuItem>
              ))}
              {config?.keycloak_admin_url && (
                <DropdownMenuItem asChild>
                  <a
                    href={config.keycloak_admin_url}
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    Keycloak <ExternalLinkIcon className="h-3 w-3" />
                  </a>
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>
          <Link to="/" className="shrink-0">
            <img src="/prodeko.svg" alt="Prodeko" className="h-10" />
          </Link>
          <NavigationMenuList className="hidden lg:flex flex-wrap items-center gap-x-2 pl-6">
            {NAV_LINKS.map(({ to, label }) => (
              <NavigationMenuLink
                key={to}
                asChild
                className={navigationMenuTriggerStyle()}
              >
                <Link to={to}>{label}</Link>
              </NavigationMenuLink>
            ))}
            {config?.keycloak_admin_url && (
              <NavigationMenuLink
                asChild
                className={navigationMenuTriggerStyle()}
              >
                <a
                  href={config.keycloak_admin_url}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Keycloak <ExternalLinkIcon className="ml-1 inline h-3 w-3" />
                </a>
              </NavigationMenuLink>
            )}
          </NavigationMenuList>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger className="p-4">
            <PersonIcon className="h-6 w-6" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem asChild>
              <Link to="/home">My page</Link>
            </DropdownMenuItem>
            <Logout />
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <Separator />
    </NavigationMenu>
  );
};

export default Header;
